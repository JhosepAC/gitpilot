use crate::{application::Foundation, error::StorageError};

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
