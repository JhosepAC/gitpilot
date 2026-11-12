use std::{path::Path, time::Duration};

use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};

use crate::error::StorageError;

static MIGRATOR: Migrator = sqlx::migrate!();

pub(crate) async fn open(directory: &Path) -> Result<SqlitePool, StorageError> {
    std::fs::create_dir_all(directory).map_err(|_| StorageError::Unavailable)?;

    let options = SqliteConnectOptions::new()
        .filename(directory.join("gitpilot.sqlite3"))
        .create_if_missing(true)
        .synchronous(SqliteSynchronous::Full)
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
    fn upgrades_v1_database_with_patched_sqlite_and_preserves_wal_data() {
        tauri::async_runtime::block_on(async {
            let directory = tempfile::tempdir().unwrap();
            let options = SqliteConnectOptions::new()
                .filename(directory.path().join("gitpilot.sqlite3"))
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
            let old = SqlitePoolOptions::new()
                .connect_with(options)
                .await
                .unwrap();
            let previous = Migrator::with_migrations(
                MIGRATOR
                    .iter()
                    .filter(|m| m.version < 202611090001)
                    .cloned()
                    .collect(),
            );
            previous.run(&old).await.unwrap();
            sqlx::query("INSERT INTO identities (label, git_name, email, is_default) VALUES ('Migration fixture', 'Author', 'author@example.test', 1)")
                .execute(&old).await.unwrap();
            // Keep the earlier pool alive so the new connection also reads WAL state.
            let upgraded = open(directory.path()).await.unwrap();
            let version: String = sqlx::query_scalar("SELECT sqlite_version()")
                .fetch_one(&upgraded)
                .await
                .unwrap();
            let parts: Vec<u32> = version
                .split('.')
                .map(|part| part.parse().unwrap())
                .collect();
            assert!(
                parts.as_slice() >= [3, 54, 1].as_slice(),
                "SQLite {version}"
            );
            let profile: (String, String, bool, Option<String>, bool) = sqlx::query_as(
                "SELECT git_name, email, is_default, ssh_key, signing_required FROM identities WHERE label = 'Migration fixture'",
            ).fetch_one(&upgraded).await.unwrap();
            assert_eq!(
                profile,
                (
                    "Author".into(),
                    "author@example.test".into(),
                    true,
                    None,
                    false
                )
            );
            let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
                .fetch_one(&upgraded)
                .await
                .unwrap();
            assert_eq!(integrity, "ok");
            old.close().await;
            upgraded.close().await;
            let reopened = open(directory.path()).await.unwrap();
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM identities")
                .fetch_one(&reopened)
                .await
                .unwrap();
            assert_eq!(count, 1);
            reopened.close().await;
        });
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn patched_glib_string_iterator_reads_both_ends_in_optimized_builds() {
        use glib::variant::ToVariant;
        let value = ["zero", "one", "two", "three", "four"].to_variant();
        let mut iter = value.array_iter_str().unwrap();
        assert_eq!(iter.next(), Some("zero"));
        assert_eq!(iter.next_back(), Some("four"));
        assert_eq!(iter.nth(1), Some("two"));
        assert_eq!(iter.last(), Some("three"));
        let mut iter = value.array_iter_str().unwrap();
        assert_eq!(iter.nth_back(1), Some("three"));
        assert_eq!(iter.collect::<Vec<_>>(), ["zero", "one", "two"]);
    }

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
