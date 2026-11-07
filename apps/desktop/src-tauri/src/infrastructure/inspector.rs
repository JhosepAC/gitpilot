use std::{
    collections::BTreeSet,
    env,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::Serialize;
use tokio::fs;
use url::Url;

use crate::repository::{InventoryError, Repository};

use super::{discovery, git, native_file};

const GIT_TIME: Duration = Duration::from_secs(2);
const MAX_REMOTES: usize = 12;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Inspector {
    branch: Option<String>,
    head: Option<String>,
    tree: Option<Tree>,
    remotes: Vec<Remote>,
    remotes_partial: bool,
    ssh: Ssh,
    https: Https,
    signing: Signing,
}

#[derive(Serialize)]
struct Tree {
    staged: usize,
    untracked: usize,
    conflicts: usize,
    unstaged: Option<usize>,
}

#[derive(Serialize, PartialEq, Eq)]
struct Endpoint {
    transport: &'static str,
    host: Option<String>,
}

#[derive(Serialize)]
struct Remote {
    name: String,
    fetch: Vec<Endpoint>,
    push: Vec<Endpoint>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Ssh {
    client_available: bool,
    custom_command: bool,
    agent_socket: bool,
    config_status: &'static str,
    aliases: Vec<Alias>,
    default_keys: Keys,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Alias {
    name: String,
    host: Option<String>,
    keys: Keys,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Keys {
    present: usize,
    missing: usize,
    restricted: usize,
    unchecked: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Https {
    helpers: Vec<&'static str>,
    scoped_helpers: bool,
    askpass_configured: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Signing {
    enabled: Option<bool>,
    format: &'static str,
    key_reference: &'static str,
    key_file: Option<&'static str>,
    custom_program: bool,
    default_program_available: bool,
}

struct Settings(Vec<(String, String)>);

impl Settings {
    fn last(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .rev()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }
}

pub(super) async fn run(
    exe: &Path,
    cwd: &Path,
    args: &[&str],
) -> Result<(std::process::ExitStatus, Vec<u8>), InventoryError> {
    run_with_input(exe, cwd, args, None).await
}

async fn run_with_input(
    exe: &Path,
    cwd: &Path,
    args: &[&str],
    input: Option<&[u8]>,
) -> Result<(std::process::ExitStatus, Vec<u8>), InventoryError> {
    let mut command = git::command(exe, cwd);
    // Block transport helpers, optional index writes, filesystem monitors and pager use.
    command
        .env("GIT_ALLOW_PROTOCOL", "")
        .env("GIT_NO_LAZY_FETCH", "1")
        .args([
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
        ]);
    git::output_with_input(command.args(args), GIT_TIME, input)
        .await
        .map_err(|_| InventoryError::GitReadFailed)
}

fn clean(value: &str, limit: usize) -> Option<&str> {
    (!value.is_empty() && value.len() <= limit && !value.chars().any(char::is_control))
        .then_some(value)
}

pub(super) async fn line(exe: &Path, cwd: &Path, args: &[&str]) -> Option<String> {
    let (status, out) = run(exe, cwd, args).await.ok()?;
    let text = std::str::from_utf8(&out)
        .ok()?
        .trim_end_matches(['\r', '\n']);
    status.success().then_some(())?;
    clean(text, 4096).map(str::to_owned)
}

async fn settings(exe: &Path, cwd: &Path) -> Result<Settings, InventoryError> {
    let (status, out) = run(exe, cwd, &["config", "--null", "--get-regexp",
        r"^(remote\..*\.(url|pushurl)|credential(\..*)?\.helper|commit\.gpgsign|gpg(\..*)?\.program|gpg\.format|gpg\.ssh\.defaultkeycommand|user\.signingkey|core\.(sshcommand|askpass))$",
    ]).await?;
    if status.code() == Some(1) && out.is_empty() {
        return Ok(Settings(Vec::new()));
    }
    if !status.success() || !out.ends_with(b"\0") {
        return Err(InventoryError::GitReadFailed);
    }
    let text = std::str::from_utf8(&out).map_err(|_| InventoryError::GitReadFailed)?;
    Ok(Settings(
        text.split_terminator('\0')
            .filter_map(|record| {
                if let Some((name, value)) = record.split_once('\n') {
                    clean(name, 4096).map(|name| (name.into(), value.into()))
                } else {
                    clean(record, 4096).map(|name| (name.into(), "true".into()))
                }
            })
            .collect(),
    ))
}

fn public_host(host: &str) -> Option<String> {
    let name = host.trim_matches(['[', ']']);
    let safe = clean(name, 253)?;
    if safe.parse::<std::net::IpAddr>().is_ok()
        || safe
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            && safe.bytes().any(|byte| byte.is_ascii_alphanumeric())
    {
        return Some(safe.to_ascii_lowercase());
    }
    None
}

fn endpoint(value: &str) -> Endpoint {
    let unknown = Endpoint {
        transport: "other",
        host: None,
    };
    if clean(value, 4096).is_none() || value.chars().any(char::is_whitespace) {
        return unknown;
    }
    if let Some((scheme, _)) = value.split_once("://") {
        let transport = match scheme {
            "ssh" | "git+ssh" | "ssh+git" => "ssh",
            "https" => "https",
            "http" => "http",
            "git" => "git",
            "file" => {
                return Endpoint {
                    transport: "local",
                    host: None,
                }
            }
            _ => return unknown,
        };
        if value.contains('\\') {
            return unknown;
        }
        return Endpoint {
            transport,
            host: Url::parse(value)
                .ok()
                .and_then(|url| url.host_str().and_then(public_host)),
        };
    }
    if value.contains("::") && !value.contains('[') {
        return unknown;
    }
    if Path::new(value).is_absolute()
        || value
            .as_bytes()
            .get(1..3)
            .is_some_and(|prefix| matches!(prefix, b":/" | b":\\"))
    {
        return Endpoint {
            transport: "local",
            host: None,
        };
    }
    let separator = if let Some(index) = value.find(']') {
        value
            .get(index + 1..)
            .filter(|rest| rest.starts_with(':'))
            .map(|_| index + 1)
    } else {
        value.find(':')
    };
    if let Some(index) = separator {
        let host = value[..index].rsplit('@').next().and_then(public_host);
        if host.is_some() {
            return Endpoint {
                transport: "ssh",
                host,
            };
        }
        return unknown;
    }
    Endpoint {
        transport: "local",
        host: None,
    }
}

async fn read_endpoints(
    exe: &Path,
    cwd: &Path,
    name: &str,
    push: bool,
) -> Option<(Vec<Endpoint>, bool)> {
    let mut args = vec!["remote", "get-url", "--all"];
    if push {
        args.push("--push");
    }
    args.extend(["--", name]);
    let (status, out) = run(exe, cwd, &args).await.ok()?;
    if !status.success() {
        return None;
    }
    let text = std::str::from_utf8(&out).ok()?;
    let mut endpoints = Vec::new();
    let mut lines = text.lines();
    for value in lines.by_ref().take(16) {
        let item = endpoint(value);
        if !endpoints.contains(&item) {
            endpoints.push(item);
        }
    }
    Some((endpoints, lines.next().is_some()))
}

async fn remotes(exe: &Path, cwd: &Path, config: &Settings) -> (Vec<Remote>, bool) {
    let mut names = BTreeSet::new();
    let mut partial = false;
    for (key, _) in &config.0 {
        if let Some(name) = key.strip_prefix("remote.").and_then(|key| {
            key.strip_suffix(".url")
                .or_else(|| key.strip_suffix(".pushurl"))
        }) {
            if clean(name, 128).is_some()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"./-_".contains(&byte))
            {
                names.insert(name);
            } else {
                partial = true;
            }
        }
    }
    partial |= names.len() > MAX_REMOTES;
    let mut remotes = Vec::new();
    let finished = tokio::time::timeout(Duration::from_secs(8), async {
        for name in names.into_iter().take(MAX_REMOTES) {
            let (fetch, push) = tokio::join!(
                read_endpoints(exe, cwd, name, false),
                read_endpoints(exe, cwd, name, true)
            );
            let (fetch, fetch_partial) = fetch.unwrap_or((Vec::new(), true));
            let (push, push_partial) = push.unwrap_or((Vec::new(), true));
            partial |= fetch_partial || push_partial;
            remotes.push(Remote {
                name: name.into(),
                fetch,
                push,
            });
        }
    })
    .await;
    (remotes, partial || finished.is_err())
}

async fn tree(exe: &Path, cwd: &Path) -> Option<Tree> {
    // Inspect the index and untracked entries without refreshing tracked file contents.
    // A normal status refresh can run clean/process filters from Git configuration.
    let (staged, untracked, conflicts, unstaged) = tokio::join!(
        run(
            exe,
            cwd,
            &[
                "diff",
                "--cached",
                "--no-ext-diff",
                "--no-textconv",
                "--no-renames",
                "--diff-filter=ACDMRT",
                "--name-only",
                "-z",
                "--"
            ]
        ),
        run(
            exe,
            cwd,
            &["ls-files", "--others", "--exclude-standard", "-z", "--"]
        ),
        run(exe, cwd, &["ls-files", "--unmerged", "-z", "--"]),
        unstaged(exe, cwd),
    );
    let records = |result: Result<(std::process::ExitStatus, Vec<u8>), InventoryError>| {
        result.ok().and_then(|(status, output)| {
            (status.success() && (output.is_empty() || output.ends_with(b"\0"))).then_some(output)
        })
    };
    let conflicts = records(conflicts)?;
    let paths: BTreeSet<_> = conflicts
        .split(|byte| *byte == 0)
        .filter_map(|line| line.splitn(2, |byte| *byte == b'\t').nth(1))
        .collect();
    let count = |out: Vec<u8>| out.iter().filter(|byte| **byte == 0).count();
    Some(Tree {
        staged: count(records(staged)?),
        untracked: count(records(untracked)?),
        conflicts: paths.len(),
        unstaged,
    })
}

async fn unstaged(exe: &Path, cwd: &Path) -> Option<usize> {
    let (tracked, index, objects, format) = tokio::join!(
        run(exe, cwd, &["ls-files", "--cached", "-z", "--"]),
        line(
            exe,
            cwd,
            &["rev-parse", "--path-format=absolute", "--git-path", "index"]
        ),
        line(
            exe,
            cwd,
            &[
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                "objects"
            ]
        ),
        line(exe, cwd, &["rev-parse", "--show-object-format"]),
    );
    let (status, tracked) = tracked.ok()?;
    if !status.success() {
        return None;
    }
    if tracked.is_empty() {
        return Some(0);
    }
    let (status, attributes) = run_with_input(
        exe,
        cwd,
        &["check-attr", "-z", "--stdin", "filter"],
        Some(&tracked),
    )
    .await
    .ok()?;
    if !status.success() || !attributes.ends_with(b"\0") {
        return None;
    }
    let mut fields = attributes[..attributes.len() - 1].split(|byte| *byte == 0);
    while fields.next().is_some() {
        if fields.next()? != b"filter" || !matches!(fields.next()?, b"unspecified" | b"unset") {
            return None;
        }
    }
    let index = PathBuf::from(index?);
    let objects = PathBuf::from(objects?);
    if !index.is_absolute() || !objects.is_absolute() {
        return None;
    }
    let config = match format?.as_str() {
        "sha1" => "[core]\nrepositoryformatversion = 0\n",
        "sha256" => "[core]\nrepositoryformatversion = 1\n[extensions]\nobjectFormat = sha256\n",
        _ => return None,
    };
    let scratch = tempfile::Builder::new()
        .prefix("gitpilot-index-")
        .tempdir()
        .ok()?;
    let dir = scratch.path();
    fs::create_dir(dir.join("refs")).await.ok()?;
    fs::create_dir(dir.join("objects")).await.ok()?;
    fs::write(dir.join("HEAD"), b"ref: refs/heads/inspection\n")
        .await
        .ok()?;
    fs::write(dir.join("config"), config).await.ok()?;
    let (status, settings) = run(
        exe,
        cwd,
        &[
            "config",
            "--null",
            "--get-regexp",
            r"^core\.(autocrlf|eol|filemode|checkstat|trustctime|symlinks|ignorecase)$",
        ],
    )
    .await
    .ok()?;
    if !status.success() && (status.code() != Some(1) || !settings.is_empty()) {
        return None;
    }
    let mut command = git::isolated(exe, dir);
    command
        .env("GIT_DIR", dir)
        .env("GIT_WORK_TREE", cwd)
        .env("GIT_INDEX_FILE", &index)
        .env("GIT_OBJECT_DIRECTORY", objects)
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
        ]);
    for entry in std::str::from_utf8(&settings).ok()?.split_terminator('\0') {
        let (key, value) = entry.split_once('\n').unwrap_or((entry, "true"));
        let value = value.to_ascii_lowercase();
        if !matches!(
            value.as_str(),
            "true"
                | "false"
                | "yes"
                | "no"
                | "on"
                | "off"
                | "0"
                | "1"
                | "input"
                | "lf"
                | "crlf"
                | "native"
                | "default"
                | "minimal"
        ) {
            return None;
        }
        command.arg("-c").arg(format!("{key}={value}"));
    }
    let (status, output) = git::output(
        command.args([
            "diff-files",
            "--name-only",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "--ignore-submodules=all",
            "--diff-filter=ACDMRT",
            "-z",
            "--",
        ]),
        GIT_TIME,
    )
    .await
    .ok()?;
    if !status.success() || (!output.is_empty() && !output.ends_with(b"\0")) {
        return None;
    }
    Some(output.iter().filter(|byte| **byte == 0).count())
}

fn https(config: &Settings) -> Https {
    let mut helpers = Vec::new();
    let mut scoped_helpers = false;
    for (key, value) in &config.0 {
        if key == "credential.helper" {
            if value.is_empty() {
                helpers.clear();
                continue;
            }
            let helper = match value.split_whitespace().next() {
                Some("manager" | "manager-core") => "manager",
                Some("wincred") => "wincred",
                Some("libsecret") => "libsecret",
                Some("cache") => "cache",
                Some("store") => "store",
                _ => "custom",
            };
            if !helpers.contains(&helper) {
                helpers.push(helper);
            }
        } else if key.starts_with("credential.") && key.ends_with(".helper") {
            scoped_helpers = true;
        }
    }
    Https {
        helpers,
        scoped_helpers,
        askpass_configured: config
            .last("core.askpass")
            .is_some_and(|value| !value.is_empty())
            || env::var_os("GIT_ASKPASS").is_some()
            || env::var_os("SSH_ASKPASS").is_some(),
    }
}

async fn signing(exe: &Path, cwd: &Path, config: &Settings, ssh_dir: Option<&Path>) -> Signing {
    let enabled = if config.last("commit.gpgsign").is_none() {
        Some(false)
    } else {
        line(
            exe,
            cwd,
            &["config", "--type=bool", "--get", "commit.gpgsign"],
        )
        .await
        .and_then(|value| match value.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        })
    };
    let (format, program, override_key) = match config.last("gpg.format") {
        None | Some("openpgp") => ("openpgp", "gpg", "gpg.openpgp.program"),
        Some("ssh") => ("ssh", "ssh-keygen", "gpg.ssh.program"),
        Some("x509") => ("x509", "gpgsm", "gpg.x509.program"),
        _ => ("unknown", "", ""),
    };
    let key_reference = match config.last("user.signingkey") {
        Some(key) if format == "ssh" && (key.starts_with("key::") || key.starts_with("ssh-")) => {
            "public"
        }
        Some(key) if !key.is_empty() && format == "ssh" => "file",
        Some(key) if !key.is_empty() => "reference",
        _ if format == "ssh" && config.last("gpg.ssh.defaultkeycommand").is_some() => "command",
        _ => "default",
    };
    let key_file = if key_reference == "file" {
        Some(signing_key_file(exe, cwd, ssh_dir).await)
    } else {
        None
    };
    Signing {
        enabled,
        format,
        key_reference,
        key_file,
        custom_program: config.last(override_key).is_some()
            || (format == "openpgp" && config.last("gpg.program").is_some()),
        default_program_available: !program.is_empty() && git::find_executable(program).is_some(),
    }
}

async fn signing_key_file(exe: &Path, cwd: &Path, ssh_dir: Option<&Path>) -> &'static str {
    let Some(dir) = ssh_dir else {
        return "unchecked";
    };
    let Some(path) = line(exe, cwd, &["config", "--path", "--get", "user.signingkey"]).await else {
        return "unchecked";
    };
    let Ok(filename) = Path::new(&path).strip_prefix(dir) else {
        return "unchecked";
    };
    let Some(filename) = filename.to_str() else {
        return "unchecked";
    };
    let parent = dir.join("config");
    let ready = tokio::time::timeout(Duration::from_secs(2), async {
        if !tauri::async_runtime::spawn_blocking(move || native_file::parent_id(&parent))
            .await
            .is_ok_and(|result| result.is_ok())
        {
            return "unchecked";
        }
        let mut keys = Keys::default();
        key_info(dir, filename, &mut keys, true).await;
        match (keys.present, keys.missing, keys.restricted) {
            (1, 0, 1) => "permissions",
            (1, 0, _) => "present",
            (_, 1, _) => "missing",
            _ => "unchecked",
        }
    })
    .await;
    ready.unwrap_or("unchecked")
}

fn ssh_directory(value: &str) -> Option<PathBuf> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return None;
    }
    #[cfg(windows)]
    if !matches!(path.components().next(), Some(std::path::Component::Prefix(p)) if matches!(p.kind(), std::path::Prefix::Disk(_)))
    {
        return None;
    }
    Some(path)
}

async fn key_info(dir: &Path, filename: &str, keys: &mut Keys, explicit: bool) {
    if clean(filename, 128).is_none()
        || !filename
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-_".contains(&byte))
        || matches!(filename, "." | "..")
    {
        keys.unchecked += 1;
        return;
    }
    match fs::symlink_metadata(dir.join(filename)).await {
        Ok(metadata) if metadata.is_file() => {
            keys.present += 1;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if !filename.ends_with(".pub") && metadata.permissions().mode() & 0o077 != 0 {
                    keys.restricted += 1;
                }
            }
        }
        Ok(_) => keys.unchecked += 1,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if explicit {
                keys.missing += 1;
            }
        }
        Err(_) => keys.unchecked += 1,
    }
}

async fn ssh(config: &Settings, remotes: &[Remote], ssh_dir: Option<&Path>) -> Ssh {
    let mut result = Ssh {
        client_available: git::find_executable("ssh").is_some(),
        custom_command: config.last("core.sshcommand").is_some()
            || env::var_os("GIT_SSH_COMMAND").is_some()
            || env::var_os("GIT_SSH").is_some(),
        agent_socket: env::var_os("SSH_AUTH_SOCK").is_some_and(|value| !value.is_empty()),
        config_status: "unavailable",
        aliases: Vec::new(),
        default_keys: Keys::default(),
    };
    let Some(dir) = ssh_dir else {
        return result;
    };
    let hosts: BTreeSet<_> = remotes
        .iter()
        .flat_map(|remote| remote.fetch.iter().chain(&remote.push))
        .filter(|endpoint| endpoint.transport == "ssh")
        .filter_map(|endpoint| endpoint.host.as_deref())
        .collect();
    let checked = tokio::time::timeout(Duration::from_secs(2), async {
        let directory = dir.to_owned();
        match tauri::async_runtime::spawn_blocking(move || {
            native_file::parent_id(&directory.join("config"))
        })
        .await
        {
            Ok(Ok(_)) => {}
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                result.config_status = "absent";
                return;
            }
            _ => return,
        }
        for filename in [
            "id_ed25519",
            "id_ed25519_sk",
            "id_ecdsa",
            "id_ecdsa_sk",
            "id_rsa",
        ] {
            key_info(dir, filename, &mut result.default_keys, false).await;
        }
        let path = dir.join("config");
        let loaded =
            tauri::async_runtime::spawn_blocking(move || native_file::snapshot(&path)).await;
        let bytes = match loaded {
            Ok(Ok(Some(snapshot))) if snapshot.bytes.len() <= 32_768 => snapshot.bytes,
            Ok(Ok(None)) => {
                result.config_status = "absent";
                return;
            }
            _ => return,
        };
        let Ok(text) = std::str::from_utf8(&bytes) else {
            return;
        };
        if let Some(entries) = simple_aliases(text, &hosts) {
            result.config_status = "simple";
            for (name, host, filenames) in entries {
                let mut keys = Keys::default();
                for filename in filenames.into_iter().take(16) {
                    if let Some(filename) = filename.strip_prefix("~/.ssh/") {
                        key_info(dir, filename, &mut keys, true).await;
                    } else {
                        keys.unchecked += 1;
                    }
                }
                result.aliases.push(Alias { name, host, keys });
            }
        } else {
            result.config_status = "complex";
        }
    })
    .await;
    if checked.is_err() {
        result.config_status = "unavailable";
    }
    result
}

type AliasEntry<'a> = (String, Option<String>, Vec<&'a str>);

fn simple_aliases<'a>(text: &'a str, hosts: &BTreeSet<&str>) -> Option<Vec<AliasEntry<'a>>> {
    let mut result: Vec<AliasEntry<'a>> = Vec::new();
    let mut active = Vec::new();
    let mut host_block = false;
    for raw in text.lines() {
        let line = raw.split('#').next()?.trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line.split_once(|ch: char| ch.is_ascii_whitespace() || ch == '=')?;
        let value = value.trim_start_matches(|ch: char| ch.is_ascii_whitespace() || ch == '=');
        let key = key.to_ascii_lowercase();
        match key.as_str() {
            "match"
            | "include"
            | "proxycommand"
            | "proxyjump"
            | "identityagent"
            | "canonicalizehostname" => return None,
            "host" => {
                host_block = true;
                active.clear();
                for candidate in value.split_whitespace() {
                    let name = public_host(candidate)?;
                    if hosts.contains(name.as_str()) {
                        let position = result
                            .iter()
                            .position(|(existing, ..)| *existing == name)
                            .unwrap_or_else(|| {
                                result.push((name, None, Vec::new()));
                                result.len() - 1
                            });
                        active.push(position);
                    }
                }
            }
            "hostname" => {
                if !host_block {
                    return None;
                }
                let hostname = public_host(value)?;
                for index in &active {
                    if result[*index].1.is_none() {
                        result[*index].1 = Some(hostname.clone());
                    }
                }
            }
            "identityfile" => {
                if !host_block {
                    return None;
                }
                for index in &active {
                    result[*index].2.push(value);
                }
            }
            _ => {}
        }
    }
    Some(result)
}

pub(super) async fn context(repository: &Repository) -> Result<(PathBuf, &Path), InventoryError> {
    if !repository.present {
        return Err(InventoryError::NotFound);
    }
    let cwd = Path::new(&repository.path);
    if discovery::directory_path(cwd).await? != repository.path {
        return Err(InventoryError::GitReadFailed);
    }
    let exe = git::executable().ok_or(InventoryError::GitUnavailable)?;
    let git_dir = line(&exe, cwd, &["rev-parse", "--absolute-git-dir"])
        .await
        .ok_or(InventoryError::GitReadFailed)?;
    if discovery::directory_path(Path::new(&git_dir)).await? != repository.git_dir {
        return Err(InventoryError::GitReadFailed);
    }
    Ok((exe, cwd))
}

pub(crate) async fn inspect(repository: &Repository) -> Result<Inspector, InventoryError> {
    let (exe, cwd) = context(repository).await?;
    let (config, home) = tokio::join!(
        settings(&exe, cwd),
        line(
            &exe,
            cwd,
            &[
                "-c",
                "gitpilot.sshdirectory=~/.ssh",
                "config",
                "--path",
                "--get",
                "gitpilot.sshdirectory"
            ]
        ),
    );
    let config = config?;
    let ssh_dir = home.as_deref().and_then(ssh_directory);
    let (branch, head, tree, (remotes, remotes_partial), signing) = tokio::join!(
        line(&exe, cwd, &["symbolic-ref", "--quiet", "--short", "HEAD"]),
        line(&exe, cwd, &["rev-parse", "--verify", "--short", "HEAD"]),
        async {
            if repository.kind == "bare" {
                None
            } else {
                tree(&exe, cwd).await
            }
        },
        remotes(&exe, cwd, &config),
        signing(&exe, cwd, &config, ssh_dir.as_deref()),
    );
    let https = https(&config);
    let ssh = ssh(&config, &remotes, ssh_dir.as_deref()).await;
    Ok(Inspector {
        branch,
        head,
        tree,
        remotes,
        remotes_partial,
        ssh,
        https,
        signing,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fixture_git(exe: &Path, cwd: &Path, global: &Path, args: &[&str]) {
        let mut command = git::command(exe, cwd);
        command
            .env("GIT_CONFIG_GLOBAL", global)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args(args);
        let (status, _) = git::output(&mut command, Duration::from_secs(5))
            .await
            .unwrap();
        assert!(status.success(), "fixture Git failed for {}", args[0]);
    }

    #[test]
    fn redacts_urls_and_only_reports_simple_ssh_aliases() {
        for (url, transport, host) in [
            (
                "https://account:secret@example.test/private?token=secret",
                "https",
                Some("example.test"),
            ),
            ("git@corp:private/repo", "ssh", Some("corp")),
            ("ssh://account@[::1]:2222/private", "ssh", Some("::1")),
            ("ext::git secret", "other", None),
            ("C:/path/private", "local", None),
        ] {
            let endpoint = endpoint(url);
            assert_eq!(endpoint.transport, transport);
            assert_eq!(endpoint.host.as_deref(), host);
        }
        let hosts = BTreeSet::from(["corp"]);
        let aliases = simple_aliases(
            "Host corp other\n HostName git.example.test\n IdentityFile ~/.ssh/corp_ed25519\nHost unrelated\n HostName other.test",
            &hosts,
        ).unwrap();
        assert_eq!(
            aliases,
            [(
                "corp".into(),
                Some("git.example.test".into()),
                vec!["~/.ssh/corp_ed25519"]
            )]
        );
        assert!(simple_aliases("Host corp\nMatch exec \"touch marker\"", &hosts).is_none());
        assert!(simple_aliases("Include ~/.ssh/hosts\nHost corp", &hosts).is_none());
    }

    #[test]
    fn native_git_inspector_reads_without_executing_configured_programs() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let path = discovery::directory_path(temp.path()).await.unwrap();
            let cwd = Path::new(&path);
            let exe = git::executable().unwrap();
            let global = cwd.join("fixture-config");
            fixture_git(
                &exe,
                cwd,
                &global,
                &["init", "--quiet", "--initial-branch=inspector-test"],
            )
            .await;
            fs::write(cwd.join("tracked"), "one").await.unwrap();
            fixture_git(&exe, cwd, &global, &["add", "tracked"]).await;
            fs::write(cwd.join("tracked"), "two").await.unwrap();
            assert_eq!(unstaged(&exe, cwd).await, Some(1));
            fs::write(cwd.join(".gitattributes"), "tracked filter=external\n")
                .await
                .unwrap();
            for (key, value) in [
                (
                    "remote.origin.url",
                    "https://user:secret@example.test/secret",
                ),
                ("remote.origin.pushurl", "git@corp:secret/repo"),
                (
                    "url.https://mirror.test/.insteadOf",
                    "https://user:secret@example.test/",
                ),
                ("credential.helper", ""),
                ("credential.helper", "manager --secret-option"),
                ("credential.helper", "!touch should-not-run"),
                ("credential.https://example.test.helper", "libsecret"),
                ("core.fsmonitor", "touch should-not-run"),
                ("filter.external.clean", "touch should-not-run"),
                ("core.sshCommand", "touch should-not-run"),
                ("commit.gpgSign", "yes"),
                ("gpg.format", "ssh"),
                ("gpg.ssh.program", "touch should-not-run"),
                ("user.signingkey", "key::ssh-ed25519 secret-identity"),
            ] {
                fixture_git(
                    &exe,
                    cwd,
                    &global,
                    &["config", "--local", "--add", key, value],
                )
                .await;
            }
            let git_dir = discovery::directory_path(&cwd.join(".git")).await.unwrap();
            let config = cwd.join(".git/config");
            let index = cwd.join(".git/index");
            let config_bytes = fs::read(&config).await.unwrap();
            let index_bytes = fs::read(&index).await.unwrap();
            let report = inspect(&Repository {
                id: 1,
                path: path.clone(),
                kind: "ordinary".into(),
                git_dir: git_dir.clone(),
                common_dir: git_dir,
                revision: 1,
                present: true,
                identity_id: None,
            })
            .await
            .unwrap();
            assert_eq!(report.branch.as_deref(), Some("inspector-test"));
            assert_eq!(report.tree.as_ref().unwrap().staged, 1);
            assert_eq!(report.tree.as_ref().unwrap().unstaged, None);
            assert!(report.ssh.custom_command);
            assert!(report.https.scoped_helpers);
            assert_eq!(report.https.helpers, ["manager", "custom"]);
            assert_eq!(report.signing.enabled, Some(true));
            assert_eq!(report.signing.key_reference, "public");
            assert_eq!(
                report.remotes[0].fetch[0].host.as_deref(),
                Some("mirror.test")
            );
            assert_eq!(report.remotes[0].push[0].host.as_deref(), Some("corp"));
            let safe = serde_json::to_string(&report).unwrap();
            assert!(!safe.contains("secret-identity"));
            assert!(!safe.contains("secret-option"));
            assert!(!safe.contains("should-not-run"));
            assert!(!cwd.join("should-not-run").exists());
            assert_eq!(fs::read(config).await.unwrap(), config_bytes);
            assert_eq!(fs::read(index).await.unwrap(), index_bytes);
        });
    }
}
