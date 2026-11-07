use std::path::Path;

use reqwest::header::{HeaderMap, LINK, RETRY_AFTER};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{decode, github_accounts, response_bytes, valid_login, Accounts, Github, GithubError};

const PER_PAGE: usize = 20;
const MAX_PAGE: u32 = 100_000;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub(crate) enum ResourceError {
    Account(GithubError),
    Access(AccessError),
    Limited {
        #[serde(rename = "retryAt")]
        retry_at: i64,
    },
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AccessError {
    PermissionDenied,
    SsoRequired,
    InstallationUnavailable,
}

impl From<GithubError> for ResourceError {
    fn from(error: GithubError) -> Self {
        Self::Account(error)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourcePage<T> {
    items: Vec<T>,
    has_next: bool,
    partial_sso: bool,
}

#[derive(Serialize)]
pub(crate) struct Installation {
    id: i64,
    login: String,
    kind: OwnerKind,
    selection: Selection,
    suspended: bool,
}

#[derive(Deserialize)]
struct Owner {
    login: String,
}

#[derive(Deserialize, Serialize)]
enum OwnerKind {
    User,
    Organization,
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Selection {
    All,
    Selected,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
struct RemoteInstallation {
    id: i64,
    account: Option<Owner>,
    target_type: OwnerKind,
    repository_selection: Selection,
    suspended_at: Option<String>,
}

#[derive(Deserialize)]
struct Installations {
    installations: Vec<RemoteInstallation>,
}

#[derive(Deserialize)]
struct RemoteRepository {
    id: i64,
    full_name: String,
    private: bool,
    visibility: Option<String>,
    archived: bool,
}

#[derive(Deserialize)]
struct Repositories {
    repositories: Vec<RemoteRepository>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Repository {
    id: i64,
    full_name: String,
    visibility: &'static str,
    archived: bool,
}

#[derive(Deserialize)]
struct ApiProblem {
    message: String,
}

impl Github {
    pub(crate) async fn installations(
        &self,
        pool: &SqlitePool,
        app: &Path,
        github_id: i64,
        page: u32,
    ) -> Result<ResourcePage<Installation>, ResourceError> {
        let (data, has_next, partial_sso): (Installations, _, _) = self
            .resource_page(pool, app, github_id, "/user/installations", page)
            .await?;
        let mut items = Vec::new();
        for remote in data.installations {
            let Some(owner) = remote.account else {
                continue;
            };
            if !valid_id(remote.id) || !valid_login(&owner.login) {
                return Err(GithubError::Protocol.into());
            }
            items.push(Installation {
                id: remote.id,
                login: owner.login,
                kind: remote.target_type,
                selection: remote.repository_selection,
                suspended: remote.suspended_at.is_some(),
            });
        }
        resource_page(items, has_next, partial_sso)
    }

    pub(crate) async fn repositories(
        &self,
        pool: &SqlitePool,
        app: &Path,
        github_id: i64,
        installation_id: i64,
        page: u32,
    ) -> Result<ResourcePage<Repository>, ResourceError> {
        if !valid_id(installation_id) {
            return Err(GithubError::Protocol.into());
        }
        let path = format!("/user/installations/{installation_id}/repositories");
        let (data, has_next, partial_sso): (Repositories, _, _) = self
            .resource_page(pool, app, github_id, &path, page)
            .await?;
        let items = data
            .repositories
            .into_iter()
            .map(repository)
            .collect::<Result<_, _>>()?;
        resource_page(items, has_next, partial_sso)
    }

    async fn resource_page<T: DeserializeOwned>(
        &self,
        pool: &SqlitePool,
        app: &Path,
        github_id: i64,
        path: &str,
        page: u32,
    ) -> Result<(T, bool, bool), ResourceError> {
        if !valid_id(github_id) || !(1..=MAX_PAGE).contains(&page) {
            return Err(GithubError::Protocol.into());
        }
        let mut limits = self.resource_limits.lock().await;
        let now = github_accounts::now()?;
        limits.retain(|_, retry_at| *retry_at > now);
        if let Some(retry_at) = limits.get(&github_id) {
            return Err(ResourceError::Limited {
                retry_at: *retry_at,
            });
        }
        let store = Accounts::lock(pool, app)?;
        let credential = self.account_credential(&store, github_id, false).await?;
        let mut response = self
            .api_get(path, &credential.access_token)?
            .query(&[("per_page", PER_PAGE as u32), ("page", page)])
            .send()
            .await
            .map_err(|_| GithubError::Network)?;
        let status = response.status().as_u16();
        if status == 401 {
            store.require_authorization(github_id).await?;
            return Err(GithubError::NotConnected.into());
        }
        if !response.status().is_success() {
            let secondary = status == 403
                && matches!(
                    response_bytes(&mut response, 8_192)
                        .await
                        .and_then(|bytes| decode::<ApiProblem>(&bytes)),
                    Ok(problem) if problem.message.to_ascii_lowercase().contains("secondary rate limit")
                );
            let now = github_accounts::now()?;
            let error = response_error(status, response.headers(), now, secondary);
            if let ResourceError::Limited { retry_at } = error {
                limits.insert(github_id, retry_at);
            }
            return Err(error);
        }
        let has_next = page < MAX_PAGE && next_page(response.headers());
        let partial_sso = sso(response.headers()) == Some("partial-results");
        let bytes = response_bytes(&mut response, 1_048_576).await?;
        Ok((decode(&bytes)?, has_next, partial_sso))
    }
}

fn resource_page<T>(
    items: Vec<T>,
    has_next: bool,
    partial_sso: bool,
) -> Result<ResourcePage<T>, ResourceError> {
    if items.len() > PER_PAGE {
        return Err(GithubError::Protocol.into());
    }
    Ok(ResourcePage {
        items,
        has_next,
        partial_sso,
    })
}

fn repository(remote: RemoteRepository) -> Result<Repository, GithubError> {
    let valid_name = remote
        .full_name
        .split_once('/')
        .is_some_and(|(owner, name)| {
            valid_login(owner)
                && !name.is_empty()
                && name.len() <= 100
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        });
    if !valid_id(remote.id) || !valid_name {
        return Err(GithubError::Protocol);
    }
    Ok(Repository {
        id: remote.id,
        full_name: remote.full_name,
        visibility: match (remote.visibility.as_deref(), remote.private) {
            (Some("internal"), _) => "internal",
            (Some("private"), _) | (_, true) => "private",
            _ => "public",
        },
        archived: remote.archived,
    })
}

fn valid_id(id: i64) -> bool {
    (1..=9_007_199_254_740_991).contains(&id)
}

fn sso(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-github-sso")?
        .to_str()
        .ok()?
        .split(';')
        .next()
        .map(str::trim)
}

fn next_page(headers: &HeaderMap) -> bool {
    // The URL in Link is never followed. The next call rebuilds the known path and query.
    headers
        .get_all(LINK)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .any(|line| {
            line.split(',')
                .filter_map(|link| link.rsplit_once('>'))
                .any(|(_, attributes)| {
                    attributes
                        .split(';')
                        .any(|attribute| attribute.trim() == "rel=\"next\"")
                })
        })
}

fn response_error(status: u16, headers: &HeaderMap, now: i64, secondary: bool) -> ResourceError {
    let primary = headers
        .get("x-ratelimit-remaining")
        .is_some_and(|value| value == "0");
    let retry_after = headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|seconds| *seconds > 0);
    if status == 429 || status == 403 && (primary || secondary || retry_after.is_some()) {
        let reset = primary
            .then(|| {
                headers
                    .get("x-ratelimit-reset")?
                    .to_str()
                    .ok()?
                    .parse::<i64>()
                    .ok()
            })
            .flatten()
            .filter(|reset| *reset > now);
        let retry_at = retry_after
            .map(|seconds| now.saturating_add(seconds))
            .into_iter()
            .chain(reset)
            .max()
            .unwrap_or(now + 60);
        return ResourceError::Limited { retry_at };
    }
    match status {
        403 | 404 if sso(headers) == Some("required") => {
            ResourceError::Access(AccessError::SsoRequired)
        }
        403 => ResourceError::Access(AccessError::PermissionDenied),
        404 => ResourceError::Access(AccessError::InstallationUnavailable),
        500..=599 => GithubError::Network.into(),
        _ => GithubError::Protocol.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn distinguishes_sso_permission_and_missing_installation() {
        let mut headers = HeaderMap::new();
        assert_eq!(
            response_error(403, &headers, 100, false),
            ResourceError::Access(AccessError::PermissionDenied),
        );
        assert_eq!(
            response_error(404, &headers, 100, false),
            ResourceError::Access(AccessError::InstallationUnavailable),
        );
        headers.insert(
            "x-github-sso",
            HeaderValue::from_static("required; url=https://example.test/secret"),
        );
        let error = response_error(403, &headers, 100, false);
        assert_eq!(error, ResourceError::Access(AccessError::SsoRequired));
        assert_eq!(serde_json::to_value(error).unwrap(), "sso_required");
    }

    #[test]
    fn respects_the_latest_rate_limit_and_waits_without_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("30"));
        headers.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));
        headers.insert("x-ratelimit-reset", HeaderValue::from_static("500"));
        assert_eq!(
            response_error(403, &headers, 100, false),
            ResourceError::Limited { retry_at: 500 },
        );
        assert_eq!(
            response_error(403, &HeaderMap::new(), 100, true),
            ResourceError::Limited { retry_at: 160 },
        );
    }
}
