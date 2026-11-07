use std::sync::{Mutex, MutexGuard, PoisonError};

use sqlx::SqlitePool;
use tokio::sync::watch;

use crate::repository::{InventoryError, ScanPhase, ScanProgress, ScanReport};

use super::{discovery, inventory, scan_preferences};

#[derive(Default)]
pub(crate) struct ScanJobs(Mutex<Option<InventoryJob>>);

enum InventoryJob {
    Exclusive,
    Scan(ActiveScan),
}

struct ActiveScan {
    id: String,
    cancel: watch::Sender<bool>,
    saving: bool,
}

pub(crate) struct InventoryGuard<'a>(&'a ScanJobs);

impl ScanJobs {
    fn active(&self) -> MutexGuard<'_, Option<InventoryJob>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn start(
        &self,
        id: String,
    ) -> Result<(InventoryGuard<'_>, watch::Receiver<bool>), InventoryError> {
        if id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(InventoryError::ScanRequestInvalid);
        }
        let (cancel, receiver) = watch::channel(false);
        let guard = self.reserve(InventoryJob::Scan(ActiveScan {
            id,
            cancel,
            saving: false,
        }))?;
        Ok((guard, receiver))
    }

    pub(crate) fn try_lock(&self) -> Result<InventoryGuard<'_>, InventoryError> {
        self.reserve(InventoryJob::Exclusive)
    }

    fn reserve(&self, job: InventoryJob) -> Result<InventoryGuard<'_>, InventoryError> {
        let mut current = self.active();
        if current.is_some() {
            return Err(InventoryError::ScanInProgress);
        }
        *current = Some(job);
        Ok(InventoryGuard(self))
    }

    pub(crate) fn cancel(&self, id: &str) -> bool {
        let current = self.active();
        let Some(InventoryJob::Scan(scan)) = current.as_ref() else {
            return false;
        };
        if scan.id != id || scan.saving {
            return false;
        }
        scan.cancel.send_replace(true);
        true
    }

    pub(crate) async fn run(
        &self,
        pool: &SqlitePool,
        directory_id: i64,
        id: String,
        mut notify: impl FnMut(ScanProgress),
    ) -> Result<ScanReport, InventoryError> {
        let (job, mut cancel) = self.start(id)?;
        let directory = inventory::directory(pool, directory_id).await?;
        let exclusions = inventory::contents(pool, directory_id).await?.exclusions;
        let preferences = scan_preferences::read(pool).await?.values;
        notify(ScanProgress::new(
            ScanPhase::Walking,
            &ScanReport::default(),
        ));
        let (repositories, mut report) = discovery::scan(
            &directory.path,
            &exclusions,
            preferences,
            &mut cancel,
            &mut notify,
        )
        .await?;
        if job.begin_saving() {
            report.cancel();
        }
        notify(ScanProgress::new(ScanPhase::Saving, &report));
        inventory::save_scan(pool, &directory, repositories, report.partial).await?;
        Ok(report)
    }
}

impl InventoryGuard<'_> {
    fn begin_saving(&self) -> bool {
        let mut current = self.0.active();
        let Some(InventoryJob::Scan(scan)) = current.as_mut() else {
            unreachable!("scan registered while its guard is alive");
        };
        scan.saving = true;
        let cancelled = *scan.cancel.borrow();
        cancelled
    }
}

impl Drop for InventoryGuard<'_> {
    fn drop(&mut self) {
        *self.0.active() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{infrastructure::storage, repository::DiscoveredRepository};

    #[test]
    fn protects_an_active_scan_and_ignores_late_cancellation() {
        let jobs = ScanJobs::default();
        let exclusive = jobs.try_lock().unwrap();
        assert!(matches!(
            jobs.start("test-one".into()),
            Err(InventoryError::ScanInProgress)
        ));
        assert!(!jobs.cancel("test-one"));
        drop(exclusive);
        let (job, cancel) = jobs.start("test-one".into()).unwrap();
        assert!(matches!(
            jobs.try_lock(),
            Err(InventoryError::ScanInProgress)
        ));
        assert!(!jobs.cancel("another-scan"));
        assert!(!*cancel.borrow());
        assert!(!job.begin_saving());
        assert!(!jobs.cancel("test-one"));
        drop(job);
        assert!(!jobs.cancel("test-one"));
        assert!(jobs.start("test-two".into()).is_ok());
    }

    #[test]
    fn cancelled_scan_preserves_existing_inventory_and_releases_the_job() {
        tauri::async_runtime::block_on(async {
            let root = tempfile::tempdir().unwrap();
            let app = tempfile::tempdir().unwrap();
            let root_path = discovery::directory_path(root.path()).await.unwrap();
            let pool = storage::open(app.path()).await.unwrap();
            let id = inventory::add(&pool, &root_path).await.unwrap();
            let snapshot = inventory::directory(&pool, id).await.unwrap();
            inventory::save_scan(
                &pool,
                &snapshot,
                vec![DiscoveredRepository {
                    path: root_path.clone(),
                    git_dir: root_path.clone(),
                    common_dir: root_path.clone(),
                    kind: "bare",
                }],
                false,
            )
            .await
            .unwrap();
            let jobs = ScanJobs::default();
            let mut phases = Vec::new();
            let result = jobs
                .run(&pool, id, "test-cancel".into(), |progress| {
                    match progress.phase {
                        ScanPhase::Walking => assert!(jobs.cancel("test-cancel")),
                        ScanPhase::Saving => assert!(!jobs.cancel("test-cancel")),
                    }
                    phases.push(progress.phase);
                })
                .await
                .unwrap();
            assert!(result.cancelled && result.partial);
            assert_eq!(result.found, 0);
            assert!(matches!(
                phases.as_slice(),
                [ScanPhase::Walking, ScanPhase::Saving]
            ));
            assert!(inventory::contents(&pool, id).await.unwrap().repositories[0].present);
            assert!(jobs.try_lock().is_ok());
        });
    }
}
