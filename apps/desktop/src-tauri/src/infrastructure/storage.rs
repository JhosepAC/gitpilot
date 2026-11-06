use std::{path::Path, time::Duration};

use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

use crate::error::StorageError;

static MIGRATOR: Migrator = sqlx::migrate!();

pub(crate) async fn open(directory: &Path) -> Result<SqlitePool, StorageError> {
    std::fs::create_dir_all(directory).map_err(|_| StorageError::Unavailable)?;

    let options = SqliteConnectOptions::new()
        .filename(directory.join("gitpilot.sqlite3"))
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(3));
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|_| StorageError::InitializationFailed)?;
    MIGRATOR
        .run(&pool)
        .await
        .map_err(|_| StorageError::InitializationFailed)?;

    Ok(pool)
}

pub(crate) async fn check(database: &SqlitePool) -> Result<(), StorageError> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(database)
        .await
        .map_err(|_| StorageError::CheckFailed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_migrates_and_reopens_database() {
        tauri::async_runtime::block_on(async {
            let directory = tempfile::tempdir().unwrap();
            let pool = open(directory.path()).await.unwrap();
            check(&pool).await.unwrap();
            let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(foreign_keys, 1);
            sqlx::query("INSERT INTO app_settings (key, value) VALUES ('fixture', 'persisted')")
                .execute(&pool)
                .await
                .unwrap();
            pool.close().await;

            let reopened = open(directory.path()).await.unwrap();
            let value: String =
                sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'fixture'")
                    .fetch_one(&reopened)
                    .await
                    .unwrap();
            assert_eq!(value, "persisted");
            reopened.close().await;
        });
    }

    #[test]
    fn returns_safe_error_for_unusable_directory() {
        tauri::async_runtime::block_on(async {
            let file = tempfile::NamedTempFile::new().unwrap();
            assert!(matches!(
                open(file.path()).await,
                Err(StorageError::Unavailable)
            ));
        });
    }
}
