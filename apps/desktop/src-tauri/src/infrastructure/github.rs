use std::{collections::HashMap, path::Path, time::Duration};

use reqwest::{header, Client, RequestBuilder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::{
    sync::{watch, Mutex},
    time::Instant,
};
use zeroize::Zeroizing;

pub(crate) use super::github_accounts::AccountStatus;
use super::github_accounts::{self, Accounts, Credential, ExpectedAccount};

#[path = "github_links.rs"]
mod links;
#[path = "github_resources.rs"]
mod resources;
pub(crate) use links::{candidates, unlink_remote, RemoteCandidates, RemoteReview};
pub(crate) use resources::{
    ActivityItem, ActivityKind, Installation, Repository, RepositoryScope, ResourceError,
    ResourcePage,
};

pub(crate) const CLIENT_ID: &str = "Iv23libwtluOIgGw4BuU";
pub(crate) const VERIFICATION_URI: &str = "https://github.com/login/device";
pub(crate) const AUTHORIZATIONS_URI: &str = "https://github.com/settings/apps/authorizations";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GithubError {
    Busy,
    Stale,
    Unavailable,
    Network,
    Protocol,
    InvalidApp,
    Denied,
    Expired,
    Cancelled,
    SecureStore,
    Storage,
    DifferentAccount,
    NotConnected,
    RateLimited,
    RenewalUncertain,
}

impl From<sqlx::Error> for GithubError {
    fn from(_: sqlx::Error) -> Self {
        Self::Storage
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlowStatus {
    session_id: String,
    stage: &'static str,
    user_code: Option<String>,
    expires_in: u64,
    retry_after_ms: u64,
    github_id: Option<i64>,
}

#[derive(Deserialize)]
struct Device {
    device_code: Zeroizing<String>,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct Tokens {
    access_token: Zeroizing<String>,
    refresh_token: Zeroizing<String>,
    token_type: String,
    expires_in: u64,
    refresh_token_expires_in: u64,
    scope: Option<String>,
}

#[derive(Deserialize)]
struct RemoteUser {
    id: i64,
    login: String,
}

#[derive(Deserialize)]
struct OAuthError {
    error: String,
}

struct Session {
    id: String,
    device: Option<Device>,
    deadline: Instant,
    next_poll: Instant,
    interval: Duration,
    in_flight: bool,
    cancel: watch::Sender<bool>,
    expected: Option<ExpectedAccount>,
}

impl Session {
    fn status(&self) -> FlowStatus {
        let now = Instant::now();
        FlowStatus {
            session_id: self.id.clone(),
            stage: if self.device.is_none() {
                "starting"
            } else if self.in_flight {
                "verifying"
            } else {
                "pending"
            },
            user_code: self.device.as_ref().map(|device| device.user_code.clone()),
            expires_in: self.deadline.saturating_duration_since(now).as_secs(),
            retry_after_ms: self
                .next_poll
                .saturating_duration_since(now)
                .as_millis()
                .max(500) as u64,
            github_id: None,
        }
    }
}

pub(crate) struct Github {
    client: Option<Client>,
    session: Mutex<Option<Session>>,
    oauth_base: String,
    api_base: String,
    resource_limits: Mutex<HashMap<i64, i64>>,
}

enum Work {
    Device,
    Token(Zeroizing<String>, Option<ExpectedAccount>),
}
enum Outcome {
    Device(Device),
    Waiting(bool),
    Connected(i64),
}

impl Github {
    pub(crate) fn new() -> Self {
        let client = Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .user_agent(concat!("GitPilot/", env!("CARGO_PKG_VERSION")))
            .build()
            .ok();
        Self {
            client,
            session: Mutex::new(None),
            oauth_base: "https://github.com".into(),
            api_base: "https://api.github.com".into(),
            resource_limits: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) async fn current(&self) -> Option<FlowStatus> {
        let mut state = self.session.lock().await;
        if state
            .as_ref()
            .is_some_and(|session| Instant::now() >= session.deadline)
        {
            if let Some(session) = state.take() {
                let _ = session.cancel.send(true);
            }
        }
        state.as_ref().map(Session::status)
    }

    pub(crate) async fn begin(
        &self,
        pool: &SqlitePool,
        app: &Path,
        id: Option<i64>,
    ) -> Result<FlowStatus, GithubError> {
        let expected = if let Some(id) = id {
            let account = Accounts::lock(pool, app)?
                .row(id)
                .await?
                .ok_or(GithubError::NotConnected)?;
            if account.state == "disconnecting" {
                return Err(GithubError::Busy);
            }
            Some(ExpectedAccount {
                id,
                reference: account.credential_ref,
            })
        } else {
            None
        };
        self.start(expected).await
    }

    async fn start(&self, expected: Option<ExpectedAccount>) -> Result<FlowStatus, GithubError> {
        if self.client.is_none() {
            return Err(GithubError::Unavailable);
        }
        self.current().await;
        let mut state = self.session.lock().await;
        if state.is_some() {
            return Err(GithubError::Busy);
        }
        let now = Instant::now();
        let (cancel, _) = watch::channel(false);
        let session = Session {
            id: github_accounts::random_id()?,
            device: None,
            deadline: now + Duration::from_secs(900),
            next_poll: now,
            interval: Duration::from_secs(5),
            in_flight: false,
            cancel,
            expected,
        };
        let status = session.status();
        *state = Some(session);
        Ok(status)
    }

    pub(crate) async fn cancel(&self, id: &str) -> Result<(), GithubError> {
        let mut state = self.session.lock().await;
        if state.as_ref().is_some_and(|session| session.id != id) {
            return Err(GithubError::Stale);
        }
        if let Some(session) = state.take() {
            let _ = session.cancel.send(true);
        }
        Ok(())
    }

    pub(crate) async fn can_open(&self, id: &str) -> bool {
        self.session.lock().await.as_ref().is_some_and(|session| {
            session.id == id && session.device.is_some() && Instant::now() < session.deadline
        })
    }

    pub(crate) async fn poll(
        &self,
        pool: &SqlitePool,
        app: &Path,
        id: &str,
    ) -> Result<FlowStatus, GithubError> {
        let (work, mut cancelled, deadline) = {
            let mut state = self.session.lock().await;
            let session = state
                .as_mut()
                .filter(|session| session.id == id)
                .ok_or(GithubError::Stale)?;
            if Instant::now() >= session.deadline {
                *state = None;
                return Err(GithubError::Expired);
            }
            if session.in_flight || Instant::now() < session.next_poll {
                return Ok(session.status());
            }
            session.in_flight = true;
            let work = session.device.as_ref().map_or(Work::Device, |device| {
                Work::Token(device.device_code.clone(), session.expected.clone())
            });
            (work, session.cancel.subscribe(), session.deadline)
        };
        let outcome = tokio::select! {
            biased;
            _ = cancelled.changed() => Err(GithubError::Cancelled),
            _ = tokio::time::sleep_until(deadline) => Err(GithubError::Expired),
            result = self.perform(pool, app, work) => result,
        };
        let mut state = self.session.lock().await;
        let session = state
            .as_mut()
            .filter(|session| session.id == id)
            .ok_or(GithubError::Cancelled)?;
        session.in_flight = false;
        match outcome {
            Ok(Outcome::Device(device)) => {
                session.deadline =
                    Instant::now() + Duration::from_secs(device.expires_in.saturating_sub(15));
                session.interval = Duration::from_secs(device.interval);
                session.next_poll = Instant::now() + session.interval;
                session.device = Some(device);
                Ok(session.status())
            }
            Ok(Outcome::Waiting(slow)) => {
                if slow {
                    session.interval += Duration::from_secs(5);
                }
                session.next_poll = Instant::now() + session.interval;
                Ok(session.status())
            }
            Ok(Outcome::Connected(id_from_github)) => {
                *state = None;
                Ok(FlowStatus {
                    session_id: id.into(),
                    stage: "connected",
                    user_code: None,
                    expires_in: 0,
                    retry_after_ms: 0,
                    github_id: Some(id_from_github),
                })
            }
            Err(GithubError::Network) => {
                session.interval = (session.interval * 2).min(Duration::from_secs(120));
                session.next_poll = Instant::now() + session.interval;
                Ok(session.status())
            }
            Err(error) => {
                *state = None;
                Err(error)
            }
        }
    }

    async fn perform(
        &self,
        pool: &SqlitePool,
        app: &Path,
        work: Work,
    ) -> Result<Outcome, GithubError> {
        let client = self.client.as_ref().ok_or(GithubError::Unavailable)?;
        match work {
            Work::Device => {
                let bytes = response(
                    client
                        .post(format!("{}/login/device/code", self.oauth_base))
                        .header(header::ACCEPT, "application/json")
                        .form(&[("client_id", CLIENT_ID), ("scope", "offline_access")]),
                )
                .await?;
                reject_oauth_error(&bytes)?;
                let device: Device = decode(&bytes)?;
                if device.verification_uri != VERIFICATION_URI
                    || device.device_code.len() != 40
                    || !device
                        .device_code
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric())
                    || device.user_code.len() != 9
                    || device.user_code.as_bytes()[4] != b'-'
                    || !device.user_code.bytes().enumerate().all(|(index, byte)| {
                        index == 4 || byte.is_ascii_uppercase() || byte.is_ascii_digit()
                    })
                    || !(30..=900).contains(&device.expires_in)
                    || !(5..=60).contains(&device.interval)
                {
                    return Err(GithubError::Protocol);
                }
                Ok(Outcome::Device(device))
            }
            Work::Token(code, expected) => {
                let received_at = github_accounts::now()?;
                let bytes = response(
                    client
                        .post(format!("{}/login/oauth/access_token", self.oauth_base))
                        .header(header::ACCEPT, "application/json")
                        .form(&[
                            ("client_id", CLIENT_ID),
                            ("device_code", code.as_str()),
                            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                        ]),
                )
                .await?;
                if let Ok(error) = serde_json::from_slice::<OAuthError>(&bytes) {
                    return match error.error.as_str() {
                        "authorization_pending" => Ok(Outcome::Waiting(false)),
                        "slow_down" => Ok(Outcome::Waiting(true)),
                        _ => Err(oauth_error(&error.error)),
                    };
                }
                let tokens = parse_tokens(&bytes)?;
                let user = self.user(&tokens.access_token).await.map_err(|error| {
                    if error == GithubError::Network {
                        GithubError::Unavailable
                    } else {
                        error
                    }
                })?;
                let credential = tokens.credential(user.id, received_at);
                Accounts::lock(pool, app)?
                    .save(&user.login, &credential, expected.as_ref())
                    .await?;
                Ok(Outcome::Connected(user.id))
            }
        }
    }

    fn api_get(&self, path: &str, token: &str) -> Result<RequestBuilder, GithubError> {
        let mut authorization = header::HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| GithubError::Protocol)?;
        authorization.set_sensitive(true);
        Ok(self
            .client
            .as_ref()
            .ok_or(GithubError::Unavailable)?
            .get(format!("{}{path}", self.api_base))
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2026-03-10")
            .header(header::AUTHORIZATION, authorization))
    }

    async fn user(&self, token: &str) -> Result<RemoteUser, GithubError> {
        let bytes = response(self.api_get("/user", token)?).await?;
        let user: RemoteUser = decode(&bytes)?;
        if user.id <= 0 || !valid_login(&user.login) {
            return Err(GithubError::Protocol);
        }
        Ok(user)
    }

    pub(crate) async fn check(
        &self,
        pool: &SqlitePool,
        app: &Path,
        id: i64,
    ) -> Result<AccountStatus, GithubError> {
        let store = Accounts::lock(pool, app)?;
        self.account_credential(&store, id, true).await?;
        store.status().await
    }

    async fn account_credential(
        &self,
        store: &Accounts<'_>,
        id: i64,
        verify: bool,
    ) -> Result<Credential, GithubError> {
        let account = store.row(id).await?.ok_or(GithubError::NotConnected)?;
        let time = github_accounts::now()?;
        let (credential, rotation) = match account.state.as_str() {
            "active" => {
                let current = store.read(&account, &account.credential_ref).await?;
                if current.expires_at > time + 60 {
                    (current, None)
                } else {
                    if current.refresh_expires_at <= time {
                        store.require_authorization(id).await?;
                        return Err(GithubError::Expired);
                    }
                    let reference = store.prepare_refresh(&account).await?;
                    let credential = match self.refresh(&current, time).await {
                        Ok(credential) => credential,
                        Err(error) => {
                            store.require_authorization(id).await?;
                            return Err(match error {
                                GithubError::Expired => GithubError::Expired,
                                _ => GithubError::RenewalUncertain,
                            });
                        }
                    };
                    store.write(&reference, &credential).await?;
                    (credential, Some(reference))
                }
            }
            "refreshing" => {
                // A pending rotation can use its staged result, never its old refresh token.
                let reference = account.refresh_ref.clone().ok_or(GithubError::Storage)?;
                let credential = store
                    .read(&account, &reference)
                    .await
                    .map_err(|_| GithubError::RenewalUncertain)?;
                if credential.expires_at <= time {
                    store.require_authorization(id).await?;
                    return Err(GithubError::Expired);
                }
                (credential, Some(reference))
            }
            "reauthorize" => return Err(GithubError::Expired),
            _ => return Err(GithubError::Busy),
        };
        if rotation.is_none() && !verify {
            return Ok(credential);
        }
        let user = match self.user(&credential.access_token).await {
            Ok(user) => user,
            Err(GithubError::NotConnected) => {
                store.require_authorization(id).await?;
                return Err(GithubError::NotConnected);
            }
            Err(error) => return Err(error),
        };
        if account.github_id != user.id {
            store.require_authorization(id).await?;
            return Err(GithubError::DifferentAccount);
        }
        if let Some(reference) = rotation {
            store.activate(&reference, &user.login, &credential).await?;
        } else {
            store.mark_verified(&account, &user.login).await?;
        }
        Ok(credential)
    }

    async fn refresh(&self, current: &Credential, time: i64) -> Result<Credential, GithubError> {
        let bytes = response(
            self.client
                .as_ref()
                .ok_or(GithubError::Unavailable)?
                .post(format!("{}/login/oauth/access_token", self.oauth_base))
                .header(header::ACCEPT, "application/json")
                .form(&[
                    ("client_id", CLIENT_ID),
                    ("grant_type", "refresh_token"),
                    ("refresh_token", current.refresh_token.as_str()),
                ]),
        )
        .await?;
        reject_oauth_error(&bytes)?;
        Ok(parse_tokens(&bytes)?.credential(current.github_id, time))
    }

    pub(crate) async fn disconnect(
        &self,
        pool: &SqlitePool,
        app: &Path,
        id: i64,
    ) -> Result<AccountStatus, GithubError> {
        {
            let mut state = self.session.lock().await;
            if state
                .as_ref()
                .and_then(|session| session.expected.as_ref())
                .is_some_and(|expected| expected.id == id)
            {
                if let Some(session) = state.take() {
                    let _ = session.cancel.send(true);
                }
            }
        }
        Accounts::lock(pool, app)?.disconnect(id).await
    }

    pub(crate) async fn account(
        &self,
        pool: &SqlitePool,
        app: &Path,
    ) -> Result<AccountStatus, GithubError> {
        Accounts::lock(pool, app)?.status().await
    }

    pub(crate) async fn acknowledge_revocation(
        &self,
        pool: &SqlitePool,
        app: &Path,
        id: i64,
    ) -> Result<AccountStatus, GithubError> {
        Accounts::lock(pool, app)?.acknowledge_revocation(id).await
    }
}

fn valid_login(login: &str) -> bool {
    !login.is_empty()
        && login.len() <= 39
        && login
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

impl Tokens {
    fn credential(self, github_id: i64, time: i64) -> Credential {
        Credential {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            github_id,
            expires_at: time + self.expires_in as i64,
            refresh_expires_at: time + self.refresh_token_expires_in as i64,
            client_id: CLIENT_ID.into(),
        }
    }
}

fn parse_tokens(bytes: &[u8]) -> Result<Tokens, GithubError> {
    let tokens: Tokens = decode(bytes)?;
    if tokens.token_type != "bearer"
        || !valid_token(&tokens.access_token, "ghu_")
        || !valid_token(&tokens.refresh_token, "ghr_")
        || tokens.scope.as_ref().is_some_and(|scope| !scope.is_empty())
        || !(60..=86_400).contains(&tokens.expires_in)
        || !(60..=31_622_400).contains(&tokens.refresh_token_expires_in)
    {
        return Err(GithubError::Protocol);
    }
    Ok(tokens)
}

fn valid_token(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix)
        && (20..=1024).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, GithubError> {
    serde_json::from_slice(bytes).map_err(|_| GithubError::Protocol)
}

fn reject_oauth_error(bytes: &[u8]) -> Result<(), GithubError> {
    if let Ok(error) = serde_json::from_slice::<OAuthError>(bytes) {
        return Err(oauth_error(&error.error));
    }
    Ok(())
}

fn oauth_error(value: &str) -> GithubError {
    match value {
        "access_denied" => GithubError::Denied,
        "expired_token" | "bad_verification_code" | "bad_refresh_token" => GithubError::Expired,
        "incorrect_client_credentials" | "device_flow_disabled" | "unauthorized_client" => {
            GithubError::InvalidApp
        }
        _ => GithubError::Protocol,
    }
}

async fn response(request: RequestBuilder) -> Result<Zeroizing<Vec<u8>>, GithubError> {
    let mut response = request.send().await.map_err(|_| GithubError::Network)?;
    if response.status().as_u16() == 401 {
        return Err(GithubError::NotConnected);
    }
    if response.status().as_u16() == 429
        || response.status().as_u16() == 403
            && response
                .headers()
                .get("x-ratelimit-remaining")
                .is_some_and(|value| value == "0")
    {
        return Err(GithubError::RateLimited);
    }
    if response.status().is_server_error() {
        return Err(GithubError::Network);
    }
    let status = response.status();
    let bytes = response_bytes(&mut response, 32_768).await?;
    if !status.is_success() {
        reject_oauth_error(&bytes)?;
        return Err(GithubError::Protocol);
    }
    Ok(bytes)
}

async fn response_bytes(
    response: &mut reqwest::Response,
    limit: usize,
) -> Result<Zeroizing<Vec<u8>>, GithubError> {
    let mut bytes = Zeroizing::new(Vec::new());
    while let Some(chunk) = response.chunk().await.map_err(|_| GithubError::Network)? {
        if chunk.len() > limit - bytes.len() {
            return Err(GithubError::Protocol);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    const DEVICE: &str = r#"{"device_code":"0123456789012345678901234567890123456789","user_code":"ABCD-1234","verification_uri":"https://github.com/login/device","expires_in":900,"interval":5}"#;
    const TOKENS: &str = r#"{"access_token":"ghu_fixture_access_0123456789","refresh_token":"ghr_fixture_refresh_0123456789","token_type":"bearer","expires_in":28800,"refresh_token_expires_in":15897600,"scope":""}"#;

    struct Server {
        url: String,
        calls: Arc<AtomicUsize>,
        authorizations: Arc<std::sync::Mutex<Vec<String>>>,
        worker: std::thread::JoinHandle<()>,
    }

    fn server(replies: Vec<(&'static str, String)>, delay: Duration) -> Server {
        server_with_status(
            replies
                .into_iter()
                .map(|(path, body)| (path, 200, "", body))
                .collect(),
            delay,
        )
    }

    fn server_with_status(
        replies: Vec<(&'static str, u16, &'static str, String)>,
        delay: Duration,
    ) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let authorizations = Arc::new(std::sync::Mutex::new(Vec::new()));
        let received_auth = authorizations.clone();
        let worker = std::thread::spawn(move || {
            for (path, status, headers, body) in replies {
                let deadline = std::time::Instant::now() + Duration::from_secs(60);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error)
                            if error.kind() == std::io::ErrorKind::WouldBlock
                                && std::time::Instant::now() < deadline =>
                        {
                            std::thread::sleep(Duration::from_millis(5))
                        }
                        Err(error) => {
                            panic!("Fixture did not receive the expected request: {error}")
                        }
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                loop {
                    let mut bytes = [0u8; 1024];
                    let length = stream.read(&mut bytes).unwrap();
                    request.extend_from_slice(&bytes[..length]);
                    if length == 0 || request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                        break;
                    }
                }
                let text = String::from_utf8_lossy(&request);
                assert!(text.lines().next().unwrap().contains(path));
                if let Some(value) = text.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("authorization")
                        .then(|| value.trim())
                }) {
                    received_auth.lock().unwrap().push(value.to_owned());
                }
                let header_end = request
                    .windows(4)
                    .position(|bytes| bytes == b"\r\n\r\n")
                    .unwrap()
                    + 4;
                let body_length = text
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while request.len() < header_end + body_length {
                    let mut bytes = [0; 1024];
                    let read = stream.read(&mut bytes).unwrap();
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&bytes[..read]);
                }
                observed.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(delay);
                let reply = format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}", body.len());
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        Server {
            url,
            calls,
            authorizations,
            worker,
        }
    }

    fn fixture(server: &Server) -> Github {
        Github {
            client: Some(
                Client::builder()
                    .no_proxy()
                    .redirect(reqwest::redirect::Policy::none())
                    .tls_built_in_native_certs(false)
                    .timeout(Duration::from_secs(10))
                    .build()
                    .unwrap(),
            ),
            session: Mutex::new(None),
            oauth_base: server.url.clone(),
            api_base: server.url.clone(),
            resource_limits: Mutex::new(HashMap::new()),
        }
    }

    async fn due(service: &Github) {
        service.session.lock().await.as_mut().unwrap().next_poll = Instant::now();
    }

    #[test]
    fn migrates_the_first_account_and_keeps_its_credential_reference() {
        tauri::async_runtime::block_on(async {
            let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
            sqlx::raw_sql(include_str!(
                "../../migrations/202611070001_github_account.sql"
            ))
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO github_credentials (reference, state) VALUES ('legacy', 'active')",
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query("INSERT INTO github_accounts (slot, client_id, github_id, login, expires_at, credential_ref, verified_at) VALUES (1, ?, 42, 'first', 123, 'legacy', 100)")
                .bind(CLIENT_ID)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::raw_sql(include_str!(
                "../../migrations/202611070002_github_accounts.sql"
            ))
            .execute(&pool)
            .await
            .unwrap();
            let account: (String, i64, i64, String) = sqlx::query_as(
                "SELECT credential_ref, expires_at, verified_at, state FROM github_accounts WHERE github_id = 42",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(account, ("legacy".into(), 123, 100, "active".into()));
            let ownership: (String, i64) = sqlx::query_as(
                "SELECT client_id, github_id FROM github_credentials WHERE reference = 'legacy'",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(ownership, (CLIENT_ID.into(), 42));
        });
    }

    #[test]
    fn polling_respects_wait_slowdown_and_denial() {
        tauri::async_runtime::block_on(async {
            let server = server(
                vec![
                    ("/login/device/code", DEVICE.into()),
                    (
                        "/login/oauth/access_token",
                        r#"{"error":"authorization_pending"}"#.into(),
                    ),
                    (
                        "/login/oauth/access_token",
                        r#"{"error":"slow_down"}"#.into(),
                    ),
                    (
                        "/login/oauth/access_token",
                        r#"{"error":"access_denied"}"#.into(),
                    ),
                ],
                Duration::ZERO,
            );
            let service = fixture(&server);
            let directory = tempfile::tempdir().unwrap();
            let pool = super::super::storage::open(directory.path()).await.unwrap();
            let id = service.start(None).await.unwrap().session_id;
            assert!(matches!(service.start(None).await, Err(GithubError::Busy)));
            let pending = service.poll(&pool, directory.path(), &id).await.unwrap();
            assert_eq!(pending.user_code.as_deref(), Some("ABCD-1234"));
            let ipc = serde_json::to_string(&pending).unwrap();
            assert!(!ipc.contains("0123456789012345678901234567890123456789"));
            service.poll(&pool, directory.path(), &id).await.unwrap();
            assert_eq!(server.calls.load(Ordering::SeqCst), 1);
            due(&service).await;
            service.poll(&pool, directory.path(), &id).await.unwrap();
            due(&service).await;
            let slow = service.poll(&pool, directory.path(), &id).await.unwrap();
            assert!(slow.retry_after_ms >= 9_000);
            due(&service).await;
            assert!(matches!(
                service.poll(&pool, directory.path(), &id).await,
                Err(GithubError::Denied)
            ));
            assert!(service.current().await.is_none());
            let id = service.start(None).await.unwrap().session_id;
            service.session.lock().await.as_mut().unwrap().deadline = Instant::now();
            assert!(matches!(
                service.poll(&pool, directory.path(), &id).await,
                Err(GithubError::Expired)
            ));
            server.worker.join().unwrap();
        });
    }

    #[test]
    fn cancelling_an_inflight_request_discards_its_response() {
        tauri::async_runtime::block_on(async {
            let server = server(
                vec![("/login/device/code", DEVICE.into())],
                Duration::from_millis(250),
            );
            let service = Arc::new(fixture(&server));
            let directory = Arc::new(tempfile::tempdir().unwrap());
            let pool = super::super::storage::open(directory.path()).await.unwrap();
            let id = service.start(None).await.unwrap().session_id;
            let job = {
                let (service, pool, directory, id) =
                    (service.clone(), pool.clone(), directory.clone(), id.clone());
                tokio::spawn(async move { service.poll(&pool, directory.path(), &id).await })
            };
            tokio::time::timeout(Duration::from_secs(5), async {
                while server.calls.load(Ordering::SeqCst) == 0 {
                    assert!(
                        !job.is_finished(),
                        "Polling ended before the fixture received its request"
                    );
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .expect("The fixture did not receive a request before the deadline");
            service.cancel(&id).await.unwrap();
            assert!(matches!(job.await.unwrap(), Err(GithubError::Cancelled)));
            assert!(service.current().await.is_none());
            server.worker.join().unwrap();
        });
    }

    #[test]
    fn rejects_untrusted_verification_urls_and_large_responses() {
        tauri::async_runtime::block_on(async {
            let server = server(
                vec![
                    (
                        "/login/device/code",
                        DEVICE.replace(VERIFICATION_URI, "https://example.test/login"),
                    ),
                    ("/oversized", "x".repeat(32_769)),
                ],
                Duration::ZERO,
            );
            let service = fixture(&server);
            let directory = tempfile::tempdir().unwrap();
            let pool = super::super::storage::open(directory.path()).await.unwrap();
            let id = service.start(None).await.unwrap().session_id;
            assert!(matches!(
                service.poll(&pool, directory.path(), &id).await,
                Err(GithubError::Protocol)
            ));
            assert!(matches!(
                response(
                    service
                        .client
                        .as_ref()
                        .unwrap()
                        .get(format!("{}/oversized", server.url))
                )
                .await,
                Err(GithubError::Protocol)
            ));
            server.worker.join().unwrap();
        });
    }

    #[test]
    #[ignore = "requires an isolated OS credential store"]
    fn verifies_persists_and_recovers_an_account_with_the_os_vault() {
        tauri::async_runtime::block_on(async {
            let server = server(
                vec![
                    ("/login/device/code", DEVICE.into()),
                    ("/login/oauth/access_token", TOKENS.into()),
                    ("/user", r#"{"id":42,"login":"fixture"}"#.into()),
                    ("/user", r#"{"id":42,"login":"fixture-renamed"}"#.into()),
                ],
                Duration::ZERO,
            );
            let service = fixture(&server);
            let directory = tempfile::tempdir().unwrap();
            let pool = super::super::storage::open(directory.path()).await.unwrap();
            let id = service.start(None).await.unwrap().session_id;
            service.poll(&pool, directory.path(), &id).await.unwrap();
            due(&service).await;
            assert_eq!(
                service
                    .poll(&pool, directory.path(), &id)
                    .await
                    .unwrap()
                    .stage,
                "connected"
            );
            let status = service.account(&pool, directory.path()).await.unwrap();
            assert!(serde_json::to_string(&status)
                .unwrap()
                .contains("connected"));
            let reference: String =
                sqlx::query_scalar("SELECT credential_ref FROM github_accounts")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            pool.close().await;
            let pool = super::super::storage::open(directory.path()).await.unwrap();
            let checked = service.check(&pool, directory.path(), 42).await.unwrap();
            assert!(serde_json::to_string(&checked)
                .unwrap()
                .contains("fixture-renamed"));
            let vault = super::super::github_vault::Vault::new(directory.path());
            let orphan = github_accounts::random_id().unwrap();
            let empty = github_accounts::random_id().unwrap();
            sqlx::query("INSERT INTO github_credentials (reference, state) VALUES (?, 'pending')")
                .bind(&empty)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO github_credentials (reference, state) VALUES (?, 'pending')")
                .bind(&orphan)
                .execute(&pool)
                .await
                .unwrap();
            vault.write(&orphan, b"fixture-orphan").await.unwrap();
            service.account(&pool, directory.path()).await.unwrap();
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM github_credentials")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(count, 1);
            let database = std::fs::read(directory.path().join("gitpilot.sqlite3")).unwrap();
            assert!(!String::from_utf8_lossy(&database).contains("ghu_fixture"));
            vault.delete(&reference).await.unwrap();
            assert!(vault.read(&reference).await.is_err());
            pool.close().await;
            server.worker.join().unwrap();
        });
    }

    #[test]
    #[ignore = "requires an isolated OS credential store"]
    fn rotates_recovers_and_disconnects_only_the_selected_account() {
        tauri::async_runtime::block_on(async {
            let rotated = TOKENS.replace("fixture_", "rotated_");
            let server = server(
                vec![
                    ("/login/oauth/access_token", rotated),
                    ("/user", "{}".into()),
                    ("/user", r#"{"id":42,"login":"first-renamed"}"#.into()),
                    ("/login/oauth/access_token", "{}".into()),
                ],
                Duration::ZERO,
            );
            let service = fixture(&server);
            let directory = tempfile::tempdir().unwrap();
            let app = directory.path();
            let pool = super::super::storage::open(app).await.unwrap();
            let vault = super::super::github_vault::Vault::new(app);
            let (old_ref, other_ref) = {
                let store = Accounts::lock(&pool, app).unwrap();
                let expired = github_accounts::now().unwrap() - 29_000;
                for (id, login) in [(42, "first"), (84, "other")] {
                    let credential = parse_tokens(TOKENS.as_bytes())
                        .unwrap()
                        .credential(id, expired);
                    store.save(login, &credential, None).await.unwrap();
                }
                assert!(matches!(
                    service.check(&pool, app, 42).await,
                    Err(GithubError::Busy)
                ));
                (
                    store.row(42).await.unwrap().unwrap().credential_ref,
                    store.row(84).await.unwrap().unwrap().credential_ref,
                )
            };
            assert!(matches!(
                service.check(&pool, app, 42).await,
                Err(GithubError::Protocol)
            ));
            assert_eq!(server.calls.load(Ordering::SeqCst), 2);
            let staged_ref = {
                let store = Accounts::lock(&pool, app).unwrap();
                let row = store.row(42).await.unwrap().unwrap();
                assert_eq!(row.state, "refreshing");
                let staged = row.refresh_ref.clone().unwrap();
                let credential = store.read(&row, &staged).await.unwrap();
                assert!(credential.access_token.contains("rotated_access"));
                staged
            };
            pool.close().await;
            let pool = super::super::storage::open(app).await.unwrap();
            let status =
                serde_json::to_value(service.check(&pool, app, 42).await.unwrap()).unwrap();
            assert_eq!(status["accounts"][0]["login"], "first-renamed");
            assert_eq!(status["accounts"][1]["login"], "other");
            assert_eq!(server.calls.load(Ordering::SeqCst), 3);
            assert!(vault.read(&old_ref).await.is_err());
            assert!(vault.read(&staged_ref).await.is_ok());

            assert!(matches!(
                service.check(&pool, app, 84).await,
                Err(GithubError::RenewalUncertain)
            ));
            assert!(matches!(
                service.check(&pool, app, 84).await,
                Err(GithubError::Expired)
            ));
            assert_eq!(server.calls.load(Ordering::SeqCst), 4);

            let status =
                serde_json::to_value(service.disconnect(&pool, app, 42).await.unwrap()).unwrap();
            assert_eq!(status["accounts"].as_array().unwrap().len(), 1);
            assert_eq!(status["accounts"][0]["githubId"], 84);
            assert_eq!(status["revocations"][0]["githubId"], 42);
            assert!(vault.read(&staged_ref).await.is_err());
            assert!(vault.read(&other_ref).await.is_ok());
            let status = serde_json::to_value(
                service
                    .acknowledge_revocation(&pool, app, 42)
                    .await
                    .unwrap(),
            )
            .unwrap();
            assert!(status["revocations"].as_array().unwrap().is_empty());
            service.disconnect(&pool, app, 84).await.unwrap();
            assert!(vault.read(&other_ref).await.is_err());
            pool.close().await;
            server.worker.join().unwrap();
        });
    }

    #[test]
    #[ignore = "requires an isolated OS credential store"]
    fn browses_authorized_resources_with_the_selected_credential() {
        tauri::async_runtime::block_on(async {
            let server = server_with_status(
                vec![
                    (
                        "/user/installations?per_page=20&page=1",
                        200,
                        "Link: <https://example.test/unsafe>; rel=\"next\"\r\n",
                        r#"{"installations":[{"id":7,"account":{"login":"example-org"},"target_type":"Organization","repository_selection":"selected","suspended_at":null}]}"#.into(),
                    ),
                    (
                        "/user/installations?per_page=20&page=2",
                        200,
                        "X-GitHub-SSO: partial-results; organizations=123\r\n",
                        r#"{"installations":[]}"#.into(),
                    ),
                    (
                        "/user/installations/7/repositories?per_page=20&page=1",
                        200,
                        "",
                        r#"{"repositories":[{"id":123,"full_name":"example-org/private-test","private":true,"archived":false,"clone_url":"https://example.test/never-forwarded"}]}"#.into(),
                    ),
                    (
                        "/user/installations/7/repositories?per_page=20&page=2",
                        429,
                        "Retry-After: 120\r\n",
                        "{}".into(),
                    ),
                    (
                        "/user/installations?per_page=20&page=1",
                        401,
                        "",
                        "{}".into(),
                    ),
                ],
                Duration::ZERO,
            );
            let service = fixture(&server);
            let directory = tempfile::tempdir().unwrap();
            let app = directory.path();
            let pool = super::super::storage::open(app).await.unwrap();
            let other_tokens = TOKENS.replace("fixture_", "other_");
            {
                let store = Accounts::lock(&pool, app).unwrap();
                for (id, login, tokens) in [(42, "first", TOKENS), (84, "other", &other_tokens)] {
                    let credential = parse_tokens(tokens.as_bytes())
                        .unwrap()
                        .credential(id, github_accounts::now().unwrap());
                    store.save(login, &credential, None).await.unwrap();
                }
            }
            let page = service.installations(&pool, app, 42, 1).await.unwrap();
            let page = serde_json::to_value(page).unwrap();
            assert_eq!(page["items"][0]["kind"], "Organization");
            assert_eq!(page["items"][0]["selection"], "selected");
            assert_eq!(page["hasNext"], true);
            let page = service.installations(&pool, app, 42, 2).await.unwrap();
            assert_eq!(serde_json::to_value(page).unwrap()["partialSso"], true);
            let page = service.repositories(&pool, app, 42, 7, 1).await.unwrap();
            let page = serde_json::to_value(page).unwrap();
            assert_eq!(page["items"][0]["fullName"], "example-org/private-test");
            assert_eq!(page["items"][0]["visibility"], "private");
            assert!(!page.to_string().contains("never-forwarded"));
            let ResourceError::Limited { retry_at } = service
                .repositories(&pool, app, 42, 7, 2)
                .await
                .err()
                .unwrap()
            else {
                panic!("GitHub's retry deadline must be preserved");
            };
            assert!(retry_at >= github_accounts::now().unwrap() + 119);
            assert!(matches!(
                service.installations(&pool, app, 42, 1).await,
                Err(ResourceError::Limited { retry_at: deadline }) if deadline == retry_at
            ));
            assert!(matches!(
                service.installations(&pool, app, 84, 1).await,
                Err(ResourceError::Account(GithubError::NotConnected))
            ));
            {
                let store = Accounts::lock(&pool, app).unwrap();
                assert_eq!(store.row(84).await.unwrap().unwrap().state, "reauthorize");
                assert_eq!(store.row(42).await.unwrap().unwrap().state, "active");
            }
            assert_eq!(server.calls.load(Ordering::SeqCst), 5);
            let auth = server.authorizations.lock().unwrap().clone();
            assert_eq!(auth.len(), 5);
            assert!(auth[..4]
                .iter()
                .all(|value| value.contains("fixture_access")));
            assert!(auth[4].contains("other_access"));
            service.disconnect(&pool, app, 42).await.unwrap();
            service.disconnect(&pool, app, 84).await.unwrap();
            server.worker.join().unwrap();
        });
    }

    #[test]
    #[ignore = "requires an isolated OS credential store"]
    fn checks_installation_membership_before_activity_and_stops_at_rate_limits() {
        tauri::async_runtime::block_on(async {
            let membership = "/user/installations/7/repositories?per_page=20&page=1";
            let listed = r#"{"repositories":[{"id":10,"full_name":"owner/project","private":true,"archived":false}]}"#;
            let server = server_with_status(
                vec![
                    (membership, 200, "", listed.into()),
                    ("/repos/owner/project/pulls?state=all&per_page=20&page=1", 200, "", r#"[{"number":2,"title":"Pull request","state":"open","draft":true}]"#.into()),
                    (membership, 200, "", listed.into()),
                    ("/repos/owner/project/issues?state=all&per_page=20&page=2", 200,
                        "Link: <https://example.test/ignored>; rel=\"next\"\r\n",
                        r#"[{"number":2,"title":"Pull request","state":"open","pull_request":{}},{"number":1,"title":"Issue","state":"closed"}]"#.into()),
                    (membership, 200, "", r#"{"repositories":[]}"#.into()),
                    (membership, 200, "X-RateLimit-Remaining: 0\r\n", listed.into()),
                ],
                Duration::ZERO,
            );
            let service = fixture(&server);
            let directory = tempfile::tempdir().unwrap();
            let app = directory.path();
            let pool = super::super::storage::open(app).await.unwrap();
            {
                let store = Accounts::lock(&pool, app).unwrap();
                let credential = parse_tokens(TOKENS.as_bytes())
                    .unwrap()
                    .credential(42, github_accounts::now().unwrap());
                store.save("owner", &credential, None).await.unwrap();
            }
            let scope = RepositoryScope {
                github_id: 42,
                installation_id: 7,
                repository_id: 10,
                repository_page: 1,
            };
            let pulls = service
                .repository_activity(&pool, app, scope, ActivityKind::Pulls, 1)
                .await
                .unwrap();
            assert_eq!(
                serde_json::to_value(pulls).unwrap()["items"][0]["state"],
                "draft"
            );
            let issues = service
                .repository_activity(&pool, app, scope, ActivityKind::Issues, 2)
                .await
                .unwrap();
            let issues = serde_json::to_value(issues).unwrap();
            assert_eq!(issues["items"].as_array().unwrap().len(), 1);
            assert_eq!(issues["items"][0]["number"], 1);
            assert_eq!(issues["hasNext"], true);
            assert!(matches!(
                service
                    .repository_activity(&pool, app, scope, ActivityKind::Issues, 1)
                    .await,
                Err(ResourceError::Access(
                    resources::AccessError::RepositoryUnavailable
                ))
            ));
            for _ in 0..2 {
                assert!(matches!(
                    service
                        .repository_activity(&pool, app, scope, ActivityKind::Pulls, 1)
                        .await,
                    Err(ResourceError::Limited { .. })
                ));
            }
            assert_eq!(server.calls.load(Ordering::SeqCst), 6);
            service.disconnect(&pool, app, 42).await.unwrap();
            server.worker.join().unwrap();
        });
    }
}
