use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::{
    error::StorageError,
    infrastructure::{inventory_scan::ScanJobs, storage},
};

pub(crate) struct Foundation {
    database: Result<SqlitePool, StorageError>,
    pub(crate) directory: Option<PathBuf>,
    pub(crate) inventory_scan: ScanJobs,
    pub(crate) github: crate::infrastructure::github::Github,
}

impl Foundation {
    pub(crate) async fn initialize(directory: Option<PathBuf>) -> Self {
        let database = match directory.as_deref() {
            Some(directory) => storage::open(directory).await,
            None => Err(StorageError::Unavailable),
        };

        if let Err(error) = &database {
            tracing::warn!(?error, "local storage startup failed");
        }

        Self {
            database,
            directory,
            inventory_scan: ScanJobs::default(),
            github: crate::infrastructure::github::Github::new(),
        }
    }

    pub(crate) fn database(&self) -> Result<&SqlitePool, StorageError> {
        self.database.as_ref().map_err(|error| *error)
    }

    pub(crate) async fn check_database(&self) -> Result<(), StorageError> {
        storage::check(self.database()?).await
    }
}
