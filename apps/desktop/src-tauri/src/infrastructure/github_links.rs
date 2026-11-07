use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::repository::Repository as LocalRepository;

use super::super::{
    inspector,
    inspector_probe::{self, ProbeKind},
    inventory,
};
use super::{
    github_accounts,
    resources::{valid_full_name, AccessError},
    Github, GithubError, RepositoryScope, ResourceError, CLIENT_ID,
};

const REMOTE_LIMIT: usize = 20;

#[derive(sqlx::FromRow)]
struct Link {
    remote: String,
    github_id: i64,
    installation_id: i64,
    repository_id: i64,
    fingerprint: String,
    version: String,
}

impl Link {
    fn matches(&self, scope: RepositoryScope) -> bool {
        self.github_id == scope.github_id
            && self.installation_id == scope.installation_id
            && self.repository_id == scope.repository_id
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalRemote {
    name: String,
    destination: Option<String>,
    fingerprint: Option<String>,
    version: Option<String>,
    state: &'static str,
}

#[derive(Serialize)]
pub(crate) struct RemoteCandidates {
    remotes: Vec<LocalRemote>,
    partial: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteReview {
    pub(super) local_id: i64,
    pub(super) remote: String,
    pub(super) fingerprint: String,
    pub(super) version: Option<String>,
}

struct Snapshot {
    destination: String,
    fingerprint: String,
}

pub(crate) async fn candidates(
    pool: &SqlitePool,
    local_id: i64,
    scope: Option<RepositoryScope>,
) -> Result<RemoteCandidates, ResourceError> {
    let local = local_repository(pool, local_id).await?;
    let links: BTreeMap<_, _> = sqlx::query_as::<_, Link>(
        "SELECT remote, github_id, installation_id, repository_id, fingerprint, version
         FROM github_remote_links WHERE local_repository_id = ? ORDER BY remote LIMIT ?",
    )
    .bind(local_id)
    .bind((REMOTE_LIMIT + 1) as i64)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|link| (link.remote.clone(), link))
    .collect();
    if scope.is_none() && links.is_empty() {
        return Ok(RemoteCandidates {
            remotes: Vec::new(),
            partial: false,
        });
    }
    let (exe, cwd) = inspector::context(&local)
        .await
        .map_err(|_| AccessError::LocalRepositoryUnavailable)?;
    let names: BTreeSet<String> = if scope.is_some() {
        let (status, output) = inspector::run(&exe, cwd, &["remote"])
            .await
            .map_err(|_| AccessError::LocalRepositoryUnavailable)?;
        if !status.success() {
            return Err(AccessError::LocalRepositoryUnavailable.into());
        }
        String::from_utf8_lossy(&output)
            .lines()
            .filter(|name| valid_remote(name))
            .map(str::to_owned)
            .collect()
    } else {
        links.keys().cloned().collect()
    };
    let mut partial = names.len() > REMOTE_LIMIT || links.len() > REMOTE_LIMIT;
    let mut remotes = Vec::new();
    partial |= tokio::time::timeout(Duration::from_secs(8), async {
        for name in names.iter().map(String::as_str).take(REMOTE_LIMIT) {
            let observed = snapshot(&exe, cwd, &local.git_dir, name).await;
            if observed.is_some() || links.contains_key(name) {
                remotes.push(candidate(name, observed, links.get(name), scope));
            }
        }
    })
    .await
    .is_err();
    for (name, link) in links.iter().take(REMOTE_LIMIT) {
        if !remotes.iter().any(|remote| remote.name == *name) {
            remotes.push(candidate(name, None, Some(link), scope));
        }
    }
    Ok(RemoteCandidates { remotes, partial })
}

fn candidate(
    name: &str,
    observed: Option<Snapshot>,
    link: Option<&Link>,
    scope: Option<RepositoryScope>,
) -> LocalRemote {
    let state = match link {
        Some(link) if scope.is_some_and(|scope| !link.matches(scope)) => "other",
        Some(link)
            if observed
                .as_ref()
                .is_some_and(|value| value.fingerprint == link.fingerprint) =>
        {
            "linked"
        }
        Some(_) => "review",
        None => "available",
    };
    let (destination, fingerprint) = observed.map_or((None, None), |value| {
        (Some(value.destination), Some(value.fingerprint))
    });
    LocalRemote {
        name: name.into(),
        destination,
        fingerprint,
        version: link.map(|link| link.version.clone()),
        state,
    }
}

impl Github {
    pub(crate) async fn link_remote(
        &self,
        pool: &SqlitePool,
        app: &Path,
        scope: RepositoryScope,
        review: RemoteReview,
    ) -> Result<(), ResourceError> {
        scope.validate()?;
        if !valid_remote(&review.remote) {
            return Err(GithubError::Protocol.into());
        }
        let mut session = self.resource_session(pool, app, scope.github_id).await?;
        let repository = session.authorized_repository(scope).await?;
        let local = local_repository(pool, review.local_id).await?;
        let (exe, cwd) = inspector::context(&local)
            .await
            .map_err(|_| AccessError::LocalRepositoryUnavailable)?;
        let observed = snapshot(&exe, cwd, &local.git_dir, &review.remote).await;
        if !observed.is_some_and(|value| {
            value.fingerprint == review.fingerprint
                && value
                    .destination
                    .eq_ignore_ascii_case(&repository.full_name)
        }) {
            return Err(AccessError::RemoteChanged.into());
        }
        save_link(pool, scope, &review).await
    }
}

async fn save_link(
    pool: &SqlitePool,
    scope: RepositoryScope,
    review: &RemoteReview,
) -> Result<(), ResourceError> {
    let mut tx = pool.begin().await?;
    let previous: Option<String> = sqlx::query_scalar(
        "SELECT version FROM github_remote_links WHERE local_repository_id = ? AND remote = ?",
    )
    .bind(review.local_id)
    .bind(&review.remote)
    .fetch_optional(&mut *tx)
    .await?;
    if previous != review.version {
        return Err(AccessError::LinkChanged.into());
    }
    let result = sqlx::query(
        "INSERT INTO github_remote_links (local_repository_id, remote, client_id, github_id, installation_id, repository_id, fingerprint, version)
         SELECT ?, ?, ?, ?, ?, ?, ?, ? WHERE EXISTS (
            SELECT 1 FROM directory_repositories WHERE repository_id = ? AND present = 1
         )
         ON CONFLICT(local_repository_id, remote) DO UPDATE SET
            client_id = excluded.client_id, github_id = excluded.github_id, installation_id = excluded.installation_id,
            repository_id = excluded.repository_id, fingerprint = excluded.fingerprint, version = excluded.version",
    ).bind(review.local_id).bind(&review.remote).bind(CLIENT_ID).bind(scope.github_id)
        .bind(scope.installation_id).bind(scope.repository_id).bind(&review.fingerprint)
        .bind(github_accounts::random_id()?).bind(review.local_id).execute(&mut *tx).await?;
    if result.rows_affected() != 1 {
        return Err(AccessError::LocalRepositoryUnavailable.into());
    }
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn unlink_remote(
    pool: &SqlitePool,
    local_id: i64,
    remote: &str,
    version: &str,
) -> Result<(), ResourceError> {
    let result = sqlx::query(
        "DELETE FROM github_remote_links WHERE local_repository_id = ? AND remote = ? AND version = ?",
    ).bind(local_id).bind(remote).bind(version).execute(pool).await?;
    if result.rows_affected() != 1 {
        return Err(AccessError::LinkChanged.into());
    }
    Ok(())
}

async fn local_repository(
    pool: &SqlitePool,
    local_id: i64,
) -> Result<LocalRepository, ResourceError> {
    inventory::repository(pool, local_id)
        .await
        .map_err(|_| AccessError::LocalRepositoryUnavailable.into())
}

fn valid_remote(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"./-_".contains(&byte))
}

async fn snapshot(exe: &Path, cwd: &Path, git_dir: &str, remote: &str) -> Option<Snapshot> {
    let (status, output) = inspector::run(exe, cwd, &["remote", "get-url", "--all", "--", remote])
        .await
        .ok()?;
    status.success().then_some(())?;
    let value = std::str::from_utf8(&output).ok()?.strip_suffix('\n')?;
    let value = value.strip_suffix('\r').unwrap_or(value);
    let destination = github_destination(value)?;
    let mut hash = Sha256::new();
    hash.update(git_dir);
    hash.update([0]);
    hash.update(value);
    Some(Snapshot {
        destination,
        fingerprint: hex::encode(hash.finalize()),
    })
}

fn github_destination(value: &str) -> Option<String> {
    let kind = if value.starts_with("https://") {
        ProbeKind::Https
    } else {
        ProbeKind::Ssh
    };
    let (url, host) = inspector_probe::remote_url(value, kind)?;
    if host != "github.com" {
        return None;
    }
    let path = match url.strip_prefix("git@github.com:") {
        Some(path) => path,
        None => url.split_once("://")?.1.split_once('/')?.1,
    };
    let name = path.strip_suffix(".git").unwrap_or(path);
    valid_full_name(name).then(|| name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        infrastructure::{discovery, git, storage},
        repository::DiscoveredRepository,
    };

    #[test]
    fn accepts_only_direct_github_fetch_destinations() {
        for url in [
            "https://github.com/owner/repo.git",
            "git@github.com:owner/repo.git",
            "ssh://git@github.com/owner/repo",
        ] {
            assert_eq!(github_destination(url).as_deref(), Some("owner/repo"));
        }
        for url in [
            "https://user:secret@github.com/owner/repo.git",
            "https://github.com/owner/repo.git?secret=token",
            "https://github.com/owner/../repo.git",
            "git@github-alias:owner/repo.git",
            "git@github.com:owner/repo.git\ngit@github.com:owner/another",
        ] {
            assert!(github_destination(url).is_none());
        }
    }

    #[test]
    fn reads_effective_git_destinations_and_flags_changes_to_saved_links() {
        tauri::async_runtime::block_on(async {
            let fixture = tempfile::tempdir().unwrap();
            let path = discovery::directory_path(fixture.path()).await.unwrap();
            let cwd = Path::new(&path);
            let exe = git::executable().unwrap();
            for args in [
                &["init", "--quiet"][..],
                &["remote", "add", "origin", "git@github.com:owner/repo.git"],
            ] {
                assert!(inspector::run(&exe, cwd, args).await.unwrap().0.success());
            }
            let git_dir = inspector::line(&exe, cwd, &["rev-parse", "--absolute-git-dir"])
                .await
                .unwrap();
            let git_dir = discovery::directory_path(Path::new(&git_dir))
                .await
                .unwrap();
            let pool = storage::open(&cwd.join("app-data")).await.unwrap();
            let directory = inventory::add(&pool, &path).await.unwrap();
            let directory = inventory::directory(&pool, directory).await.unwrap();
            inventory::save_scan(
                &pool,
                &directory,
                vec![DiscoveredRepository {
                    path: path.clone(),
                    kind: "ordinary",
                    git_dir: git_dir.clone(),
                    common_dir: git_dir,
                }],
                false,
            )
            .await
            .unwrap();
            let id = inventory::contents(&pool, directory.id)
                .await
                .unwrap()
                .repositories[0]
                .id;
            sqlx::query("INSERT INTO github_credentials (reference, state, client_id, github_id) VALUES ('fixture', 'active', ?, 42)")
                .bind(CLIENT_ID).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO github_accounts (client_id, github_id, login, expires_at, credential_ref, verified_at) VALUES (?, 42, 'owner', 123, 'fixture', 100)")
                .bind(CLIENT_ID).execute(&pool).await.unwrap();
            let scope = RepositoryScope {
                github_id: 42,
                installation_id: 7,
                repository_id: 10,
                repository_page: 1,
            };
            let remotes = candidates(&pool, id, Some(scope)).await.unwrap().remotes;
            assert_eq!(remotes[0].state, "available");
            let review = RemoteReview {
                local_id: id,
                remote: "origin".into(),
                fingerprint: remotes[0].fingerprint.clone().unwrap(),
                version: None,
            };
            save_link(&pool, scope, &review).await.unwrap();
            assert_eq!(
                save_link(&pool, scope, &review).await,
                Err(AccessError::LinkChanged.into())
            );
            let linked = candidates(&pool, id, Some(scope)).await.unwrap().remotes;
            assert_eq!(linked[0].state, "linked");
            assert!(inspector::run(
                &exe,
                cwd,
                &[
                    "remote",
                    "set-url",
                    "origin",
                    "https://github.com/owner/changed.git"
                ]
            )
            .await
            .unwrap()
            .0
            .success());
            let changed = candidates(&pool, id, Some(scope)).await.unwrap().remotes;
            assert_eq!(changed[0].state, "review");
            assert_ne!(linked[0].fingerprint, changed[0].fingerprint);
            assert!(inspector::run(
                &exe,
                cwd,
                &[
                    "config",
                    "--add",
                    "remote.origin.url",
                    "https://github.com/owner/other.git"
                ]
            )
            .await
            .unwrap()
            .0
            .success());
            let multiple = candidates(&pool, id, Some(scope)).await.unwrap().remotes;
            assert_eq!(multiple[0].state, "review");
            assert!(multiple[0].destination.is_none());
            assert_eq!(
                unlink_remote(&pool, id, "origin", "stale-version").await,
                Err(AccessError::LinkChanged.into())
            );
            unlink_remote(&pool, id, "origin", changed[0].version.as_deref().unwrap())
                .await
                .unwrap();
            assert!(candidates(&pool, id, Some(scope))
                .await
                .unwrap()
                .remotes
                .is_empty());
        });
    }
}
