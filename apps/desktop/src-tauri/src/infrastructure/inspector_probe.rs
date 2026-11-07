use std::{
    path::{Path, PathBuf},
    process::ExitStatus,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::{fs, process::Command};
use url::Url;

use crate::repository::{InventoryError, Repository};

use super::{git, inspector, inspector_credentials, inspector_openpgp, inspector_ssh, native_file};

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProbeKind {
    Ssh,
    SshConfigured,
    Https,
    HttpsCredentials,
    Signing,
    OpenPgp,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProbeError {
    Unsupported,
    Stale,
    Unavailable,
    TimedOut,
    Failed,
    CredentialsUnavailable,
    SigningUnavailable,
}

impl From<InventoryError> for ProbeError {
    fn from(_: InventoryError) -> Self {
        Self::Unavailable
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProbePreview {
    confirmation: String,
    host: Option<String>,
    via: Option<String>,
    key: Option<String>,
    kind: ProbeKind,
}

#[derive(Serialize)]
enum Request {
    Remote {
        git: PathBuf,
        url: String,
        host: String,
        ssh: Option<PathBuf>,
    },
    SshConfigured {
        git: PathBuf,
        url: String,
        route: inspector_ssh::SshRoute,
    },
    HttpsCredentials {
        git: PathBuf,
        url: String,
        host: String,
        credentials: inspector_credentials::Credentials,
    },
    Signature {
        keygen: PathBuf,
        public: String,
    },
    OpenPgp(inspector_openpgp::OpenPgp),
}

fn allowed_host(host: &str) -> bool {
    matches!(host, "github.com" | "gitlab.com" | "bitbucket.org")
}

pub(super) fn allowed_ssh_host(host: &str) -> bool {
    allowed_host(host)
        || matches!(
            host,
            "ssh.github.com" | "altssh.gitlab.com" | "altssh.bitbucket.org"
        )
}

pub(super) fn allowed_path(path: &str) -> bool {
    path.len() <= 512
        && path.split('/').count() >= 2
        && path.split('/').all(|part| {
            !matches!(part, "" | "." | "..")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        })
}

fn remote_url(value: &str, kind: ProbeKind) -> Option<(String, String)> {
    if value.len() > 1024
        || value
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
        || value.contains(['\\', '%', '?', '#'])
    {
        return None;
    }
    let uri = value.contains("://");
    let (host, path) = if uri {
        let (_, raw_path) = value.split_once("://")?.1.split_once('/')?;
        if !allowed_path(raw_path) {
            return None;
        }
        let parsed = Url::parse(value).ok()?;
        let valid = match kind {
            ProbeKind::Ssh => parsed.scheme() == "ssh" && parsed.username() == "git",
            ProbeKind::Https => {
                parsed.scheme() == "https"
                    && parsed.username().is_empty()
                    && value
                        .split_once("://")?
                        .1
                        .split('/')
                        .next()?
                        .find('@')
                        .is_none()
            }
            _ => false,
        };
        if !valid || parsed.password().is_some() || parsed.port().is_some() {
            return None;
        }
        (
            parsed.host_str()?.to_owned(),
            parsed.path().strip_prefix('/')?.to_owned(),
        )
    } else {
        if !matches!(kind, ProbeKind::Ssh) {
            return None;
        }
        let (host, path) = value.strip_prefix("git@")?.split_once(':')?;
        (host.to_ascii_lowercase(), path.into())
    };
    if !allowed_host(&host) || !allowed_path(&path) {
        return None;
    }
    let url = match kind {
        ProbeKind::Ssh if uri => format!("ssh://git@{host}/{path}"),
        ProbeKind::Ssh => format!("git@{host}:{path}"),
        ProbeKind::Https => format!("https://{host}/{path}"),
        _ => return None,
    };
    Some((url, host))
}

fn public_key(value: &str) -> Option<String> {
    let mut fields = value.split_whitespace();
    let algorithm = fields.next()?;
    if !matches!(
        algorithm,
        "ssh-ed25519"
            | "ssh-rsa"
            | "ecdsa-sha2-nistp256"
            | "ecdsa-sha2-nistp384"
            | "ecdsa-sha2-nistp521"
            | "sk-ssh-ed25519@openssh.com"
    ) {
        return None;
    }
    let body = fields.next()?;
    if body.len() > 8192
        || !body
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"+/=".contains(&byte))
    {
        return None;
    }
    Some(format!("{algorithm} {body}\n"))
}

async fn prepare_request(
    repo: &Repository,
    kind: ProbeKind,
    remote: Option<&str>,
    scratch: &Path,
) -> Result<Request, ProbeError> {
    let (exe, cwd) = inspector::context(repo).await?;
    if matches!(kind, ProbeKind::OpenPgp) {
        return Ok(Request::OpenPgp(
            inspector_openpgp::prepare(&exe, cwd).await?,
        ));
    }
    if matches!(kind, ProbeKind::Signing) {
        if inspector::line(&exe, cwd, &["config", "--get", "gpg.format"])
            .await
            .as_deref()
            != Some("ssh")
        {
            return Err(ProbeError::Unsupported);
        }
        let (status, _) =
            inspector::run(&exe, cwd, &["config", "--get", "gpg.ssh.program"]).await?;
        if status.code() != Some(1) {
            return Err(ProbeError::Unsupported);
        }
        let key = inspector::line(&exe, cwd, &["config", "--get", "user.signingkey"])
            .await
            .ok_or(ProbeError::Unsupported)?;
        let public = if let Some(literal) = key
            .strip_prefix("key::")
            .or_else(|| key.starts_with("ssh-").then_some(key.as_str()))
        {
            public_key(literal)
        } else {
            let file =
                inspector::line(&exe, cwd, &["config", "--path", "--get", "user.signingkey"])
                    .await
                    .ok_or(ProbeError::Unsupported)?;
            let directory = inspector::ssh_home(&exe, cwd)
                .await
                .ok_or(ProbeError::Unsupported)?;
            let file = PathBuf::from(file);
            if !file.is_absolute()
                || file.parent() != Some(directory.as_path())
                || file.extension().and_then(|part| part.to_str()) != Some("pub")
            {
                return Err(ProbeError::Unsupported);
            }
            let snapshot = tauri::async_runtime::spawn_blocking(move || {
                native_file::parent_id(&file)?;
                native_file::snapshot(&file)
            })
            .await
            .map_err(|_| ProbeError::Unavailable)?
            .map_err(|_| ProbeError::Unavailable)?;
            snapshot.and_then(|value| std::str::from_utf8(&value.bytes).ok().and_then(public_key))
        }
        .ok_or(ProbeError::Unsupported)?;
        let keygen = git::find_executable("ssh-keygen").ok_or(ProbeError::Unavailable)?;
        return Ok(Request::Signature { keygen, public });
    }
    let name = remote
        .filter(|name| {
            !name.is_empty()
                && name.len() <= 128
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"./-_".contains(&byte))
        })
        .ok_or(ProbeError::Unsupported)?;
    let (status, value) =
        inspector::run(&exe, cwd, &["remote", "get-url", "--all", "--", name]).await?;
    if !status.success() {
        return Err(ProbeError::Unsupported);
    }
    let text = std::str::from_utf8(&value).map_err(|_| ProbeError::Unsupported)?;
    let mut lines = text.lines();
    let value = lines.next().ok_or(ProbeError::Unsupported)?;
    if lines.next().is_some() {
        return Err(ProbeError::Unsupported);
    }
    if matches!(kind, ProbeKind::SshConfigured) {
        let route = inspector_ssh::prepare(&exe, cwd, value, scratch).await?;
        return Ok(Request::SshConfigured {
            git: exe,
            url: value.into(),
            route,
        });
    }
    if matches!(kind, ProbeKind::HttpsCredentials) {
        let (url, host) = remote_url(value, ProbeKind::Https).ok_or(ProbeError::Unsupported)?;
        let credentials = inspector_credentials::prepare(&exe, cwd).await?;
        return Ok(Request::HttpsCredentials {
            git: exe,
            url,
            host,
            credentials,
        });
    }
    let (url, host) = remote_url(value, kind).ok_or(ProbeError::Unsupported)?;
    let ssh = if matches!(kind, ProbeKind::Ssh) {
        Some(git::find_executable("ssh").ok_or(ProbeError::Unavailable)?)
    } else {
        None
    };
    Ok(Request::Remote {
        git: exe,
        url,
        host,
        ssh,
    })
}

impl Request {
    fn preview(&self, repo: &Repository, kind: ProbeKind) -> Result<ProbePreview, ProbeError> {
        let (host, via, key) = match self {
            Self::Remote { host, .. } | Self::HttpsCredentials { host, .. } => {
                (Some(host.clone()), None, None)
            }
            Self::SshConfigured { route, .. } => (
                Some(route.target().into()),
                route.via().map(str::to_owned),
                None,
            ),
            Self::Signature { .. } => (None, None, None),
            Self::OpenPgp(probe) => (None, None, Some(probe.key().into())),
        };
        let bytes = serde_json::to_vec(&(&repo.path, &repo.git_dir, repo.revision, kind, self))
            .map_err(|_| ProbeError::Unavailable)?;
        Ok(ProbePreview {
            confirmation: hex::encode(native_file::digest(&bytes)),
            host,
            via,
            key,
            kind,
        })
    }
}

pub(crate) async fn preview(
    repo: &Repository,
    app: &Path,
    kind: ProbeKind,
    remote: Option<&str>,
) -> Result<ProbePreview, ProbeError> {
    let scratch = scratch(app)?;
    prepare_request(repo, kind, remote, scratch.path())
        .await?
        .preview(repo, kind)
}

pub(crate) async fn verify(
    repo: &Repository,
    app: &Path,
    kind: ProbeKind,
    remote: Option<&str>,
    confirmation: &str,
) -> Result<(), ProbeError> {
    let scratch = scratch(app)?;
    let scratch = scratch.path();
    let request = prepare_request(repo, kind, remote, scratch).await?;
    if request.preview(repo, kind)?.confirmation != confirmation {
        return Err(ProbeError::Stale);
    }
    match request {
        Request::Remote {
            git: executable,
            url,
            ssh,
            ..
        } => {
            let mut command = git::isolated(&executable, scratch);
            if let Some(ssh) = ssh {
                let executable = shell_path(&ssh)?;
                ssh_command(&mut command, &format!("{executable} -F none -o BatchMode=yes -o StrictHostKeyChecking=yes -o UpdateHostKeys=no -o ConnectTimeout=5 -o ServerAliveInterval=4 -o ServerAliveCountMax=1 -o ClearAllForwardings=yes"));
            } else {
                https_command(&mut command, scratch);
            }
            remote_read(&mut command, &url).await
        }
        Request::SshConfigured { git, url, route } => {
            let mut command = git::isolated(&git, scratch);
            ssh_command(&mut command, &route.command()?);
            remote_read(&mut command, &url).await
        }
        Request::HttpsCredentials {
            git,
            url,
            credentials,
            ..
        } => {
            let header = credentials.header(&git, scratch, &url).await?;
            let mut command = git::isolated(&git, scratch);
            https_command(&mut command, scratch);
            command
                .env("GITPILOT_AUTH_HEADER", header.as_str())
                .arg(format!(
                    "--config-env=http.{url}.extraHeader=GITPILOT_AUTH_HEADER"
                ));
            remote_read(&mut command, &url).await
        }
        Request::Signature { keygen, public } => {
            verify_signature(&keygen, scratch, &public, None).await
        }
        Request::OpenPgp(probe) => probe.verify(scratch).await,
    }
}

fn scratch(app: &Path) -> Result<tempfile::TempDir, ProbeError> {
    tempfile::Builder::new()
        .prefix("gitpilot-probe-")
        .tempdir_in(app)
        .map_err(|_| ProbeError::Unavailable)
}

pub(super) fn shell_path(path: &Path) -> Result<String, ProbeError> {
    let path = path.to_str().ok_or(ProbeError::Unsupported)?;
    let path = if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.into()
    };
    Ok(format!("'{}'", path.replace('\'', "'\\''")))
}

fn ssh_command(command: &mut Command, ssh: &str) {
    command
        .env("GIT_ALLOW_PROTOCOL", "ssh")
        .env("GIT_SSH_VARIANT", "ssh")
        .env("GIT_SSH_COMMAND", ssh);
}

fn https_command(command: &mut Command, scratch: &Path) {
    command
        .env("GIT_ALLOW_PROTOCOL", "https")
        .env("HOME", scratch)
        .env("USERPROFILE", scratch)
        .env("CURL_HOME", scratch)
        .args([
            "-c",
            "credential.helper=",
            "-c",
            "http.sslVerify=true",
            "-c",
            "http.followRedirects=false",
            "-c",
            "http.lowSpeedLimit=1",
            "-c",
            "http.lowSpeedTime=5",
        ]);
}

async fn remote_read(command: &mut Command, url: &str) -> Result<(), ProbeError> {
    success(
        command.args(["ls-remote", "--quiet", "--", url, "HEAD"]),
        None,
    )
    .await
}

pub(super) async fn bounded(
    command: &mut Command,
    input: Option<&[u8]>,
) -> Result<(ExitStatus, Vec<u8>), ProbeError> {
    git::output_with_input(command, Duration::from_secs(8), input)
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::TimedOut {
                ProbeError::TimedOut
            } else {
                ProbeError::Unavailable
            }
        })
}

async fn success(command: &mut Command, input: Option<&[u8]>) -> Result<(), ProbeError> {
    let (status, _) = bounded(command, input).await?;
    if status.success() {
        Ok(())
    } else {
        Err(ProbeError::Failed)
    }
}

async fn verify_signature(
    keygen: &Path,
    scratch: &Path,
    public: &str,
    agent: Option<&std::ffi::OsStr>,
) -> Result<(), ProbeError> {
    let key = scratch.join("key.pub");
    let message = scratch.join("gitpilot-challenge");
    let challenge = b"GitPilot temporary SSH signature check\n";
    fs::write(&key, public)
        .await
        .map_err(|_| ProbeError::Unavailable)?;
    fs::write(&message, challenge)
        .await
        .map_err(|_| ProbeError::Unavailable)?;
    let mut sign = Command::new(keygen);
    sign.current_dir(scratch)
        .env_remove("SSH_ASKPASS")
        .env("SSH_ASKPASS_REQUIRE", "never");
    if let Some(agent) = agent {
        sign.env("SSH_AUTH_SOCK", agent);
    }
    success(
        sign.args(["-Y", "sign", "-n", "git", "-f"])
            .arg(&key)
            .arg(&message),
        None,
    )
    .await?;
    let mut verify = Command::new(keygen);
    success(
        verify
            .current_dir(scratch)
            .args(["-Y", "check-novalidate", "-n", "git", "-s"])
            .arg(scratch.join("gitpilot-challenge.sig")),
        Some(challenge),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_require_direct_allowed_destinations_and_public_keys() {
        assert_eq!(
            remote_url("git@github.com:owner/project.git", ProbeKind::Ssh).unwrap(),
            (
                "git@github.com:owner/project.git".into(),
                "github.com".into()
            ),
        );
        assert!(remote_url("https://gitlab.com/group/project.git", ProbeKind::Https).is_some());
        for url in [
            "https://token@github.com/owner/project",
            "https://github.com/owner/project?token",
            "https://github.com/../project",
            "https://evil.test/owner/project",
            "https://github.com:443@evil.test/owner/project",
        ] {
            assert!(remote_url(url, ProbeKind::Https).is_none(), "{url}");
        }
        assert!(remote_url("git@corp:owner/project", ProbeKind::Ssh).is_none());
        assert!(public_key("ssh-ed25519 AAAA comment").is_some());
        assert!(public_key("-----BEGIN OPENSSH PRIVATE KEY-----").is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn signs_and_verifies_with_an_isolated_ssh_agent() {
        use std::process::Stdio;
        tauri::async_runtime::block_on(async {
            let (Some(agent), Some(add), Some(keygen)) = (
                git::find_executable("ssh-agent"),
                git::find_executable("ssh-add"),
                git::find_executable("ssh-keygen"),
            ) else {
                return;
            };
            let fixture = tempfile::tempdir().unwrap();
            let probe = tempfile::tempdir().unwrap();
            let socket = fixture.path().join("agent.sock");
            let key = fixture.path().join("test-key");
            success(
                Command::new(&keygen)
                    .args(["-q", "-t", "ed25519", "-N", "", "-f"])
                    .arg(&key),
                None,
            )
            .await
            .unwrap();
            let mut process = Command::new(agent)
                .args(["-D", "-a"])
                .arg(&socket)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .unwrap();
            for _ in 0..80 {
                if socket.exists() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            success(
                Command::new(add).env("SSH_AUTH_SOCK", &socket).arg(&key),
                None,
            )
            .await
            .unwrap();
            let text = fs::read_to_string(key.with_extension("pub")).await.unwrap();
            let key = public_key(&text).unwrap();
            verify_signature(&keygen, probe.path(), &key, Some(socket.as_os_str()))
                .await
                .unwrap();
            process.kill().await.unwrap();
        });
    }
}
