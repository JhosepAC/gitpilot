use std::{
    env,
    path::{Path, PathBuf},
};

use serde::Serialize;
use tokio::{fs, process::Command};
use url::Url;

use super::{git, inspector, inspector_probe, native_file};
use inspector_probe::ProbeError;

const CONFIG: &str = "gitpilot-ssh-0";
const POLICY: &str = "BatchMode yes\nStrictHostKeyChecking yes\nUpdateHostKeys no\n\
    ClearAllForwardings yes\nForwardAgent no\nConnectTimeout 5\n\
    ServerAliveInterval 4\nServerAliveCountMax 1\nCanonicalizeHostname no\n";

#[derive(Serialize)]
pub(super) struct SshRoute {
    client: PathBuf,
    files: Vec<Vec<Directive>>,
    target: String,
    via: Option<String>,
}

#[derive(Serialize)]
enum Directive {
    Line(String),
    Include(usize),
}

struct Destination<'a> {
    host: &'a str,
    user: Option<&'a str>,
    port: Option<u16>,
}

struct RemoteAddress {
    host: String,
    user: Option<String>,
    port: Option<u16>,
}

struct Resolved {
    host: String,
    user: String,
    port: u16,
    proxy: Option<String>,
}

fn hostname(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
        })
}

fn jump(value: &str) -> Option<Destination<'_>> {
    let (user, host) = value
        .split_once('@')
        .map_or((None, value), |(user, host)| (Some(user), host));
    if user.is_some_and(|user| {
        user.is_empty()
            || user.starts_with('-')
            || !user
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    }) {
        return None;
    }
    let (host, port) = match host.split_once(':') {
        Some((host, port)) => (
            host,
            Some(port.parse::<u16>().ok().filter(|port| *port != 0)?),
        ),
        None => (host, None),
    };
    hostname(host).then_some(Destination { host, user, port })
}

fn path_value(value: &str) -> Option<String> {
    let path = if let Some(quoted) = value.strip_prefix('"') {
        quoted.strip_suffix('"')?
    } else {
        if value.chars().any(char::is_whitespace) {
            return None;
        }
        value
    };
    let path = if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.into()
    };
    let absolute = path.starts_with("~/") || Path::new(&path).is_absolute();
    if !absolute
        || path.starts_with("//")
        || path.contains(['"', '\'', '\\', '%', '$', '*', '?', '[', ']', '#'])
        || path.chars().any(char::is_control)
    {
        return None;
    }
    Some(path)
}

fn include_name(name: &str) -> bool {
    name.len() <= 240
        && name.split('/').all(|part| {
            !matches!(part, "" | "." | "..")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        })
}

fn validate_directive(key: &str, value: &str) -> Option<String> {
    match key {
        "host"
            if !value.is_empty()
                && value.split_ascii_whitespace().all(|pattern| {
                    !pattern.starts_with('-')
                        && pattern
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"._-*!?".contains(&byte))
                }) =>
        {
            Some(format!(
                "Host {}",
                value.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
            ))
        }
        "hostname" | "hostkeyalias" if hostname(value) => Some(format!("{key} {value}")),
        "user" if hostname(value) => Some(format!("{key} {value}")),
        "port" if value.parse::<u16>().is_ok_and(|port| port != 0) => {
            Some(format!("{key} {value}"))
        }
        "identitiesonly" | "batchmode" if matches!(value, "yes" | "no") => {
            Some(format!("{key} {value}"))
        }
        "preferredauthentications" if value == "publickey" => Some(format!("{key} {value}")),
        "stricthostkeychecking" if matches!(value, "yes" | "ask" | "accept-new") => {
            Some(format!("{key} {value}"))
        }
        "proxyjump" if value == "none" || jump(value).is_some() => Some(format!("{key} {value}")),
        "identityfile" | "certificatefile" | "userknownhostsfile" => {
            path_value(value).map(|path| format!("{key} \"{path}\""))
        }
        _ => None,
    }
}

fn read_config(
    root: &Path,
    path: &Path,
    files: &mut Vec<Vec<Directive>>,
    budget: &mut usize,
) -> Result<usize, ProbeError> {
    if files.len() >= 12 {
        return Err(ProbeError::Unsupported);
    }
    let index = files.len();
    files.push(Vec::new());
    let parent = native_file::parent_id(path);
    if index == 0
        && parent
            .as_ref()
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
    {
        return Ok(index);
    }
    parent.map_err(|_| ProbeError::Unsupported)?;
    let snapshot = native_file::snapshot(path).map_err(|_| ProbeError::Unsupported)?;
    let Some(snapshot) = snapshot else {
        return if index == 0 {
            Ok(index)
        } else {
            Err(ProbeError::Unsupported)
        };
    };
    *budget = budget
        .checked_sub(snapshot.bytes.len())
        .ok_or(ProbeError::Unsupported)?;
    let text = std::str::from_utf8(&snapshot.bytes).map_err(|_| ProbeError::Unsupported)?;
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once(|ch: char| ch.is_ascii_whitespace() || ch == '=')
            .ok_or(ProbeError::Unsupported)?;
        let value = value.trim_start_matches(|ch: char| ch.is_ascii_whitespace() || ch == '=');
        let key = key.to_ascii_lowercase();
        if key == "include" {
            if value.is_empty() {
                return Err(ProbeError::Unsupported);
            }
            for name in value.split_ascii_whitespace() {
                if !include_name(name) {
                    return Err(ProbeError::Unsupported);
                }
                let nested = read_config(root, &root.join(name), files, budget)?;
                files[index].push(Directive::Include(nested));
            }
        } else {
            let line = validate_directive(&key, value).ok_or(ProbeError::Unsupported)?;
            files[index].push(Directive::Line(line));
        }
    }
    Ok(index)
}

fn remote_address(value: &str) -> Option<RemoteAddress> {
    if value.len() > 1024
        || value
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control())
        || value.contains(['\\', '%', '?', '#'])
    {
        return None;
    }
    if let Some((scheme, rest)) = value.split_once("://") {
        let (authority, path) = rest.split_once('/')?;
        if scheme != "ssh" || !inspector_probe::allowed_path(path) || authority.ends_with('@') {
            return None;
        }
        let url = Url::parse(value).ok()?;
        let host = url.host_str()?;
        let user = url.username();
        if !hostname(host)
            || !matches!(user, "" | "git")
            || url.password().is_some()
            || url.port() == Some(0)
        {
            return None;
        }
        return Some(RemoteAddress {
            host: host.into(),
            user: (!user.is_empty()).then(|| user.into()),
            port: url.port(),
        });
    }
    let (target, path) = value.split_once(':')?;
    let host = target.strip_prefix("git@").unwrap_or(target);
    if !hostname(host) || !inspector_probe::allowed_path(path) {
        return None;
    }
    Some(RemoteAddress {
        host: host.into(),
        user: (host != target).then(|| "git".into()),
        port: None,
    })
}

async fn effective(
    client: &Path,
    scratch: &Path,
    destination: Destination<'_>,
) -> Result<Resolved, ProbeError> {
    let mut command = Command::new(client);
    command.current_dir(scratch).args(["-G", "-F", CONFIG]);
    if let Some(user) = destination.user {
        command.args(["-l", user]);
    }
    if let Some(port) = destination.port {
        command.arg("-p").arg(port.to_string());
    }
    let (status, bytes) = inspector_probe::bounded(command.arg(destination.host), None).await?;
    if !status.success() {
        return Err(ProbeError::Unsupported);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| ProbeError::Unsupported)?;
    let field = |key: &str| {
        text.lines()
            .filter_map(|line| line.split_once(' '))
            .find_map(|(name, value)| (name == key).then_some(value))
    };
    let host = field("hostname")
        .filter(|host| hostname(host))
        .ok_or(ProbeError::Unsupported)?;
    let user = field("user").ok_or(ProbeError::Unsupported)?;
    let port = field("port")
        .and_then(|port| port.parse().ok())
        .filter(|port| *port != 0)
        .ok_or(ProbeError::Unsupported)?;
    let via = field("proxyjump")
        .filter(|value| *value != "none")
        .map(str::to_owned);
    Ok(Resolved {
        host: host.to_ascii_lowercase(),
        user: user.into(),
        port,
        proxy: via,
    })
}

pub(super) async fn prepare(
    exe: &Path,
    cwd: &Path,
    value: &str,
    scratch: &Path,
) -> Result<SshRoute, ProbeError> {
    if ["GIT_SSH", "GIT_SSH_COMMAND", "GIT_SSH_VARIANT"]
        .iter()
        .any(|key| env::var_os(key).is_some())
    {
        return Err(ProbeError::Unsupported);
    }
    let (status, _) = inspector::run(
        exe,
        cwd,
        &[
            "config",
            "--get-regexp",
            r"^(core\.sshcommand|ssh\.variant)$",
        ],
    )
    .await?;
    if status.code() != Some(1) {
        return Err(ProbeError::Unsupported);
    }
    let address = remote_address(value).ok_or(ProbeError::Unsupported)?;
    let client = git::find_executable("ssh").ok_or(ProbeError::Unavailable)?;
    let directory = inspector::ssh_home(exe, cwd)
        .await
        .ok_or(ProbeError::Unavailable)?;
    let files = tauri::async_runtime::spawn_blocking(move || {
        let mut files = Vec::new();
        read_config(
            &directory,
            &directory.join("config"),
            &mut files,
            &mut 32_768,
        )?;
        Ok::<_, ProbeError>(files)
    })
    .await
    .map_err(|_| ProbeError::Unavailable)??;
    let mut route = SshRoute {
        client,
        files,
        target: String::new(),
        via: None,
    };
    route.write_config(scratch).await?;
    let resolved = effective(
        &route.client,
        scratch,
        Destination {
            host: &address.host,
            user: address.user.as_deref(),
            port: address.port,
        },
    )
    .await?;
    if resolved.user != "git" || !inspector_probe::allowed_ssh_host(&resolved.host) {
        return Err(ProbeError::Unsupported);
    }
    route.target = format!("{}:{}", resolved.host, resolved.port);
    if let Some(proxy) = resolved.proxy {
        // OpenSSH builds a shell command for ProxyJump. Both the client path and
        // the relative -F filename must be safe even without shell quoting.
        let program = route.client.to_str().ok_or(ProbeError::Unsupported)?;
        if !program
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/\\:._-".contains(&byte))
        {
            return Err(ProbeError::Unsupported);
        }
        let destination = jump(&proxy).ok_or(ProbeError::Unsupported)?;
        let resolved = effective(&route.client, scratch, destination).await?;
        if resolved.proxy.is_some() || !hostname(&resolved.user) {
            return Err(ProbeError::Unsupported);
        }
        route.via = Some(format!("{}:{}", resolved.host, resolved.port));
    }
    Ok(route)
}

impl SshRoute {
    async fn write_config(&self, scratch: &Path) -> Result<(), ProbeError> {
        for (index, file) in self.files.iter().enumerate() {
            let mut contents = if index == 0 {
                String::from(POLICY)
            } else {
                String::new()
            };
            for directive in file {
                match directive {
                    Directive::Line(line) => contents.push_str(line),
                    Directive::Include(index) => {
                        let path = scratch.join(format!("gitpilot-ssh-{index}"));
                        let path = path.to_str().ok_or(ProbeError::Unsupported)?;
                        let quoted =
                            path_value(&format!("\"{path}\"")).ok_or(ProbeError::Unsupported)?;
                        contents.push_str(&format!("Include \"{quoted}\""));
                    }
                }
                contents.push('\n');
            }
            fs::write(scratch.join(format!("gitpilot-ssh-{index}")), contents)
                .await
                .map_err(|_| ProbeError::Unavailable)?;
        }
        Ok(())
    }

    pub(super) fn target(&self) -> &str {
        &self.target
    }
    pub(super) fn via(&self) -> Option<&str> {
        self.via.as_deref()
    }

    pub(super) fn command(&self) -> Result<String, ProbeError> {
        Ok(format!(
            "{} -F {CONFIG}",
            inspector_probe::shell_path(&self.client)?
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_snapshot_refuses_directives_that_can_run_commands() {
        for key in [
            "match",
            "proxycommand",
            "localcommand",
            "knownhostscommand",
            "identityagent",
            "localforward",
            "controlpath",
        ] {
            assert!(validate_directive(key, "echo unsafe").is_none());
        }
        assert!(validate_directive("proxyjump", "git@host;touch").is_none());
        assert!(validate_directive("proxyjump", "jump,other").is_none());
        assert!(validate_directive("proxyjump", "git@host:2222").is_some());
        assert!(validate_directive("host", "github-* !github-home").is_some());
        assert!(include_name("work/hosts.conf"));
        assert!(!include_name("../ssh/config"));
        assert!(!include_name("work/*.conf"));
        assert!(remote_address("ssh://git@company:443/owner/repo").is_some());
        assert!(remote_address("ssh://git@company:0/owner/repo").is_none());
        assert!(remote_address("company:owner/repo").is_some());
    }
}
