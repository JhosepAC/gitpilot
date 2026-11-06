use crate::{
    application::Foundation,
    error::StorageError,
    identity::{Identity, IdentityError, IdentityInput},
    infrastructure::{git_identity, identities},
};

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum DatabaseStatus {
    Ready,
}

#[derive(serde::Serialize)]
pub(crate) struct FoundationStatus {
    database: DatabaseStatus,
}

#[tauri::command]
pub(crate) async fn foundation_status(
    foundation: tauri::State<'_, Foundation>,
) -> Result<FoundationStatus, StorageError> {
    foundation.check_database().await?;
    Ok(FoundationStatus {
        database: DatabaseStatus::Ready,
    })
}

impl From<StorageError> for IdentityError {
    fn from(_: StorageError) -> Self {
        Self::StorageUnavailable
    }
}

#[tauri::command]
pub(crate) async fn list_identities(
    foundation: tauri::State<'_, Foundation>,
) -> Result<Vec<Identity>, IdentityError> {
    identities::list(foundation.database()?).await
}

#[tauri::command]
pub(crate) async fn create_identity(
    foundation: tauri::State<'_, Foundation>,
    input: IdentityInput,
) -> Result<i64, IdentityError> {
    identities::create(foundation.database()?, input).await
}

#[tauri::command]
pub(crate) async fn update_identity(
    foundation: tauri::State<'_, Foundation>,
    id: i64,
    revision: i64,
    input: IdentityInput,
) -> Result<(), IdentityError> {
    identities::update(foundation.database()?, id, revision, input).await
}

#[tauri::command]
pub(crate) async fn duplicate_identity(
    foundation: tauri::State<'_, Foundation>,
    id: i64,
    revision: i64,
) -> Result<i64, IdentityError> {
    identities::duplicate(foundation.database()?, id, revision).await
}

#[tauri::command]
pub(crate) async fn set_identity_default(
    foundation: tauri::State<'_, Foundation>,
    id: i64,
    revision: i64,
    enabled: bool,
) -> Result<(), IdentityError> {
    identities::set_default(foundation.database()?, id, revision, enabled).await
}

#[tauri::command]
pub(crate) async fn delete_identity(
    foundation: tauri::State<'_, Foundation>,
    id: i64,
    revision: i64,
) -> Result<(), IdentityError> {
    identities::delete(foundation.database()?, id, revision).await
}

#[tauri::command]
pub(crate) async fn detect_git_identity(
    foundation: tauri::State<'_, Foundation>,
) -> Result<IdentityInput, IdentityError> {
    let directory = foundation
        .directory
        .as_deref()
        .ok_or(IdentityError::StorageUnavailable)?;
    git_identity::detect(directory).await
}
