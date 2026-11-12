use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::Serialize;
use sqlx::SqlitePool;
use tokio::fs;

use crate::repository::{InventoryError, ObservedIdentity, Repository};

use super::{discovery, git, native_file};

const PREVIEW_LIMIT: usize = 80;
const GIT_TIME: Duration = Duration::from_secs(2);

#[derive(Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub(super) struct Profile {
    id: i64,
    revision: i64,
    label: String,
    git_name: String,
    email: String,
    signing_key: Option<String>,
    signing_format: Option<String>,
    ssh_key: Option<String>,
    signing_required: bool,
}

impl Profile {
    pub(super) async fn check_key_files(&self) -> Result<(), super::config_apply::ApplyError> {
        for (key, inline) in self.ssh_key.iter().map(|key| (key, false)).chain(
            self.signing_key
                .iter()
                .filter(|_| self.signing_format.as_deref().unwrap_or("ssh") == "ssh")
                .map(|key| (key, true)),
        ) {
            if inline && (key.starts_with("key::") || key.starts_with("ssh-")) {
                continue;
            }
            let path = if let Some(relative) =
                key.strip_prefix("~/").or_else(|| key.strip_prefix("~\\"))
            {
                PathBuf::from(
                    env::var_os("HOME")
                        .or_else(|| env::var_os("USERPROFILE"))
                        .ok_or(super::config_apply::ApplyError::KeyUnavailable)?,
                )
                .join(relative)
            } else {
                PathBuf::from(key)
            };
            if !path.is_absolute() || !fs::metadata(&path).await.is_ok_and(|meta| meta.is_file()) {
                return Err(super::config_apply::ApplyError::KeyUnavailable);
            }
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct Root {
    revision: i64,
    path: String,
    scan_partial: Option<bool>,
    identity_id: Option<i64>,
}

struct Policy {
    roots: Vec<Root>,
    repositories: Vec<Repository>,
    profiles: BTreeMap<i64, Profile>,
    default_id: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Rule {
    path: String,
    source: &'static str,
    pattern: String,
    profile: Profile,
    profile_path: String,
}

#[derive(Serialize)]
struct Finding {
    code: &'static str,
    path: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Comparison {
    path: String,
    expected: Option<String>,
    current: ObservedIdentity,
    proposed: ObservedIdentity,
    result: &'static str,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigPreview {
    git_version: String,
    global_path: Option<String>,
    global_append: Option<String>,
    manifest_path: String,
    rules: Vec<Rule>,
    comparisons: Vec<Comparison>,
    findings: Vec<Finding>,
    confirmation: Option<String>,
    #[serde(skip)]
    pub(super) candidate: Option<Candidate>,
}

pub(super) const MANAGED: &str = "# GitPilot managed identity configuration";

pub(super) struct Candidate {
    pub(super) target: PathBuf,
    pub(super) parent: [u64; 2],
    pub(super) before: Option<native_file::Stamp>,
    pub(super) generated: Option<(PathBuf, BTreeMap<String, Vec<u8>>)>,
    pub(super) append: String,
}

impl ConfigPreview {
    fn warn(&mut self, code: &'static str, path: Option<&str>) {
        self.findings.push(Finding {
            code,
            path: path.map(str::to_owned),
        });
    }
}

async fn snapshot(pool: &SqlitePool) -> Result<Policy, InventoryError> {
    let mut tx = pool.begin().await?;
    let roots = sqlx::query_as(
        "SELECT d.path, d.revision, d.scan_partial, r.identity_id FROM directories d
         LEFT JOIN identity_rules r ON r.directory_id = d.id ORDER BY d.path",
    )
    .fetch_all(&mut *tx)
    .await?;
    let repositories = sqlx::query_as(
        "SELECT p.id, p.path, p.kind, p.git_dir, p.common_dir, p.revision, r.identity_id,
                EXISTS (SELECT 1 FROM directory_repositories dr WHERE dr.repository_id = p.id
                        AND dr.present = 1) AS present
         FROM repositories p LEFT JOIN identity_rules r ON r.repository_id = p.id ORDER BY p.path",
    )
    .fetch_all(&mut *tx)
    .await?;
    let profiles: Vec<Profile> =
        sqlx::query_as("SELECT id, revision, label, git_name, email, signing_key, signing_format, ssh_key, signing_required FROM identities")
            .fetch_all(&mut *tx)
            .await?;
    let default_id = sqlx::query_scalar("SELECT id FROM identities WHERE is_default = 1")
        .fetch_optional(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Policy {
        roots,
        repositories,
        profiles: profiles.into_iter().map(|p| (p.id, p)).collect(),
        default_id,
    })
}

fn git_path(path: &Path) -> Option<String> {
    let value = path.to_str()?;
    if !path.is_absolute() || value.chars().any(char::is_control) {
        return None;
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        if !matches!(path.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_)))
        {
            return None;
        }
        Some(value.replace('\\', "/"))
    }
    #[cfg(not(windows))]
    Some(value.to_owned())
}

fn literal_gitdir(path: &Path) -> Option<String> {
    let mut pattern = String::from("gitdir:");
    for character in git_path(path)?.chars() {
        match character {
            '[' => pattern.push_str("[[]"),
            '*' => pattern.push_str("[*]"),
            '?' => pattern.push_str("[?]"),
            '\\' => pattern.push_str(r"\\"),
            _ => pattern.push(character),
        }
    }
    Some(pattern)
}

fn plan_rules(policy: &Policy, native: &Path, preview: &mut ConfigPreview) {
    let mut roots: Vec<_> = policy.roots.iter().collect();
    roots.sort_by_key(|root| Path::new(&root.path).components().count());
    for root in roots {
        let path = Path::new(&root.path);
        let inherited = policy.roots.iter().any(|other| {
            other.identity_id.is_some() && other.path != root.path && path.starts_with(&other.path)
        });
        let choice = match root.identity_id {
            Some(id) => Some((id, "directory")),
            None if !inherited => policy.default_id.map(|id| (id, "default")),
            None => None,
        };
        if let Some((id, source)) = choice {
            if path.parent().is_none() {
                preview.warn("unsupported_path", Some(&root.path));
                continue;
            }
            if root.scan_partial != Some(false) {
                preview.warn("unverified_directory", Some(&root.path));
            }
            add_rule(policy, native, preview, &root.path, path, id, source);
        }
    }
    for repo in &policy.repositories {
        if let Some(id) = repo.identity_id {
            let path = Path::new(&repo.path);
            let git_dir = Path::new(&repo.git_dir);
            if repo.present && repo.kind == "ordinary" && path.join(".git") == git_dir {
                add_rule(
                    policy,
                    native,
                    preview,
                    &repo.path,
                    git_dir,
                    id,
                    "repository",
                );
            } else {
                preview.warn("unsupported_exception", Some(&repo.path));
            }
        }
    }
}

fn add_rule(
    policy: &Policy,
    native: &Path,
    preview: &mut ConfigPreview,
    path: &str,
    match_path: &Path,
    id: i64,
    source: &'static str,
) {
    let Some(mut pattern) = literal_gitdir(match_path) else {
        preview.warn("unsupported_path", Some(path));
        return;
    };
    if source != "repository" {
        if !pattern.ends_with('/') {
            pattern.push('/');
        }
        pattern.push_str("**/.git");
    }
    if let Some(profile) = policy.profiles.get(&id) {
        preview.rules.push(Rule {
            path: path.into(),
            source,
            pattern,
            profile: profile.clone(),
            profile_path: native
                .join(profile_filename(profile))
                .to_string_lossy()
                .into_owned(),
        });
    }
}

fn profile_filename(profile: &Profile) -> String {
    format!("profile-{}-{}.config", profile.id, profile.revision)
}

pub(super) fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn include(path: &str) -> String {
    format!("[include]\n\tpath = {}\n", quote(path))
}

pub(super) fn profile_config(profile: &Profile) -> String {
    let format = profile.signing_format.as_deref().unwrap_or("ssh");
    let key = profile.signing_key.as_deref().unwrap_or("");
    let key = if format == "ssh" {
        key.replace('\\', "/")
    } else {
        key.into()
    };
    let mut ssh = String::from("ssh -F none -o IdentitiesOnly=yes -o IdentityAgent=none -o IdentityFile=none -o CertificateFile=none -o PreferredAuthentications=publickey");
    if let Some(key) = profile
        .ssh_key
        .as_deref()
        .filter(|key| !key.trim().is_empty())
    {
        // IdentityFile is expanded by OpenSSH after parsing; unlike -i, it does
        // not discard a literal-percent path during an early filesystem check.
        let identity = format!(
            "IdentityFile={}",
            quote(&key.replace('\\', "/").replace('%', "%%"))
        );
        ssh.push_str(&format!(" -o '{}'", identity.replace('\'', "'\\''")));
    } else {
        ssh.push_str(" -o PubkeyAuthentication=no");
    }
    format!(
        "[user]\n\tname = {}\n\temail = {}\n\tsigningkey = {}\n[commit]\n\tgpgsign = {}\n[tag]\n\tgpgsign = false\n[gpg]\n\tformat = {}\n\tprogram = gpg\n[gpg \"ssh\"]\n\tprogram = ssh-keygen\n\tdefaultKeyCommand =\n[gpg \"openpgp\"]\n\tprogram = gpg\n[core]\n\tsshCommand = {}\n[ssh]\n\tvariant = ssh\n",
        quote(&profile.git_name), quote(&profile.email), quote(&key),
        profile.signing_required && !key.is_empty(), quote(format), quote(&ssh)
    )
}

pub(super) async fn expected_profile(
    pool: &SqlitePool,
    repo: &Repository,
) -> Result<Profile, InventoryError> {
    let id: Option<i64> = if let Some(id) = repo.identity_id {
        Some(id)
    } else {
        let rules: Vec<(String, i64)> = sqlx::query_as("SELECT d.path, r.identity_id FROM directories d JOIN identity_rules r ON r.directory_id = d.id").fetch_all(pool).await?;
        match rules
            .into_iter()
            .filter(|(path, _)| Path::new(&repo.path).starts_with(path))
            .max_by_key(|(path, _)| Path::new(path).components().count())
        {
            Some((_, id)) => Some(id),
            None => {
                sqlx::query_scalar("SELECT id FROM identities WHERE is_default = 1")
                    .fetch_optional(pool)
                    .await?
            }
        }
    };
    sqlx::query_as("SELECT id, revision, label, git_name, email, signing_key, signing_format, ssh_key, signing_required FROM identities WHERE id = ?").bind(id.ok_or(InventoryError::NotFound)?).fetch_one(pool).await.map_err(Into::into)
}

fn generated_files(rules: &[Rule]) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut manifest = String::new();
    for rule in rules {
        let filename = profile_filename(&rule.profile);
        files
            .entry(filename.clone())
            .or_insert_with(|| profile_config(&rule.profile).into_bytes());
        manifest.push_str(&format!(
            "[includeIf {}]\n\tpath = {}\n",
            quote(&rule.pattern),
            quote(&filename)
        ));
    }
    files.insert("active.config".into(), manifest.into_bytes());
    files
}

pub(super) async fn git_line(executable: &Path, cwd: &Path, args: &[&str]) -> Option<String> {
    let mut command = git::command(executable, cwd);
    let (status, bytes) = git::output(command.args(args), GIT_TIME).await.ok()?;
    let output = String::from_utf8(bytes).ok()?;
    let line = output.trim_end_matches(['\r', '\n']);
    (status.success() && !line.is_empty() && !line.chars().any(char::is_control))
        .then(|| line.into())
}

pub(super) fn compatible(version: &str) -> bool {
    let Some(version) = version.strip_prefix("git version ") else {
        return false;
    };
    let mut parts = version
        .split('.')
        .map_while(|part| part.parse::<u32>().ok());
    matches!((parts.next(), parts.next(), parts.next()), (Some(a), Some(b), Some(c)) if (a, b, c) >= (2, 39, 5))
}

pub(crate) async fn version_status(directory: &Path) -> (Option<String>, bool) {
    let version = match git::executable() {
        Some(executable) => git_line(&executable, directory, &["--version"]).await,
        None => None,
    };
    let supported = version.as_deref().is_some_and(compatible);
    (version, supported)
}

struct GlobalFiles {
    destination: PathBuf,
    existing: Vec<PathBuf>,
}

async fn global_files(home: &Path, xdg: &Path) -> Option<GlobalFiles> {
    let mut existing = Vec::new();
    for candidate in [xdg, home] {
        git_path(candidate)?;
        match fs::symlink_metadata(candidate).await {
            Ok(meta) if meta.file_type().is_file() => {
                let actual = fs::canonicalize(candidate).await.ok()?;
                #[cfg(windows)]
                let actual = windows_display_path(&actual);
                if actual != candidate {
                    return None;
                }
                existing.push(candidate.to_owned());
            }
            Ok(_) => return None,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return None,
        }
    }
    Some(GlobalFiles {
        destination: existing
            .last()
            .map_or_else(|| home.to_owned(), PathBuf::clone),
        existing,
    })
}

#[cfg(windows)]
fn windows_display_path(path: &Path) -> PathBuf {
    path.to_str()
        .and_then(|path| path.strip_prefix(r"\\?\"))
        .map_or_else(|| path.to_owned(), PathBuf::from)
}

async fn simulation(
    rules: &[Rule],
    files: &GlobalFiles,
    scratch: &Path,
) -> Result<PathBuf, InventoryError> {
    for (filename, bytes) in generated_files(rules) {
        fs::write(scratch.join(filename), bytes)
            .await
            .map_err(|_| InventoryError::StorageUnavailable)?;
    }
    let mut global = String::new();
    for path in &files.existing {
        global.push_str(&include(
            &git_path(path).ok_or(InventoryError::InvalidPath)?,
        ));
    }
    global.push_str(&include("active.config"));
    let path = scratch.join("global.config");
    fs::write(&path, global)
        .await
        .map_err(|_| InventoryError::StorageUnavailable)?;
    Ok(path)
}

fn expected_rule<'a>(repo: &Repository, rules: &'a [Rule]) -> Option<&'a Rule> {
    rules.iter().rev().find(|rule| {
        if rule.source == "repository" {
            rule.path == repo.path
        } else {
            Path::new(&repo.path).starts_with(&rule.path)
        }
    })
}

async fn compare(
    executable: &Path,
    path: &Path,
    global: &Path,
    expected: Option<&Profile>,
    ordinary: bool,
) -> Result<Comparison, InventoryError> {
    let current = discovery::read_config(executable, path, None).await?;
    let proposed = discovery::read_config(executable, path, Some(global)).await?;
    let same = current.name == proposed.name && current.email == proposed.email;
    let matches = expected.is_some_and(|profile| {
        proposed
            .name
            .as_ref()
            .is_some_and(|field| field.value == profile.git_name)
            && proposed
                .email
                .as_ref()
                .is_some_and(|field| field.value == profile.email)
    });
    let result = match (ordinary, expected, same, matches) {
        (false, _, false, _) | (true, None, false, _) => "unexpected",
        (false, _, true, _) => "diagnostic_only",
        (_, None, _, _) => "unchanged",
        (_, Some(_), _, true) => "matches",
        (_, Some(_), _, false) => "conflict",
    };
    Ok(Comparison {
        path: path.to_string_lossy().into_owned(),
        expected: expected.map(|profile| profile.label.clone()),
        current,
        proposed,
        result,
    })
}

pub(super) async fn worktrees(
    executable: &Path,
    path: &Path,
) -> Result<Vec<String>, InventoryError> {
    let mut command = git::command(executable, path);
    command.args(["worktree", "list", "--porcelain", "-z"]);
    let (status, output) = git::output(&mut command, GIT_TIME)
        .await
        .map_err(|_| InventoryError::GitReadFailed)?;
    let text = std::str::from_utf8(&output).map_err(|_| InventoryError::GitReadFailed)?;
    let mut paths = text
        .split_terminator('\0')
        .filter_map(|part| part.strip_prefix("worktree "));
    if !status.success() || paths.next().is_none() {
        return Err(InventoryError::GitReadFailed);
    }
    Ok(paths.map(str::to_owned).collect())
}

fn project_origins(result: &mut Comparison, simulated_paths: &BTreeMap<String, &str>) {
    for item in [&mut result.proposed.name, &mut result.proposed.email]
        .into_iter()
        .flatten()
    {
        let original = item
            .origin
            .strip_prefix("file:")
            .and_then(|path| git_path(Path::new(path)));
        if let Some(path) = original
            .as_ref()
            .and_then(|origin| simulated_paths.get(origin))
        {
            item.origin = format!("file:{path}");
        }
    }
}

pub(crate) async fn prepare(
    pool: &SqlitePool,
    app_data: &Path,
) -> Result<ConfigPreview, InventoryError> {
    let executable = git::executable().ok_or(InventoryError::GitUnavailable)?;
    let git_version = git_line(&executable, app_data, &["--version"])
        .await
        .ok_or(InventoryError::GitReadFailed)?;
    let native = app_data.join("native");
    let mut preview = ConfigPreview {
        git_version,
        manifest_path: native.join("active.config").to_string_lossy().into_owned(),
        ..ConfigPreview::default()
    };
    let policy = snapshot(pool).await?;
    plan_rules(&policy, &native, &mut preview);
    let unavailable: Vec<_> = {
        let mut paths = Vec::new();
        for rule in &preview.rules {
            if rule.profile.check_key_files().await.is_err() {
                paths.push(rule.path.clone());
            }
        }
        paths
    };
    for path in unavailable {
        preview.warn("key_unavailable", Some(&path));
    }
    if !compatible(&preview.git_version) {
        preview.warn("unsupported_git", None);
        return Ok(preview);
    }
    if env::var_os("GIT_CONFIG_GLOBAL").is_some()
        || [
            "GIT_CONFIG",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
            "GIT_DIR",
            "GIT_COMMON_DIR",
            "GIT_WORK_TREE",
        ]
        .iter()
        .any(|name| env::var_os(name).is_some_and(|value| !value.is_empty()))
    {
        preview.warn("redirected_config", None);
        return Ok(preview);
    }
    let Some(home_config) = git_line(
        &executable,
        app_data,
        &[
            "-c",
            "gitpilot.home=~/.gitconfig",
            "config",
            "--path",
            "--get",
            "gitpilot.home",
        ],
    )
    .await
    .map(PathBuf::from) else {
        preview.warn("unknown_global", None);
        return Ok(preview);
    };
    let Some(home) = home_config.parent() else {
        preview.warn("unknown_global", None);
        return Ok(preview);
    };
    let xdg = env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map_or_else(|| home.join(".config"), PathBuf::from)
        .join("git")
        .join("config");
    let Some(files) = tokio::time::timeout(GIT_TIME, global_files(&home_config, &xdg))
        .await
        .ok()
        .flatten()
    else {
        preview.warn("unknown_global", None);
        return Ok(preview);
    };
    preview.global_path = files.destination.to_str().map(str::to_owned);
    if preview.rules.is_empty() {
        return Ok(preview);
    }
    let generated = generated_files(&preview.rules);
    let generation_hash = native_file::digest(
        &serde_json::to_vec(&generated).map_err(|_| InventoryError::StorageUnavailable)?,
    );
    let generation = native.join(hex::encode(generation_hash));
    let manifest = generation.join("active.config");
    let Some(manifest_text) = git_path(&manifest) else {
        preview.warn("unknown_global", None);
        return Ok(preview);
    };
    preview.manifest_path = manifest.to_string_lossy().into_owned();
    for rule in &mut preview.rules {
        rule.profile_path = generation
            .join(profile_filename(&rule.profile))
            .to_string_lossy()
            .into_owned();
    }
    let append = format!("{MANAGED}\n{}", include(&manifest_text));
    preview.global_append = Some(append.clone());
    let external = native_file::parent_id(&files.destination)
        .and_then(|id| native_file::snapshot(&files.destination).map(|snapshot| (id, snapshot)));
    let scratch = tempfile::Builder::new()
        .prefix("native-preview-")
        .tempdir_in(app_data)
        .map_err(|_| InventoryError::StorageUnavailable)?;
    let global = simulation(&preview.rules, &files, scratch.path()).await?;
    let simulated_paths: BTreeMap<_, _> = preview
        .rules
        .iter()
        .filter_map(|rule| {
            git_path(&scratch.path().join(profile_filename(&rule.profile)))
                .map(|origin| (origin, rule.profile_path.as_str()))
        })
        .collect();
    let mut comparisons = Vec::new();
    let mut findings = Vec::new();
    let mut extra_worktrees = BTreeSet::new();
    let finished = tokio::time::timeout(Duration::from_secs(15), async {
        for (index, repo) in policy
            .repositories
            .iter()
            .filter(|repo| repo.present)
            .enumerate()
        {
            if index == PREVIEW_LIMIT {
                findings.push(("comparison_limit", None));
                break;
            }
            let path = Path::new(&repo.path);
            let valid = discovery::directory_path(path)
                .await
                .is_ok_and(|canonical| canonical == repo.path);
            let git_dir = git_line(&executable, path, &["rev-parse", "--absolute-git-dir"]).await;
            let same_git_dir = git_dir
                .and_then(|value| git_path(Path::new(&value)))
                .is_some_and(|value| Some(value) == git_path(Path::new(&repo.git_dir)));
            let ordinary = repo.kind == "ordinary";
            let expected = expected_rule(repo, &preview.rules).map(|rule| &rule.profile);
            if valid && same_git_dir {
                if let Ok(mut result) =
                    compare(&executable, path, &global, expected, ordinary).await
                {
                    project_origins(&mut result, &simulated_paths);
                    comparisons.push(result);
                    if repo.kind != "worktree" {
                        match worktrees(&executable, path).await {
                            Ok(paths) => extra_worktrees.extend(paths),
                            Err(_) => {
                                findings.push(("unverified_worktree", Some(repo.path.clone())))
                            }
                        }
                    }
                    continue;
                }
            }
            findings.push(("unverified_repository", Some(repo.path.clone())));
        }
        for reported in extra_worktrees {
            if comparisons.len() >= PREVIEW_LIMIT {
                findings.push(("comparison_limit", None));
                break;
            }
            let Ok(path) = discovery::directory_path(Path::new(&reported)).await else {
                findings.push(("unverified_worktree", git_path(Path::new(&reported))));
                continue;
            };
            if comparisons.iter().any(|result| result.path == path) {
                continue;
            }
            match compare(&executable, Path::new(&path), &global, None, false).await {
                Ok(mut result) => {
                    project_origins(&mut result, &simulated_paths);
                    comparisons.push(result);
                }
                Err(_) => findings.push(("unverified_worktree", Some(path))),
            }
        }
    })
    .await;
    if finished.is_err() && !findings.iter().any(|(code, _)| *code == "comparison_limit") {
        findings.push(("comparison_limit", None));
    }
    findings.sort_unstable();
    findings.dedup();
    preview.comparisons = comparisons;
    preview.findings.extend(
        findings
            .into_iter()
            .map(|(code, path)| Finding { code, path }),
    );
    if let Ok((parent, before)) = external {
        let managed = before.as_ref().is_some_and(|snapshot| {
            snapshot
                .bytes
                .windows(MANAGED.len())
                .any(|window| window == MANAGED.as_bytes())
        });
        let journal: Option<String> = sqlx::query_scalar("SELECT applied_stamp FROM config_operations WHERE target = ? AND state = 'applied' ORDER BY rowid DESC LIMIT 1").bind(files.destination.to_str()).fetch_optional(pool).await?.flatten();
        let owned = journal
            .as_deref()
            .and_then(|value| serde_json::from_str::<native_file::Stamp>(value).ok())
            .is_some_and(|stamp| before.as_ref().is_some_and(|file| file.stamp == stamp));
        if managed && !owned {
            preview.warn("managed_config_present", None);
        } else if native_file::parent_id(&native).is_err() {
            preview.warn("unsafe_global", None);
        } else if preview
            .findings
            .iter()
            .all(|finding| finding.code == "unverified_directory")
            && !preview
                .comparisons
                .iter()
                .any(|item| item.result == "unexpected")
        {
            let before = before.map(|snapshot| snapshot.stamp);
            let roots: Vec<_> = policy
                .roots
                .iter()
                .map(|root| (&root.path, root.revision))
                .collect();
            let repositories: Vec<_> = policy
                .repositories
                .iter()
                .map(|repo| (repo.id, repo.revision))
                .collect();
            let material = serde_json::to_vec(&(&preview, parent, &before, roots, repositories))
                .map_err(|_| InventoryError::StorageUnavailable)?;
            preview.confirmation = Some(hex::encode(native_file::digest(&material)));
            preview.candidate = Some(Candidate {
                target: files.destination,
                parent,
                before,
                generated: Some((generation, generated)),
                append,
            });
        }
    } else {
        preview.warn("unsafe_global", None);
    }
    Ok(preview)
}

impl ConfigPreview {
    pub(super) fn confirmation(&self) -> Option<&str> {
        self.confirmation.as_deref()
    }

    pub(super) async fn verify(&self) -> Result<(), InventoryError> {
        let executable = git::executable().ok_or(InventoryError::GitUnavailable)?;
        tokio::time::timeout(Duration::from_secs(15), async {
            for item in &self.comparisons {
                let observed =
                    discovery::read_config(&executable, Path::new(&item.path), None).await?;
                if !same_value(observed.name.as_ref(), item.proposed.name.as_ref())
                    || !same_value(observed.email.as_ref(), item.proposed.email.as_ref())
                {
                    return Err(InventoryError::GitReadFailed);
                }
            }
            Ok(())
        })
        .await
        .map_err(|_| InventoryError::GitReadFailed)?
    }
}

fn same_value(
    actual: Option<&crate::repository::ConfigValue>,
    expected: Option<&crate::repository::ConfigValue>,
) -> bool {
    match (actual, expected) {
        (Some(actual), Some(expected)) => {
            let normalize = |origin: &str| {
                origin
                    .strip_prefix("file:")
                    .and_then(|file| git_path(Path::new(file)))
                    .map_or_else(|| origin.to_owned(), |file| format!("file:{file}"))
            };
            actual.value == expected.value
                && actual.scope == expected.scope
                && normalize(&actual.origin) == normalize(&expected.origin)
        }
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires the isolated loopback SSH fixture in scripts/test-profile-remote.sh"]
    fn profile_remote_keys_and_commit_signatures_are_independent() {
        use std::process::Command;
        let root =
            PathBuf::from(env::var_os("GITPILOT_REMOTE_TEST_ROOT").expect("SSH fixture required"));
        let repo = root.join("client");
        let run = |args: &[&str]| {
            Command::new("git")
                .current_dir(&repo)
                .env("HOME", root.join("home"))
                .env("GNUPGHOME", root.join("gnupg"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", root.join("empty.config"))
                .env_remove("GIT_SSH_COMMAND")
                .env_remove("GIT_SSH")
                .args(args)
                .output()
                .unwrap()
        };
        assert!(run(&["init", "--quiet"]).status.success());
        let selected = root.join("key-a");
        let foreign = root.join("key-b");
        let mut policy = profile(1, "SSH fixture");
        policy.git_name = "Fixture Author".into();
        policy.email = "fixture@example.test".into();
        policy.ssh_key = Some(selected.to_str().unwrap().into());
        let port = env::var("GITPILOT_REMOTE_TEST_PORT").unwrap();
        let remote = format!("ssh://root@127.0.0.1:{port}{}/remote.git", root.display());
        let write_policy = |policy: &Profile| {
            std::fs::write(root.join("policy.config"), profile_config(policy)).unwrap();
            assert!(run(&[
                "config",
                "include.path",
                root.join("policy.config").to_str().unwrap()
            ])
            .status
            .success());
        };
        let read_remote = || {
            let output = run(&["config", "core.sshCommand"]);
            assert!(output.status.success());
            let command = format!(
                "core.sshCommand={} -o StrictHostKeyChecking=yes -o UserKnownHostsFile={}",
                String::from_utf8(output.stdout).unwrap().trim(),
                root.join("known_hosts").display()
            );
            run(&["-c", &command, "ls-remote", &remote])
                .status
                .success()
        };
        write_policy(&policy);
        assert!(read_remote(), "assigned key must authenticate");
        policy.ssh_key = Some(foreign.to_str().unwrap().into());
        write_policy(&policy);
        assert!(
            !read_remote(),
            "another profile's key must not authenticate"
        );
        policy.ssh_key = None;
        write_policy(&policy);
        assert!(
            !read_remote(),
            "no key must not fall back to the default key or agent"
        );

        policy.signing_format = Some("ssh".into());
        policy.signing_key = Some(foreign.to_str().unwrap().into());
        policy.signing_required = true;
        write_policy(&policy);
        let output = run(&["commit", "--allow-empty", "-m", "SSH signed fixture"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let allowed = format!(
            "gpg.ssh.allowedSignersFile={}",
            root.join("allowed_signers").display()
        );
        assert!(run(&["-c", &allowed, "verify-commit", "HEAD"])
            .status
            .success());
        assert!(
            !read_remote(),
            "signing key must not become an authentication key"
        );
        policy.signing_required = false;
        write_policy(&policy);
        assert!(
            run(&["commit", "--allow-empty", "-m", "Optional unsigned fixture"])
                .status
                .success()
        );
        assert_eq!(
            String::from_utf8(run(&["log", "-1", "--format=%G?"]).stdout)
                .unwrap()
                .trim(),
            "N"
        );
        assert!(run(&[
            "-c",
            "commit.gpgsign=true",
            "commit",
            "--allow-empty",
            "-m",
            "Explicit SSH signature"
        ])
        .status
        .success());
        assert!(run(&["-c", &allowed, "verify-commit", "HEAD"])
            .status
            .success());

        policy.signing_format = Some("openpgp".into());
        policy.signing_key = Some(env::var("GITPILOT_REMOTE_TEST_GPG_KEY").unwrap());
        policy.signing_required = true;
        write_policy(&policy);
        let output = run(&["commit", "--allow-empty", "-m", "OpenPGP signed fixture"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(run(&["verify-commit", "HEAD"]).status.success());
        assert!(!read_remote());
    }

    fn profile(id: i64, label: &str) -> Profile {
        Profile {
            id,
            revision: 1,
            label: label.into(),
            git_name: format!(r#"{label} "Fixture"\Git"#),
            email: format!("{id}@example.test"),
            signing_key: None,
            signing_format: None,
            ssh_key: None,
            signing_required: false,
        }
    }

    #[test]
    fn profile_config_includes_signing_and_ssh_auth() {
        let p = Profile {
            id: 1,
            revision: 1,
            label: "Trabajo".into(),
            git_name: "Ana Lopez".into(),
            email: "ana@empresa.test".into(),
            signing_key: Some("~/.ssh/id_ed25519_trabajo.pub".into()),
            signing_format: Some("ssh".into()),
            ssh_key: Some("~/.ssh/id_ed25519_trabajo".into()),
            signing_required: true,
        };
        let cfg = profile_config(&p);
        assert!(cfg.contains(r#"name = "Ana Lopez""#));
        assert!(cfg.contains(r#"email = "ana@empresa.test""#));
        assert!(cfg.contains(r#"signingkey = "~/.ssh/id_ed25519_trabajo.pub""#));
        assert!(cfg.contains("gpgsign = true"));
        assert!(cfg.contains(r#"format = "ssh""#));
        assert!(cfg.contains(r#"[core]"#));
        assert!(cfg.contains("id_ed25519_trabajo"));
        assert!(cfg.contains("IdentityFile="));
        assert!(cfg.contains("IdentityAgent=none"));
    }

    async fn fixture_git(executable: &Path, cwd: &Path, global: &Path, args: &[&str]) {
        let mut command = git::command(executable, cwd);
        command
            .env("GIT_CONFIG_GLOBAL", global)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args(args);
        let (status, _) = git::output(&mut command, Duration::from_secs(6))
            .await
            .unwrap();
        assert!(status.success(), "fixture Git failed: {args:?}");
    }

    #[test]
    fn native_simulation_preserves_precedence_and_original_files() {
        tauri::async_runtime::block_on(async {
            let executable = git::executable().unwrap();
            let temp = tempfile::tempdir().unwrap();
            let base = PathBuf::from(discovery::directory_path(temp.path()).await.unwrap());
            let home = base.join(".gitconfig");
            let xdg = base.join("xdg").join("config");
            let relative = base.join("base.config");
            let original_home = include("base.config");
            fs::write(&home, &original_home).await.unwrap();
            fs::write(&relative, "[user]\nemail = original@example.test\n")
                .await
                .unwrap();
            let files = global_files(&home, &xdg).await.unwrap();
            assert_eq!(files.destination, home);
            let parent = base.join("Equipo [local]");
            let child = parent.join("Anidado");
            let direct = parent.join("directo");
            let inherited = child.join("nuevo");
            let override_path = child.join("excepción");
            let outside = base.join("Equipo local").join("otro");
            let fallback = base.join("Personal");
            for path in [&direct, &inherited, &override_path, &outside, &fallback] {
                fs::create_dir_all(path).await.unwrap();
                fixture_git(&executable, path, &home, &["init", "--quiet"]).await;
            }
            let path_text = |path: &Path| path.to_str().unwrap().to_owned();
            let policy = Policy {
                roots: [(&parent, Some(1)), (&child, None), (&fallback, None)]
                    .map(|(path, identity_id)| Root {
                        path: path_text(path),
                        revision: 1,
                        identity_id,
                        scan_partial: Some(false),
                    })
                    .into(),
                repositories: vec![Repository {
                    id: 1,
                    path: path_text(&override_path),
                    kind: "ordinary".into(),
                    git_dir: path_text(&override_path.join(".git")),
                    common_dir: path_text(&override_path.join(".git")),
                    present: true,
                    revision: 1,
                    identity_id: Some(3),
                }],
                profiles: [(1, "Equipo"), (2, "Predeterminada"), (3, "Excepción")]
                    .map(|(id, label)| (id, profile(id, label)))
                    .into(),
                default_id: Some(2),
            };
            let scratch = base.join("scratch");
            fs::create_dir(&scratch).await.unwrap();
            let mut preview = ConfigPreview::default();
            plan_rules(&policy, &scratch, &mut preview);
            assert_eq!(preview.rules.len(), 3);
            assert!(preview.findings.is_empty());
            assert!(preview
                .rules
                .last()
                .unwrap()
                .pattern
                .ends_with("/excepción/.git"));
            let global = simulation(&preview.rules, &files, &scratch).await.unwrap();
            for (path, expected) in [
                (&direct, "1@example.test"),
                (&inherited, "1@example.test"),
                (&override_path, "3@example.test"),
                (&outside, "original@example.test"),
                (&fallback, "2@example.test"),
            ] {
                let result = discovery::read_config(&executable, path, Some(&global))
                    .await
                    .unwrap();
                assert_eq!(result.email.unwrap().value, expected, "{path:?}");
                if path == &direct {
                    assert_eq!(result.name.unwrap().value, policy.profiles[&1].git_name);
                }
            }
            fixture_git(
                &executable,
                &inherited,
                &home,
                &["config", "--local", "user.email", "local@example.test"],
            )
            .await;
            let local = inherited.join(".git").join("config");
            let before = fs::read(&local).await.unwrap();
            let result = discovery::read_config(&executable, &inherited, Some(&global))
                .await
                .unwrap();
            assert_eq!(result.name.unwrap().scope, "global");
            let email = result.email.unwrap();
            assert_eq!(email.value, "local@example.test");
            assert_eq!(email.scope, "local");
            assert!(worktrees(&executable, &inherited).await.unwrap().is_empty());
            assert_eq!(fs::read(&local).await.unwrap(), before);
            assert_eq!(fs::read_to_string(&home).await.unwrap(), original_home);
            assert_eq!(
                fs::read_to_string(&relative).await.unwrap(),
                "[user]\nemail = original@example.test\n"
            );
        });
    }

    #[test]
    fn selects_the_last_available_global_file_and_checks_git_version() {
        assert!(compatible("git version 2.55.0.windows.5"));
        assert!(compatible("git version 2.39.5"));
        assert!(!compatible("git version 2.39.4"));
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let base = PathBuf::from(discovery::directory_path(temp.path()).await.unwrap());
            let home = base.join(".gitconfig");
            let xdg = base.join("config");
            assert_eq!(global_files(&home, &xdg).await.unwrap().destination, home);
            fs::write(&xdg, "[user]\n").await.unwrap();
            assert_eq!(global_files(&home, &xdg).await.unwrap().destination, xdg);
            fs::write(&home, "[user]\n").await.unwrap();
            assert_eq!(
                global_files(&home, &xdg).await.unwrap().existing,
                [xdg, home]
            );
        });
    }
}
