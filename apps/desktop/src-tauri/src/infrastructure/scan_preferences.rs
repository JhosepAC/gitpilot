use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::repository::InventoryError;

const KEY: &str = "inventory.scan";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ScanPreferences {
    pub(crate) time_seconds: u32,
    pub(crate) max_entries: u32,
    pub(crate) max_depth: u32,
    pub(crate) max_repositories: u32,
}

impl ScanPreferences {
    pub(crate) const DEFAULT: Self = Self {
        time_seconds: 12,
        max_entries: 12_000,
        max_depth: 24,
        max_repositories: 200,
    };

    const MIN: Self = Self {
        time_seconds: 1,
        max_entries: 100,
        max_depth: 0,
        max_repositories: 1,
    };

    const MAX: Self = Self {
        time_seconds: 60,
        max_entries: 100_000,
        max_depth: 48,
        max_repositories: 1000,
    };

    fn validate(self) -> Result<Self, InventoryError> {
        if (Self::MIN.time_seconds..=Self::MAX.time_seconds).contains(&self.time_seconds)
            && (Self::MIN.max_entries..=Self::MAX.max_entries).contains(&self.max_entries)
            && (Self::MIN.max_depth..=Self::MAX.max_depth).contains(&self.max_depth)
            && (Self::MIN.max_repositories..=Self::MAX.max_repositories)
                .contains(&self.max_repositories)
        {
            Ok(self)
        } else {
            Err(InventoryError::ScanPreferencesInvalid)
        }
    }
}

#[derive(Serialize)]
pub(crate) struct ScanSettings {
    pub(crate) values: ScanPreferences,
    defaults: ScanPreferences,
    minimum: ScanPreferences,
    maximum: ScanPreferences,
}

impl From<ScanPreferences> for ScanSettings {
    fn from(values: ScanPreferences) -> Self {
        Self {
            values,
            defaults: ScanPreferences::DEFAULT,
            minimum: ScanPreferences::MIN,
            maximum: ScanPreferences::MAX,
        }
    }
}

fn decode(value: Option<&str>) -> Result<ScanPreferences, InventoryError> {
    match value {
        Some(value) => serde_json::from_str::<ScanPreferences>(value)
            .ok()
            .and_then(|value| value.validate().ok())
            .ok_or(InventoryError::StorageUnavailable),
        None => Ok(ScanPreferences::DEFAULT),
    }
}

async fn stored(pool: &SqlitePool) -> Result<Option<String>, InventoryError> {
    sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(KEY)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub(crate) async fn read(pool: &SqlitePool) -> Result<ScanSettings, InventoryError> {
    Ok(decode(stored(pool).await?.as_deref())?.into())
}

pub(crate) async fn save(
    pool: &SqlitePool,
    expected: ScanPreferences,
    values: ScanPreferences,
) -> Result<ScanSettings, InventoryError> {
    let values = values.validate()?;
    let previous = stored(pool).await?;
    if decode(previous.as_deref())? != expected {
        return Err(InventoryError::ScanPreferencesChanged);
    }
    let value = serde_json::to_string(&values).map_err(|_| InventoryError::StorageUnavailable)?;
    let saved: Option<String> = match previous {
        Some(previous) => {
            sqlx::query_scalar("UPDATE app_settings SET value = ? WHERE key = ? AND value = ? RETURNING value")
                .bind(value)
                .bind(KEY)
                .bind(previous)
                .fetch_optional(pool)
                .await?
        }
        None => {
            sqlx::query_scalar("INSERT INTO app_settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO NOTHING RETURNING value")
                .bind(KEY)
                .bind(value)
                .fetch_optional(pool)
                .await?
        }
    };
    let saved = saved.ok_or(InventoryError::ScanPreferencesChanged)?;
    Ok(decode(Some(&saved))?.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage;

    #[test]
    fn validates_and_reopens_saved_preferences_without_overwriting_a_newer_choice() {
        tauri::async_runtime::block_on(async {
            let directory = tempfile::tempdir().unwrap();
            let pool = storage::open(directory.path()).await.unwrap();
            let defaults = read(&pool).await.unwrap().values;
            assert_eq!(defaults, ScanPreferences::DEFAULT);
            let invalid = ScanPreferences {
                max_entries: 99,
                ..defaults
            };
            assert!(matches!(
                save(&pool, defaults, invalid).await,
                Err(InventoryError::ScanPreferencesInvalid)
            ));
            assert_eq!(read(&pool).await.unwrap().values, defaults);
            let selected = ScanPreferences {
                max_entries: 30_000,
                max_depth: 0,
                ..defaults
            };
            assert_eq!(
                save(&pool, defaults, selected).await.unwrap().values,
                selected
            );
            assert!(matches!(
                save(&pool, defaults, defaults).await,
                Err(InventoryError::ScanPreferencesChanged)
            ));
            pool.close().await;
            let reopened = storage::open(directory.path()).await.unwrap();
            assert_eq!(read(&reopened).await.unwrap().values, selected);
            assert_eq!(
                save(&reopened, selected, defaults).await.unwrap().values,
                defaults
            );
        });
    }
}
