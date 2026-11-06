use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::{error::StorageError, infrastructure::storage};

pub(crate) struct Foundation {
    database: Result<SqlitePool, StorageError>,
}

impl Foundation {
    pub(crate) async fn initialize(directory: Option<PathBuf>) -> Self {
        let database = match directory {
            Some(directory) => storage::open(&directory).await,
            None => Err(StorageError::Unavailable),
        };

        if let Err(error) = &database {
            tracing::warn!(?error, "local storage startup failed");
        }

        Self { database }
    }

    pub(crate) async fn check_database(&self) -> Result<(), StorageError> {
        let database = self.database.as_ref().map_err(|error| *error)?;
        storage::check(database).await
    }
}
