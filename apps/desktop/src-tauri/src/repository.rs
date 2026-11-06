use serde::Serialize;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InventoryError {
    InvalidPath,
    OutsideDirectory,
    AlreadyRegistered,
    NotFound,
    Conflict,
    ScanInProgress,
    GitUnavailable,
    GitReadFailed,
    StorageUnavailable,
}

#[derive(sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Directory {
    pub(crate) id: i64,
    pub(crate) path: String,
    pub(crate) revision: i64,
    pub(crate) scan_partial: Option<bool>,
    pub(crate) identity_id: Option<i64>,
}

#[derive(sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Repository {
    pub(crate) id: i64,
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) git_dir: String,
    pub(crate) common_dir: String,
    pub(crate) revision: i64,
    pub(crate) present: bool,
    pub(crate) identity_id: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectoryInventory {
    pub(crate) exclusions: Vec<String>,
    pub(crate) repositories: Vec<Repository>,
}

pub(crate) struct DiscoveredRepository {
    pub(crate) path: String,
    pub(crate) kind: &'static str,
    pub(crate) git_dir: String,
    pub(crate) common_dir: String,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanReport {
    pub(crate) found: usize,
    pub(crate) skipped: usize,
    pub(crate) partial: bool,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ObservedIdentity {
    pub(crate) name: Option<ConfigValue>,
    pub(crate) email: Option<ConfigValue>,
    pub(crate) author: Option<String>,
    pub(crate) committer: Option<String>,
}

#[derive(Serialize, PartialEq, Eq)]
pub(crate) struct ConfigValue {
    pub(crate) value: String,
    pub(crate) scope: String,
    pub(crate) origin: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExpectedIdentity {
    pub(crate) label: String,
    pub(crate) git_name: String,
    pub(crate) email: String,
    pub(crate) source: &'static str,
    pub(crate) directory: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IdentityDiagnosis {
    pub(crate) expected: Option<ExpectedIdentity>,
    pub(crate) observed: Option<ObservedIdentity>,
    pub(crate) conflict: bool,
    pub(crate) conflict_fingerprint: Option<String>,
    pub(crate) kept: bool,
    pub(crate) observation_error: Option<InventoryError>,
}
