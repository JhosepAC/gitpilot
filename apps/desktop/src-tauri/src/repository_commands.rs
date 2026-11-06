use std::path::Path;
use tauri::State;

use crate::{
    application::Foundation,
    error::StorageError,
    infrastructure::{config_preview, discovery, inventory},
    repository::{Directory, DirectoryInventory, IdentityDiagnosis, InventoryError, ScanReport},
};

#[tauri::command]
pub(crate) async fn preview_native_config(
    foundation: State<'_, Foundation>,
) -> Result<config_preview::ConfigPreview, InventoryError> {
    let _guard = foundation
        .inventory_scan
        .try_lock()
        .map_err(|_| InventoryError::ScanInProgress)?;
    let app_data = foundation
        .directory
        .as_deref()
        .ok_or(InventoryError::StorageUnavailable)?;
    config_preview::prepare(foundation.database()?, app_data).await
}

impl From<StorageError> for InventoryError {
    fn from(_: StorageError) -> Self {
        Self::StorageUnavailable
    }
}

#[tauri::command]
pub(crate) async fn list_directories(
    foundation: State<'_, Foundation>,
) -> Result<Vec<Directory>, InventoryError> {
    inventory::directories(foundation.database()?).await
}

#[tauri::command]
pub(crate) async fn add_directory(
    foundation: State<'_, Foundation>,
    path: String,
) -> Result<i64, InventoryError> {
    let path = discovery::directory_path(Path::new(&path)).await?;
    inventory::add(foundation.database()?, &path).await
}

#[tauri::command]
pub(crate) async fn remove_directory(
    foundation: State<'_, Foundation>,
    id: i64,
    revision: i64,
) -> Result<(), InventoryError> {
    inventory::remove(foundation.database()?, id, revision).await
}

#[tauri::command]
pub(crate) async fn list_directory_contents(
    foundation: State<'_, Foundation>,
    id: i64,
) -> Result<DirectoryInventory, InventoryError> {
    inventory::contents(foundation.database()?, id).await
}

#[tauri::command]
pub(crate) async fn set_directory_exclusion(
    foundation: State<'_, Foundation>,
    id: i64,
    revision: i64,
    path: String,
    excluded: bool,
) -> Result<(), InventoryError> {
    let pool = foundation.database()?;
    let directory = inventory::directory(pool, id).await?;
    if directory.revision != revision {
        return Err(InventoryError::Conflict);
    }
    let path = if excluded {
        discovery::directory_path(Path::new(&path)).await?
    } else {
        path
    };
    inventory::set_exclusion(pool, &directory, &path, excluded).await
}

#[tauri::command]
pub(crate) async fn scan_directory(
    foundation: State<'_, Foundation>,
    id: i64,
) -> Result<ScanReport, InventoryError> {
    let _guard = foundation
        .inventory_scan
        .try_lock()
        .map_err(|_| InventoryError::ScanInProgress)?;
    let pool = foundation.database()?;
    let directory = inventory::directory(pool, id).await?;
    let contents = inventory::contents(pool, id).await?;
    let (repos, report) = discovery::scan(&directory.path, &contents.exclusions).await?;
    inventory::save_scan(pool, &directory, repos, report.partial).await?;
    Ok(report)
}

#[tauri::command]
pub(crate) async fn set_inventory_identity(
    foundation: State<'_, Foundation>,
    repository: bool,
    id: i64,
    revision: i64,
    identity_id: Option<i64>,
) -> Result<(), InventoryError> {
    inventory::assign(
        foundation.database()?,
        repository,
        id,
        revision,
        identity_id,
    )
    .await
}

#[tauri::command]
pub(crate) async fn diagnose_repository(
    foundation: State<'_, Foundation>,
    id: i64,
) -> Result<IdentityDiagnosis, InventoryError> {
    let pool = foundation.database()?;
    let repo = inventory::repository(pool, id).await?;
    let expected = inventory::expected(pool, &repo).await?;
    let (observed, observation_error) = if repo.present {
        match discovery::observe(&repo.path, &repo.git_dir).await {
            Ok(observed) => (Some(observed), None),
            Err(error) => (None, Some(error)),
        }
    } else {
        (None, None)
    };
    let conflict = expected
        .as_ref()
        .zip(observed.as_ref())
        .is_some_and(|(expected, observed)| {
            observed
                .name
                .as_ref()
                .is_none_or(|name| name.value != expected.git_name)
                || observed
                    .email
                    .as_ref()
                    .is_none_or(|email| email.value != expected.email)
        });
    Ok(IdentityDiagnosis {
        expected,
        observed,
        conflict,
        observation_error,
    })
}
