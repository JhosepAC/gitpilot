#[derive(Clone, Copy, Debug, serde::Serialize)]
pub(crate) enum StorageError {
    #[serde(rename = "storage_unavailable")]
    Unavailable,
    #[serde(rename = "storage_initialization_failed")]
    InitializationFailed,
    #[serde(rename = "storage_check_failed")]
    CheckFailed,
}
