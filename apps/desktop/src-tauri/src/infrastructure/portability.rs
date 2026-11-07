use std::{
    collections::{HashMap, HashSet, VecDeque},
    io::Write,
    path::Path,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{SqliteConnection, SqlitePool};

use super::scan_preferences::{self, ScanPreferences};
use crate::{error::StorageError, identity::IdentityInput};

const VERSION: u64 = 1;
const MAX_BYTES: usize = 1024 * 1024;
const LANGUAGE_KEY: &str = "ui.language";

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PortableError {
    FileTooLarge,
    InvalidFile,
    UnsupportedVersion,
    StalePreview,
    StorageUnavailable,
    FileExists,
    FileWriteFailed,
    DialogFailed,
}

impl From<StorageError> for PortableError {
    fn from(_: StorageError) -> Self {
        Self::StorageUnavailable
    }
}

impl From<sqlx::Error> for PortableError {
    fn from(_: sqlx::Error) -> Self {
        Self::StorageUnavailable
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Language {
    #[default]
    Es,
    En,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Preferences {
    pub(crate) language: Language,
    scan: ScanPreferences,
}

#[derive(Deserialize, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Profile {
    id: i64,
    label: String,
    git_name: String,
    email: String,
    is_default: bool,
}

impl Profile {
    fn key(&self) -> (&str, &str, &str) {
        (&self.label, &self.git_name, &self.email)
    }
}

#[derive(Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Scope {
    Directory,
    Repository,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Rule {
    scope: Scope,
    path: String,
    profile_id: i64,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Archive {
    version: u64,
    profiles: Vec<Profile>,
    rules: Vec<Rule>,
    preferences: Preferences,
}

#[derive(Serialize, sqlx::FromRow)]
struct Target {
    id: i64,
    path: String,
    profile_id: Option<i64>,
    available: bool,
}

#[derive(Serialize)]
struct Local {
    profiles: Vec<Profile>,
    directories: Vec<Target>,
    repositories: Vec<Target>,
    preferences: Preferences,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RuleDecision {
    Add,
    Existing,
    Conflict,
    Missing,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileReview<'a> {
    label: &'a str,
    git_name: &'a str,
    email: &'a str,
    reuse: bool,
    make_default: bool,
    keep_local_default: bool,
    #[serde(skip)]
    local_id: Option<i64>,
}

#[derive(Serialize)]
struct RuleReview<'a> {
    scope: Scope,
    path: &'a str,
    profile: &'a str,
    decision: RuleDecision,
    #[serde(skip)]
    target_id: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Review<'a> {
    token: String,
    profiles: Vec<ProfileReview<'a>>,
    rules: Vec<RuleReview<'a>>,
    before: Preferences,
    after: Preferences,
}

fn encode(value: &impl Serialize) -> Result<Vec<u8>, PortableError> {
    serde_json::to_vec(value).map_err(|_| PortableError::StorageUnavailable)
}

async fn setting(db: &mut SqliteConnection, key: &str) -> Result<Option<String>, PortableError> {
    Ok(
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(db)
            .await?,
    )
}

async fn put(
    db: &mut SqliteConnection,
    key: &str,
    value: &impl Serialize,
) -> Result<(), PortableError> {
    let json = serde_json::to_string(value).map_err(|_| PortableError::StorageUnavailable)?;
    sqlx::query("INSERT INTO app_settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key).bind(json).execute(db).await?;
    Ok(())
}

async fn language(db: &mut SqliteConnection) -> Result<Language, PortableError> {
    setting(db, LANGUAGE_KEY)
        .await?
        .map(|value| serde_json::from_str(&value).map_err(|_| PortableError::StorageUnavailable))
        .transpose()
        .map(Option::unwrap_or_default)
}

pub(crate) async fn read_language(pool: &SqlitePool) -> Result<Language, PortableError> {
    language(&mut *pool.acquire().await?).await
}

pub(crate) async fn save_language(pool: &SqlitePool, value: Language) -> Result<(), PortableError> {
    put(&mut *pool.acquire().await?, LANGUAGE_KEY, &value).await
}

async fn snapshot(db: &mut SqliteConnection) -> Result<Local, PortableError> {
    Ok(Local {
        profiles: sqlx::query_as("SELECT id, label, git_name, email, is_default FROM identities ORDER BY id")
            .fetch_all(&mut *db).await?,
        directories: sqlx::query_as(
            "SELECT d.id, d.path, i.identity_id AS profile_id, 1 AS available FROM directories d
            LEFT JOIN identity_rules i ON d.id = i.directory_id ORDER BY d.id",
        ).fetch_all(&mut *db).await?,
        repositories: sqlx::query_as(
            "SELECT r.id, r.path, i.identity_id AS profile_id,
            EXISTS(SELECT 1 FROM directory_repositories dr WHERE dr.repository_id = r.id AND dr.present = 1) AS available
            FROM repositories r LEFT JOIN identity_rules i ON r.id = i.repository_id ORDER BY r.id",
        ).fetch_all(&mut *db).await?,
        preferences: Preferences {
            language: language(db).await?,
            scan: scan_preferences::decode(setting(db, scan_preferences::KEY).await?.as_deref())
                .map_err(|_| PortableError::StorageUnavailable)?,
        },
    })
}

fn portable_path(path: &str) -> bool {
    if path.len() > 4096 || path.chars().any(char::is_control) {
        return false;
    }
    let windows = path.starts_with(r"\\")
        || matches!(path.as_bytes(), [drive, b':', b'\\' | b'/', ..] if drive.is_ascii_alphabetic());
    if windows {
        !path
            .split(['/', '\\'])
            .any(|part| matches!(part, "." | ".."))
    } else {
        path.starts_with('/') && !path.split('/').any(|part| matches!(part, "." | ".."))
    }
}

fn decode(contents: &str) -> Result<Archive, PortableError> {
    if contents.len() > MAX_BYTES {
        return Err(PortableError::FileTooLarge);
    }
    #[derive(Deserialize)]
    struct Header {
        version: u64,
    }
    let contents = contents.strip_prefix('\u{feff}').unwrap_or(contents);
    let header: Header = serde_json::from_str(contents).map_err(|_| PortableError::InvalidFile)?;
    if header.version != VERSION {
        return Err(PortableError::UnsupportedVersion);
    }
    let archive: Archive =
        serde_json::from_str(contents).map_err(|_| PortableError::InvalidFile)?;
    let mut ids = HashSet::new();
    let mut has_default = false;
    for profile in &archive.profiles {
        if profile.id <= 0 || !ids.insert(profile.id) || (profile.is_default && has_default) {
            return Err(PortableError::InvalidFile);
        }
        let input = IdentityInput {
            label: profile.label.clone(),
            git_name: profile.git_name.clone(),
            email: profile.email.clone(),
        }
        .validate()
        .map_err(|_| PortableError::InvalidFile)?;
        if (
            input.label.as_str(),
            input.git_name.as_str(),
            input.email.as_str(),
        ) != profile.key()
        {
            return Err(PortableError::InvalidFile);
        }
        has_default |= profile.is_default;
    }
    let mut targets = HashSet::new();
    for rule in &archive.rules {
        if !portable_path(&rule.path)
            || !ids.contains(&rule.profile_id)
            || !targets.insert((rule.scope, rule.path.as_str()))
        {
            return Err(PortableError::InvalidFile);
        }
    }
    archive
        .preferences
        .scan
        .validate()
        .map_err(|_| PortableError::InvalidFile)?;
    Ok(archive)
}

fn review<'a>(archive: &'a Archive, local: &Local) -> Result<Review<'a>, PortableError> {
    let token = hex::encode(Sha256::digest(encode(&(archive, local))?));
    let mut matches = HashMap::<_, VecDeque<_>>::new();
    for profile in &local.profiles {
        matches.entry(profile.key()).or_default().push_back(profile);
    }
    let has_default = local.profiles.iter().any(|profile| profile.is_default);
    let profiles: Vec<_> = archive
        .profiles
        .iter()
        .map(|profile| {
            let existing = matches
                .get_mut(&profile.key())
                .and_then(VecDeque::pop_front);
            ProfileReview {
                label: &profile.label,
                git_name: &profile.git_name,
                email: &profile.email,
                reuse: existing.is_some(),
                make_default: profile.is_default && !has_default,
                keep_local_default: profile.is_default
                    && has_default
                    && !existing.is_some_and(|p| p.is_default),
                local_id: existing.map(|p| p.id),
            }
        })
        .collect();
    let profile_indices: HashMap<_, _> = archive
        .profiles
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id, i))
        .collect();
    let directories: HashMap<_, _> = local
        .directories
        .iter()
        .map(|t| (t.path.as_str(), t))
        .collect();
    let repositories: HashMap<_, _> = local
        .repositories
        .iter()
        .map(|t| (t.path.as_str(), t))
        .collect();
    let rules = archive
        .rules
        .iter()
        .map(|rule| {
            let profile = &profiles[profile_indices[&rule.profile_id]];
            let target = match rule.scope {
                Scope::Directory => directories.get(rule.path.as_str()),
                Scope::Repository => repositories.get(rule.path.as_str()),
            }
            .filter(|t| t.available);
            let decision = match target.map(|t| t.profile_id) {
                None => RuleDecision::Missing,
                Some(None) => RuleDecision::Add,
                Some(Some(id)) if Some(id) == profile.local_id => RuleDecision::Existing,
                Some(Some(_)) => RuleDecision::Conflict,
            };
            RuleReview {
                scope: rule.scope,
                path: &rule.path,
                profile: profile.label,
                decision,
                target_id: target.map(|t| t.id),
            }
        })
        .collect();
    Ok(Review {
        token,
        profiles,
        rules,
        before: local.preferences,
        after: archive.preferences,
    })
}

pub(crate) async fn preview(
    pool: &SqlitePool,
    contents: &str,
) -> Result<serde_json::Value, PortableError> {
    let archive = decode(contents)?;
    let mut tx = pool.begin().await?;
    let local = snapshot(&mut tx).await?;
    tx.commit().await?;
    serde_json::to_value(review(&archive, &local)?).map_err(|_| PortableError::StorageUnavailable)
}

pub(crate) async fn import(
    pool: &SqlitePool,
    contents: &str,
    token: &str,
) -> Result<Preferences, PortableError> {
    let archive = decode(contents)?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let local = snapshot(&mut tx).await?;
    let review = review(&archive, &local)?;
    if review.token != token {
        return Err(PortableError::StalePreview);
    }

    let mut ids = HashMap::new();
    for (profile, entry) in archive.profiles.iter().zip(&review.profiles) {
        let id = if let Some(id) = entry.local_id {
            if entry.make_default {
                sqlx::query("UPDATE identities SET is_default = 1, revision = revision + 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(id).execute(&mut *tx).await?;
            }
            id
        } else {
            sqlx::query(
                "INSERT INTO identities (label, git_name, email, is_default) VALUES (?, ?, ?, ?)",
            )
            .bind(&profile.label)
            .bind(&profile.git_name)
            .bind(&profile.email)
            .bind(entry.make_default)
            .execute(&mut *tx)
            .await?
            .last_insert_rowid()
        };
        ids.insert(profile.id, id);
    }
    for (rule, entry) in archive.rules.iter().zip(&review.rules) {
        if let (RuleDecision::Add, Some(target)) = (entry.decision, entry.target_id) {
            let (insert, revise) = match rule.scope {
                Scope::Directory => (
                    "INSERT INTO identity_rules (identity_id, directory_id) VALUES (?, ?)",
                    "UPDATE directories SET revision = revision + 1 WHERE id = ?",
                ),
                Scope::Repository => (
                    "INSERT INTO identity_rules (identity_id, repository_id) VALUES (?, ?)",
                    "UPDATE repositories SET revision = revision + 1 WHERE id = ?",
                ),
            };
            sqlx::query(insert)
                .bind(ids[&rule.profile_id])
                .bind(target)
                .execute(&mut *tx)
                .await?;
            sqlx::query(revise).bind(target).execute(&mut *tx).await?;
        }
    }
    if review.before.language != review.after.language {
        put(&mut tx, LANGUAGE_KEY, &review.after.language).await?;
    }
    if review.before.scan != review.after.scan {
        put(&mut tx, scan_preferences::KEY, &review.after.scan).await?;
    }
    record(&mut tx, "imported", archive.profiles.len()).await?;
    tx.commit().await?;
    Ok(archive.preferences)
}

pub(crate) async fn export(pool: &SqlitePool) -> Result<(Vec<u8>, usize), PortableError> {
    let mut tx = pool.begin().await?;
    let local = snapshot(&mut tx).await?;
    tx.commit().await?;
    let rules = [
        (Scope::Directory, local.directories),
        (Scope::Repository, local.repositories),
    ]
    .into_iter()
    .flat_map(|(scope, targets)| {
        targets.into_iter().filter_map(move |target| {
            Some(Rule {
                scope,
                path: target.path,
                profile_id: target.profile_id?,
            })
        })
    })
    .collect();
    let count = local.profiles.len();
    let bytes = serde_json::to_vec_pretty(&Archive {
        version: VERSION,
        profiles: local.profiles,
        rules,
        preferences: local.preferences,
    })
    .map_err(|_| PortableError::StorageUnavailable)?;
    decode(std::str::from_utf8(&bytes).map_err(|_| PortableError::InvalidFile)?)?;
    Ok((bytes, count))
}

pub(crate) fn save_file(path: &Path, bytes: &[u8]) -> Result<(), PortableError> {
    let parent = path.parent().ok_or(PortableError::FileWriteFailed)?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| PortableError::FileWriteFailed)?;
    temporary
        .write_all(bytes)
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|_| PortableError::FileWriteFailed)?;
    temporary.persist_noclobber(path).map_err(|problem| {
        if problem.error.kind() == std::io::ErrorKind::AlreadyExists {
            PortableError::FileExists
        } else {
            PortableError::FileWriteFailed
        }
    })?;
    Ok(())
}

async fn record(
    db: &mut SqliteConnection,
    action: &str,
    count: usize,
) -> Result<(), PortableError> {
    sqlx::query("INSERT INTO portable_activity (action, profile_count) VALUES (?, ?)")
        .bind(action)
        .bind(count as i64)
        .execute(db)
        .await?;
    Ok(())
}

pub(crate) async fn record_export(pool: &SqlitePool, count: usize) -> Result<(), PortableError> {
    record(&mut *pool.acquire().await?, "exported", count).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::infrastructure::{config_apply, storage};

    fn fixture() -> serde_json::Value {
        json!({
            "version": 1,
            "profiles": [
                { "id": 90, "label": "Work", "gitName": "Pat Example", "email": "pat@example.test", "isDefault": true },
                { "id": 91, "label": "Personal", "gitName": "Pat Example", "email": "personal@example.test", "isDefault": false }
            ],
            "rules": [
                { "scope": "directory", "path": "/work", "profileId": 90 },
                { "scope": "directory", "path": "/personal", "profileId": 91 },
                { "scope": "repository", "path": "/work/project", "profileId": 90 },
                { "scope": "repository", "path": "/work/missing", "profileId": 90 },
                { "scope": "repository", "path": "C:\\project", "profileId": 90 }
            ],
            "preferences": {
                "language": "en",
                "scan": { "timeSeconds": 20, "maxEntries": 12000, "maxDepth": 24, "maxRepositories": 200 }
            }
        })
    }

    #[test]
    fn rejects_unknown_versions_invalid_references_and_unsafe_paths() {
        let mut archive = fixture();
        assert!(decode(&archive.to_string()).is_ok());
        archive["version"] = json!(2);
        assert!(matches!(
            decode(&archive.to_string()),
            Err(PortableError::UnsupportedVersion)
        ));
        archive["version"] = json!(1);
        archive["rules"][0]["path"] = json!("/work/../elsewhere");
        assert!(matches!(
            decode(&archive.to_string()),
            Err(PortableError::InvalidFile)
        ));
        archive["rules"][0]["path"] = json!("/work");
        archive["rules"][0]["profileId"] = json!(999);
        assert!(matches!(
            decode(&archive.to_string()),
            Err(PortableError::InvalidFile)
        ));
        archive["rules"][0]["profileId"] = json!(90);
        archive["profiles"][0]["token"] = json!("unexpected");
        assert!(matches!(
            decode(&archive.to_string()),
            Err(PortableError::InvalidFile)
        ));
        assert!(matches!(
            decode(&" ".repeat(MAX_BYTES + 1)),
            Err(PortableError::FileTooLarge)
        ));
    }

    #[tokio::test]
    async fn previews_and_imports_local_settings_without_replacing_conflicts() {
        let directory = tempfile::tempdir().unwrap();
        let pool = storage::open(directory.path()).await.unwrap();
        sqlx::raw_sql(
            "INSERT INTO identities (label, git_name, email, is_default) VALUES
                ('Local', 'Local Author', 'local@example.test', 1),
                ('Personal', 'Pat Example', 'personal@example.test', 0);
             INSERT INTO directories (path) VALUES ('/work'), ('/personal');
             INSERT INTO repositories (path, kind, git_dir, common_dir) VALUES
                ('/work/project', 'ordinary', '/work/project/.git', '/work/project/.git'),
                ('/work/missing', 'ordinary', '/work/missing/.git', '/work/missing/.git');
             INSERT INTO directory_repositories (directory_id, repository_id, present) VALUES (1, 1, 1), (1, 2, 0);
             INSERT INTO identity_rules (identity_id, directory_id) VALUES (1, 1);
             INSERT INTO app_settings (key, value) VALUES ('unrelated', 'private-fixture');",
        ).execute(&pool).await.unwrap();

        let contents = fixture().to_string();
        let review = preview(&pool, &contents).await.unwrap();
        assert_eq!(review["before"]["language"], "es");
        assert_eq!(review["profiles"][0]["keepLocalDefault"], true);
        assert_eq!(review["profiles"][1]["reuse"], true);
        for (index, expected) in ["conflict", "add", "add", "missing", "missing"]
            .iter()
            .enumerate()
        {
            assert_eq!(review["rules"][index]["decision"], *expected);
        }
        let token = review["token"].as_str().unwrap();
        save_language(&pool, Language::En).await.unwrap();
        assert!(matches!(
            import(&pool, &contents, token).await,
            Err(PortableError::StalePreview)
        ));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM identities")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM portable_activity")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );

        let review = preview(&pool, &contents).await.unwrap();
        let token = review["token"].as_str().unwrap();
        let imported = import(&pool, &contents, token).await.unwrap();
        assert_eq!(imported.scan.time_seconds, 20);
        let rules: Vec<(i64, Option<i64>, Option<i64>)> = sqlx::query_as(
            "SELECT identity_id, directory_id, repository_id FROM identity_rules ORDER BY identity_id",
        ).fetch_all(&pool).await.unwrap();
        assert_eq!(
            rules,
            [(1, Some(1), None), (2, Some(2), None), (3, None, Some(1))]
        );
        let activity = serde_json::to_value(config_apply::activity(&pool).await.unwrap()).unwrap();
        assert_eq!(activity[0]["scope"], "data");
        assert_eq!(activity[0]["state"], "imported");
        assert_eq!(activity[0]["profileCount"], 2);
        let repeated = preview(&pool, &contents).await.unwrap();
        assert_eq!(repeated["profiles"][0]["reuse"], true);
        assert_eq!(repeated["rules"][2]["decision"], "existing");

        let (bytes, count) = export(&pool).await.unwrap();
        let file = std::str::from_utf8(&bytes).unwrap();
        assert!(!file.contains("private-fixture"));
        assert!(!file.contains("portable_activity"));
        let archive = decode(file).unwrap();
        assert_eq!(count, 3);
        assert_eq!(archive.rules.len(), 3);
        assert!(archive.profiles[0].is_default);
        pool.close().await;
        let reopened = storage::open(directory.path()).await.unwrap();
        assert_eq!(read_language(&reopened).await.unwrap(), Language::En);
        reopened.close().await;
    }

    #[tokio::test]
    async fn rolls_back_import_if_the_activity_write_fails() {
        let directory = tempfile::tempdir().unwrap();
        let pool = storage::open(directory.path()).await.unwrap();
        sqlx::query("CREATE TRIGGER reject_activity BEFORE INSERT ON portable_activity BEGIN SELECT RAISE(ABORT, 'fixture'); END")
            .execute(&pool).await.unwrap();
        let contents = fixture().to_string();
        let review = preview(&pool, &contents).await.unwrap();
        assert!(matches!(
            import(&pool, &contents, review["token"].as_str().unwrap()).await,
            Err(PortableError::StorageUnavailable)
        ));
        assert_eq!(read_language(&pool).await.unwrap(), Language::Es);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM identities")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        pool.close().await;
    }

    #[test]
    fn export_never_replaces_an_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("settings.json");
        save_file(&file, b"first").unwrap();
        assert_eq!(save_file(&file, b"second"), Err(PortableError::FileExists));
        assert_eq!(std::fs::read(file).unwrap(), b"first");
    }
}
