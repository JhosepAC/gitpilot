use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use sqlx::SqlitePool;

use super::{
    config_backup,
    config_preview::{self, Candidate},
    git,
    native_file::{self, GitLock, Snapshot, Stage, Stamp},
};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ApplyError {
    StalePreview,
    Busy,
    UnsafeTarget,
    BackupUnavailable,
    VerificationFailed,
    RecoveryRequired,
    StorageUnavailable,
}

impl From<sqlx::Error> for ApplyError {
    fn from(_: sqlx::Error) -> Self {
        Self::StorageUnavailable
    }
}

#[derive(sqlx::FromRow)]
struct Operation {
    token: String,
    target: String,
    parent_id: String,
    before_stamp: Option<String>,
    backup: Option<Vec<u8>>,
    candidate_hash: String,
    candidate_id: Option<String>,
    applied_stamp: Option<String>,
    recovery_id: Option<String>,
    state: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OperationStatus {
    state: String,
    target: String,
    confirmation: Option<String>,
}

async fn live(pool: &SqlitePool) -> Result<Option<Operation>, ApplyError> {
    Ok(sqlx::query_as(
        "SELECT token, target, parent_id, before_stamp, backup, candidate_hash,
                candidate_id, applied_stamp, recovery_id, state FROM config_operations
         WHERE state IN ('preparing', 'applied', 'restoring')",
    )
    .fetch_optional(pool)
    .await?)
}

fn encoded<T: Serialize>(value: &T) -> Result<String, ApplyError> {
    serde_json::to_string(value).map_err(|_| ApplyError::StorageUnavailable)
}

fn snapshot(target: &Path, parent: [u64; 2]) -> Result<Option<Snapshot>, ApplyError> {
    if native_file::parent_id(target).map_err(|_| ApplyError::UnsafeTarget)? != parent {
        return Err(ApplyError::UnsafeTarget);
    }
    native_file::snapshot(target).map_err(|_| ApplyError::UnsafeTarget)
}

fn random_token() -> Result<String, ApplyError> {
    let mut bytes = [0; 24];
    getrandom::fill(&mut bytes).map_err(|_| ApplyError::BackupUnavailable)?;
    Ok(hex::encode(bytes))
}

fn lock(app: &Path) -> Result<fs::File, ApplyError> {
    native_file::app_lock(&app.join("native-config.lock")).map_err(|_| ApplyError::Busy)
}

fn stage_path(target: &Path, token: &str, recovery: bool) -> Result<PathBuf, ApplyError> {
    if token.len() != 48 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ApplyError::RecoveryRequired);
    }
    let suffix = if recovery { "restore" } else { "apply" };
    Ok(target.with_file_name(format!(".gitpilot-{token}-{suffix}.stage")))
}

impl Operation {
    fn parent(&self) -> Result<[u64; 2], ApplyError> {
        serde_json::from_str(&self.parent_id).map_err(|_| ApplyError::RecoveryRequired)
    }

    fn before(&self) -> Result<Option<Stamp>, ApplyError> {
        self.before_stamp
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(|_| ApplyError::RecoveryRequired)
    }

    fn applied(&self, actual: Option<&Snapshot>) -> bool {
        actual.is_some_and(|file| {
            if let Some(stamp) = &self.applied_stamp {
                return encoded(&file.stamp).ok().as_ref() == Some(stamp);
            }
            self.candidate_id.as_deref() == encoded(&file.stamp.id).ok().as_deref()
                && hex::encode(file.stamp.digest) == self.candidate_hash
        })
    }

    fn previous(&self, actual: Option<&Snapshot>, before: Option<&Stamp>) -> bool {
        match (actual, before) {
            (None, None) => true,
            (Some(file), Some(before)) => {
                file.stamp == *before
                    || (file.stamp.digest == before.digest
                        && file.stamp.same_permissions(before)
                        && self.recovery_id.as_deref() == encoded(&file.stamp.id).ok().as_deref())
            }
            _ => false,
        }
    }

    fn restore_confirmation(&self, current: &Stamp) -> Result<String, ApplyError> {
        Ok(hex::encode(native_file::digest(
            encoded(&(&self.token, current))?.as_bytes(),
        )))
    }
}

async fn set_state(pool: &SqlitePool, token: &str, state: &str) -> Result<(), ApplyError> {
    let result = sqlx::query("UPDATE config_operations SET state = ? WHERE token = ?")
        .bind(state)
        .bind(token)
        .execute(pool)
        .await?;
    if result.rows_affected() != 1 {
        return Err(ApplyError::StorageUnavailable);
    }
    Ok(())
}

fn clean_stage(operation: &Operation, target: &Path, recovery: bool) -> Result<(), ApplyError> {
    let path = stage_path(target, &operation.token, recovery)?;
    if let Some(stage) = native_file::snapshot(&path).map_err(|_| ApplyError::RecoveryRequired)? {
        let expected = if recovery {
            &operation.recovery_id
        } else {
            &operation.candidate_id
        };
        if expected.as_deref() != Some(encoded(&stage.stamp.id)?.as_str()) {
            return Err(ApplyError::RecoveryRequired);
        }
        fs::remove_file(&path).map_err(|_| ApplyError::RecoveryRequired)?;
        native_file::sync_parent(&path).map_err(|_| ApplyError::RecoveryRequired)?;
    }
    Ok(())
}

async fn verify_global(target: &Path) -> Result<(), ApplyError> {
    if !target.exists() {
        return Ok(());
    }
    let exe = git::executable().ok_or(ApplyError::VerificationFailed)?;
    let cwd = target.parent().ok_or(ApplyError::UnsafeTarget)?;
    let mut command = git::command(&exe, cwd);
    // The output may contain user-defined keys. It stays inside the Rust process.
    let (status, _) = git::output(
        command.arg("config").arg("--file").arg(target).args([
            "--includes",
            "--name-only",
            "--list",
        ]),
        std::time::Duration::from_secs(3),
    )
    .await
    .map_err(|_| ApplyError::VerificationFailed)?;
    if !status.success() {
        return Err(ApplyError::VerificationFailed);
    }
    Ok(())
}

async fn recover(
    pool: &SqlitePool,
    operation: &Operation,
    take_over_lock: bool,
) -> Result<(), ApplyError> {
    let target = Path::new(&operation.target);
    let parent = operation.parent()?;
    let before = operation.before()?;
    let current = snapshot(target, parent)?;
    let terminal = if operation.state == "restoring" {
        "restored"
    } else {
        "rolled_back"
    };
    if take_over_lock {
        GitLock::clear_owned(target, &operation.token).map_err(|_| ApplyError::RecoveryRequired)?;
    }
    let _git = if take_over_lock {
        Some(GitLock::acquire(target, &operation.token).map_err(|_| ApplyError::Busy)?)
    } else {
        None
    };
    let checked = snapshot(target, parent)?;
    if current.as_ref().map(|f| &f.stamp) != checked.as_ref().map(|f| &f.stamp) {
        return Err(ApplyError::RecoveryRequired);
    }
    if operation.state == "preparing" && operation.candidate_id.is_none() {
        clean_stage(operation, target, false)?;
        return set_state(pool, &operation.token, terminal).await;
    }
    if operation.previous(checked.as_ref(), before.as_ref()) {
        clean_stage(operation, target, false)?;
        clean_stage(operation, target, true)?;
        if operation.recovery_id.is_some() {
            verify_global(target).await?;
        }
        return set_state(pool, &operation.token, terminal).await;
    }
    if !operation.applied(checked.as_ref()) {
        return Err(ApplyError::RecoveryRequired);
    }
    clean_stage(operation, target, true)?;
    if let (Some(before), Some(backup)) = (&before, &operation.backup) {
        let original = config_backup::open(&operation.token, backup)
            .await
            .map_err(|_| ApplyError::BackupUnavailable)?;
        if native_file::digest(&original) != before.digest {
            return Err(ApplyError::RecoveryRequired);
        }
        let mut stage = Stage::create(stage_path(target, &operation.token, true)?, Some(before))
            .map_err(|_| ApplyError::RecoveryRequired)?;
        sqlx::query("UPDATE config_operations SET recovery_id = ? WHERE token = ?")
            .bind(encoded(&stage.id())?)
            .bind(&operation.token)
            .execute(pool)
            .await?;
        stage
            .write(&original, Some(before))
            .map_err(|_| ApplyError::RecoveryRequired)?;
        if !operation.applied(snapshot(target, parent)?.as_ref()) {
            return Err(ApplyError::RecoveryRequired);
        }
        native_file::replace(&stage.path, target, true)
            .map_err(|_| ApplyError::RecoveryRequired)?;
        let restored = snapshot(target, parent)?;
        if !restored.is_some_and(|file| {
            file.stamp.id == stage.id()
                && file.stamp.digest == before.digest
                && file.stamp.same_permissions(before)
        }) {
            return Err(ApplyError::RecoveryRequired);
        }
    } else {
        if !operation.applied(snapshot(target, parent)?.as_ref()) {
            return Err(ApplyError::RecoveryRequired);
        }
        fs::remove_file(target).map_err(|_| ApplyError::RecoveryRequired)?;
        native_file::sync_parent(target).map_err(|_| ApplyError::RecoveryRequired)?;
    }
    clean_stage(operation, target, false)?;
    verify_global(target).await?;
    set_state(pool, &operation.token, terminal).await
}

pub(crate) async fn reconcile(pool: &SqlitePool, app: &Path) -> Result<(), ApplyError> {
    let _application = lock(app)?;
    if let Some(operation) = live(pool).await? {
        if operation.state != "applied" {
            recover(pool, &operation, true).await?;
        }
    }
    Ok(())
}

pub(crate) async fn status(pool: &SqlitePool) -> Result<Option<OperationStatus>, ApplyError> {
    let Some(operation) = live(pool).await? else {
        return Ok(None);
    };
    let current = snapshot(Path::new(&operation.target), operation.parent()?)
        .ok()
        .flatten();
    let confirmation = if operation.state == "applied" && operation.applied(current.as_ref()) {
        current
            .as_ref()
            .map(|file| operation.restore_confirmation(&file.stamp))
            .transpose()?
    } else {
        None
    };
    Ok(Some(OperationStatus {
        state: if operation.state == "applied" && confirmation.is_some() {
            "applied".into()
        } else {
            "recovery_required".into()
        },
        target: operation.target,
        confirmation,
    }))
}

fn candidate_bytes(
    candidate: &Candidate,
    before: Option<&Snapshot>,
) -> Result<Vec<u8>, ApplyError> {
    let mut bytes = before.map_or_else(Vec::new, |file| file.bytes.clone());
    if bytes.contains(&0)
        || bytes
            .iter()
            .rev()
            .find(|byte| !matches!(byte, b'\r' | b'\n'))
            == Some(&b'\\')
    {
        return Err(ApplyError::UnsafeTarget);
    }
    if !bytes.is_empty() && !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    bytes.extend_from_slice(candidate.append.as_bytes());
    if bytes.len() > 1024 * 1024 {
        return Err(ApplyError::UnsafeTarget);
    }
    Ok(bytes)
}

pub(crate) async fn apply(
    pool: &SqlitePool,
    app: &Path,
    confirmation: &str,
) -> Result<(), ApplyError> {
    let _application = lock(app)?;
    if live(pool).await?.is_some() {
        return Err(ApplyError::RecoveryRequired);
    }
    let preview = config_preview::prepare(pool, app)
        .await
        .map_err(|_| ApplyError::StalePreview)?;
    if preview.confirmation() != Some(confirmation) {
        return Err(ApplyError::StalePreview);
    }
    let candidate = preview.candidate.as_ref().ok_or(ApplyError::StalePreview)?;
    let target = &candidate.target;
    let before = snapshot(target, candidate.parent)?;
    if before.as_ref().map(|file| &file.stamp) != candidate.before.as_ref() {
        return Err(ApplyError::StalePreview);
    }
    let bytes = candidate_bytes(candidate, before.as_ref())?;
    let token = random_token()?;
    let backup = if let Some(before) = &before {
        let encrypted = config_backup::seal(&token, &before.bytes)
            .await
            .map_err(|_| ApplyError::BackupUnavailable)?;
        let verified = config_backup::open(&token, &encrypted)
            .await
            .map_err(|_| ApplyError::BackupUnavailable)?;
        if verified != before.bytes {
            return Err(ApplyError::BackupUnavailable);
        }
        Some(encrypted)
    } else {
        None
    };
    sqlx::query(
        "INSERT INTO config_operations (token, target, parent_id, before_stamp, backup, candidate_hash, state)
         VALUES (?, ?, ?, ?, ?, ?, 'preparing')",
    )
    .bind(&token).bind(target.to_str().ok_or(ApplyError::UnsafeTarget)?)
    .bind(encoded(&candidate.parent)?)
    .bind(candidate.before.as_ref().map(encoded).transpose()?)
    .bind(backup)
    .bind(hex::encode(native_file::digest(&bytes))).execute(pool).await?;
    let git_lock = match GitLock::acquire(target, &token) {
        Ok(guard) => guard,
        Err(_) => {
            set_state(pool, &token, "rolled_back").await?;
            return Err(ApplyError::Busy);
        }
    };
    let result = async {
        let checked = config_preview::prepare(pool, app)
            .await
            .map_err(|_| ApplyError::StalePreview)?;
        if checked.confirmation() != Some(confirmation) {
            return Err(ApplyError::StalePreview);
        }
        let root = candidate
            .generation
            .parent()
            .ok_or(ApplyError::UnsafeTarget)?;
        native_file::private_directory(root).map_err(|_| ApplyError::UnsafeTarget)?;
        native_file::private_directory(&candidate.generation)
            .map_err(|_| ApplyError::UnsafeTarget)?;
        for (name, bytes) in &candidate.files {
            native_file::create_immutable(&candidate.generation.join(name), bytes)
                .map_err(|_| ApplyError::UnsafeTarget)?;
        }
        let mut stage = Stage::create(
            stage_path(target, &token, false)?,
            candidate.before.as_ref(),
        )
        .map_err(|_| ApplyError::UnsafeTarget)?;
        sqlx::query("UPDATE config_operations SET candidate_id = ? WHERE token = ?")
            .bind(encoded(&stage.id())?)
            .bind(&token)
            .execute(pool)
            .await?;
        stage
            .write(&bytes, candidate.before.as_ref())
            .map_err(|_| ApplyError::UnsafeTarget)?;
        if snapshot(target, candidate.parent)?
            .as_ref()
            .map(|f| &f.stamp)
            != candidate.before.as_ref()
        {
            return Err(ApplyError::StalePreview);
        }
        native_file::replace(&stage.path, target, candidate.before.is_some())
            .map_err(|_| ApplyError::UnsafeTarget)?;
        let applied = snapshot(target, candidate.parent)?
            .filter(|file| {
                file.stamp.id == stage.id() && file.stamp.digest == native_file::digest(&bytes)
            })
            .ok_or(ApplyError::VerificationFailed)?;
        if candidate
            .before
            .as_ref()
            .is_some_and(|before| !applied.stamp.same_permissions(before))
        {
            return Err(ApplyError::VerificationFailed);
        }
        preview
            .verify()
            .await
            .map_err(|_| ApplyError::VerificationFailed)?;
        if !snapshot(target, candidate.parent)?.is_some_and(|file| file.stamp == applied.stamp) {
            return Err(ApplyError::VerificationFailed);
        }
        sqlx::query(
            "UPDATE config_operations SET state = 'applied', applied_stamp = ? WHERE token = ?",
        )
        .bind(encoded(&applied.stamp)?)
        .bind(&token)
        .execute(pool)
        .await?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let operation = live(pool).await?.ok_or(ApplyError::RecoveryRequired)?;
        if recover(pool, &operation, false).await.is_err() {
            return Err(ApplyError::RecoveryRequired);
        }
    }
    drop(git_lock);
    result
}

pub(crate) async fn restore(
    pool: &SqlitePool,
    app: &Path,
    confirmation: &str,
) -> Result<(), ApplyError> {
    let _application = lock(app)?;
    let operation = live(pool).await?.ok_or(ApplyError::StalePreview)?;
    let target = Path::new(&operation.target);
    let current = snapshot(target, operation.parent()?)?;
    let Some(current) = current.filter(|file| operation.applied(Some(file))) else {
        return Err(ApplyError::RecoveryRequired);
    };
    if operation.state != "applied"
        || operation.restore_confirmation(&current.stamp)? != confirmation
    {
        return Err(ApplyError::StalePreview);
    }
    GitLock::clear_owned(target, &operation.token).map_err(|_| ApplyError::Busy)?;
    let _git = GitLock::acquire(target, &operation.token).map_err(|_| ApplyError::Busy)?;
    if snapshot(target, operation.parent()?)?
        .as_ref()
        .map(|f| &f.stamp)
        != Some(&current.stamp)
    {
        return Err(ApplyError::StalePreview);
    }
    set_state(pool, &operation.token, "restoring").await?;
    let operation = live(pool).await?.ok_or(ApplyError::RecoveryRequired)?;
    recover(pool, &operation, false)
        .await
        .map_err(|_| ApplyError::RecoveryRequired)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::{discovery, storage};

    #[test]
    fn recovery_uses_file_identity_and_preserves_external_changes() {
        tauri::async_runtime::block_on(async {
            for edited in [false, true] {
                let temp = tempfile::tempdir().unwrap();
                let app = PathBuf::from(discovery::directory_path(temp.path()).await.unwrap());
                let pool = storage::open(&app).await.unwrap();
                let target = app.join(".gitconfig");
                let token = random_token().unwrap();
                let parent = native_file::parent_id(&target).unwrap();
                let bytes = b"[user]\nname = Fixture\n";
                let mut stage =
                    Stage::create(stage_path(&target, &token, false).unwrap(), None).unwrap();
                stage.write(bytes, None).unwrap();
                sqlx::query("INSERT INTO config_operations (token, target, parent_id, candidate_hash, candidate_id, state)
                    VALUES (?, ?, ?, ?, ?, 'preparing')")
                    .bind(&token)
                    .bind(target.to_str().unwrap())
                    .bind(encoded(&parent).unwrap())
                    .bind(hex::encode(native_file::digest(bytes)))
                    .bind(encoded(&stage.id()).unwrap())
                    .execute(&pool).await.unwrap();
                let lock = GitLock::acquire(&target, &token).unwrap();
                native_file::replace(&stage.path, &target, false).unwrap();
                std::mem::forget(lock); // The interrupted process leaves its Git lock on disk.
                if edited {
                    fs::write(&target, b"[user]\nname = External\n").unwrap();
                    assert_eq!(
                        reconcile(&pool, &app).await,
                        Err(ApplyError::RecoveryRequired)
                    );
                    assert_eq!(fs::read(&target).unwrap(), b"[user]\nname = External\n");
                    assert_eq!(live(&pool).await.unwrap().unwrap().state, "preparing");
                } else {
                    reconcile(&pool, &app).await.unwrap();
                    assert!(!target.exists());
                    assert!(live(&pool).await.unwrap().is_none());
                }
            }
        });
    }

    #[test]
    fn respects_a_git_lock_and_protects_the_backup_on_windows() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join(".gitconfig");
            let lock = temp.path().join(".gitconfig.lock");
            fs::write(&lock, b"another writer").unwrap();
            assert!(GitLock::acquire(&path, "fixture").is_err());
            assert!(GitLock::clear_owned(&path, "fixture").is_err());
            assert_eq!(fs::read(lock).unwrap(), b"another writer");
            #[cfg(windows)]
            {
                let plain = b"[credential]\nhelper = sensitive-fixture\n";
                let encrypted = config_backup::seal("fixture", plain).await.unwrap();
                assert!(!encrypted.windows(plain.len()).any(|bytes| bytes == plain));
                assert_eq!(
                    config_backup::open("fixture", &encrypted).await.unwrap(),
                    plain
                );
                assert!(config_backup::open("different", &encrypted).await.is_err());
            }
        });
    }
}
