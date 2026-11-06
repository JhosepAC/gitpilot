use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::{error::StorageError, infrastructure::storage};

pub(crate) struct Foundation {
    database: Result<SqlitePool, StorageError>,
    pub(crate) directory: Option<PathBuf>,
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
        }
    }

    pub(crate) fn database(&self) -> Result<&SqlitePool, StorageError> {
        self.database.as_ref().map_err(|error| *error)
    }

    pub(crate) async fn check_database(&self) -> Result<(), StorageError> {
        storage::check(self.database()?).await
    }
}
