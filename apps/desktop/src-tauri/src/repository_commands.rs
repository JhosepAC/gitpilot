use std::path::Path;
use tauri::State;

use crate::{
    application::Foundation,
    error::StorageError,
    infrastructure::{config_apply, config_preview, conflicts, discovery, inspector, inventory},
    repository::{Directory, DirectoryInventory, IdentityDiagnosis, InventoryError, ScanReport},
};

#[tauri::command]
pub(crate) async fn inspect_repository(
    foundation: State<'_, Foundation>,
    id: i64,
) -> Result<inspector::Inspector, InventoryError> {
    let repository = inventory::repository(foundation.database()?, id).await?;
    inspector::inspect(&repository).await
}

#[tauri::command]
pub(crate) async fn native_config_status(
    foundation: State<'_, Foundation>,
    operation_id: Option<i64>,
) -> Result<Option<config_apply::OperationStatus>, config_apply::ApplyError> {
    let pool = foundation
        .database()
        .map_err(|_| config_apply::ApplyError::StorageUnavailable)?;
    config_apply::status(pool, operation_id).await
}

#[tauri::command]
pub(crate) async fn confirm_native_config(
    foundation: State<'_, Foundation>,
    confirmation: String,
    restore: bool,
    repository_id: Option<i64>,
    operation_id: Option<i64>,
) -> Result<(), config_apply::ApplyError> {
    let _guard = foundation
        .inventory_scan
        .try_lock()
        .map_err(|_| config_apply::ApplyError::Busy)?;
    let pool = foundation
        .database()
        .map_err(|_| config_apply::ApplyError::StorageUnavailable)?;
    let app = foundation
        .directory
        .as_deref()
        .ok_or(config_apply::ApplyError::StorageUnavailable)?;
    if restore {
        config_apply::restore(pool, app, &confirmation, operation_id).await
    } else {
        config_apply::apply(pool, app, &confirmation, repository_id).await
    }
}

#[tauri::command]
pub(crate) async fn native_activity(
    foundation: State<'_, Foundation>,
) -> Result<Vec<config_apply::ActivityItem>, config_apply::ApplyError> {
    let pool = foundation
        .database()
        .map_err(|_| config_apply::ApplyError::StorageUnavailable)?;
    config_apply::activity(pool).await
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetupStatus {
    git_version: Option<String>,
    compatible_git: bool,
    identities: i64,
    directories: i64,
    assigned_directories: i64,
    applied: bool,
}

#[tauri::command]
pub(crate) async fn setup_status(
    foundation: State<'_, Foundation>,
) -> Result<SetupStatus, InventoryError> {
    let pool = foundation.database()?;
    let (identities, directories, assigned_directories, applied) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM identities), (SELECT COUNT(*) FROM directories),
            (SELECT COUNT(*) FROM identity_rules WHERE directory_id IS NOT NULL),
            EXISTS (SELECT 1 FROM config_operations WHERE repository_id IS NULL AND state = 'applied')",
    ).fetch_one(pool).await?;
    let (git_version, compatible_git) = match foundation.directory.as_deref() {
        Some(directory) => config_preview::version_status(directory).await,
        None => (None, false),
    };
    Ok(SetupStatus {
        git_version,
        compatible_git,
        identities,
        directories,
        assigned_directories,
        applied,
    })
}

#[tauri::command]
pub(crate) async fn keep_repository_identity(
    foundation: State<'_, Foundation>,
    id: i64,
    fingerprint: String,
) -> Result<(), InventoryError> {
    conflicts::keep(foundation.database()?, id, &fingerprint).await
}

#[tauri::command]
pub(crate) async fn preview_local_identity(
    foundation: State<'_, Foundation>,
    id: i64,
) -> Result<conflicts::LocalPreview, config_apply::ApplyError> {
    let pool = foundation
        .database()
        .map_err(|_| config_apply::ApplyError::StorageUnavailable)?;
    conflicts::prepare(pool, id).await
}

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
    conflicts::diagnose(pool, &repo).await
}
