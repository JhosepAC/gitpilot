use serde::Serialize;
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::{
    application::Foundation,
    infrastructure::portability::{self, Language, PortableError, Preferences},
};

#[tauri::command]
pub(crate) async fn read_language(
    foundation: State<'_, Foundation>,
) -> Result<Language, PortableError> {
    portability::read_language(foundation.database()?).await
}

#[tauri::command]
pub(crate) async fn save_language(
    foundation: State<'_, Foundation>,
    language: Language,
) -> Result<(), PortableError> {
    portability::save_language(foundation.database()?, language).await
}

#[tauri::command]
pub(crate) async fn preview_portable_settings(
    foundation: State<'_, Foundation>,
    contents: String,
) -> Result<serde_json::Value, PortableError> {
    portability::preview(foundation.database()?, &contents).await
}

#[tauri::command]
pub(crate) async fn import_portable_settings(
    foundation: State<'_, Foundation>,
    contents: String,
    token: String,
) -> Result<Preferences, PortableError> {
    portability::import(foundation.database()?, &contents, &token).await
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExportOutcome {
    Cancelled,
    Saved,
    SavedWithoutActivity,
}

#[tauri::command]
pub(crate) async fn export_portable_settings(
    window: WebviewWindow,
    foundation: State<'_, Foundation>,
) -> Result<ExportOutcome, PortableError> {
    let pool = foundation.database()?;
    let (bytes, count) = portability::export(pool).await?;
    let (send, receive) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(&window)
        .set_file_name("gitpilot-settings.json")
        .add_filter("JSON", &["json"])
        .save_file(move |path| {
            let _ = send.send(path);
        });
    let Some(path) = receive.await.map_err(|_| PortableError::DialogFailed)? else {
        return Ok(ExportOutcome::Cancelled);
    };
    let path = path
        .into_path()
        .map_err(|_| PortableError::FileWriteFailed)?;
    tokio::task::spawn_blocking(move || portability::save_file(&path, &bytes))
        .await
        .map_err(|_| PortableError::FileWriteFailed)??;
    Ok(if portability::record_export(pool, count).await.is_ok() {
        ExportOutcome::Saved
    } else {
        ExportOutcome::SavedWithoutActivity
    })
}
