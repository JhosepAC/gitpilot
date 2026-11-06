use std::{
    fs::Metadata,
    path::Path,
    time::{Duration, Instant},
};

use tokio::fs;

use crate::repository::{
    ConfigValue, DiscoveredRepository, InventoryError, ObservedIdentity, ScanReport,
};

use super::git;

const SCAN_TIME: Duration = Duration::from_secs(12);
const GIT_TIME: Duration = Duration::from_secs(2);
const MAX_ENTRIES: usize = 12_000;
const MAX_DEPTH: usize = 24;
const MAX_REPOSITORIES: usize = 200;

fn is_link(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    {
        metadata.is_symlink()
    }
}

fn path_text(path: &Path) -> Result<String, InventoryError> {
    let text = path.to_str().ok_or(InventoryError::InvalidPath)?;
    if text.chars().any(char::is_control) {
        return Err(InventoryError::InvalidPath);
    }
    #[cfg(windows)]
    if let Some(text) = text.strip_prefix(r"\\?\") {
        return Ok(match text.strip_prefix(r"UNC\") {
            Some(unc) => format!(r"\\{unc}"),
            None => text.into(),
        });
    }
    Ok(text.into())
}

pub(crate) async fn directory_path(path: &Path) -> Result<String, InventoryError> {
    if !path.is_absolute() {
        return Err(InventoryError::InvalidPath);
    }
    tokio::time::timeout(GIT_TIME, async {
        let path = fs::canonicalize(path)
            .await
            .map_err(|_| InventoryError::InvalidPath)?;
        if !safe_metadata(&path).await.is_some_and(|m| m.is_dir()) {
            return Err(InventoryError::InvalidPath);
        }
        path_text(&path)
    })
    .await
    .map_err(|_| InventoryError::InvalidPath)?
}

async fn safe_metadata(path: &Path) -> Option<Metadata> {
    let metadata = fs::symlink_metadata(path).await.ok()?;
    (!is_link(&metadata)).then_some(metadata)
}

pub(crate) async fn scan(
    stored_path: &str,
    exclusions: &[String],
) -> Result<(Vec<DiscoveredRepository>, ScanReport), InventoryError> {
    let executable = git::executable().ok_or(InventoryError::GitUnavailable)?;
    let registered = Path::new(stored_path);
    let root = tokio::time::timeout(GIT_TIME, async {
        let root = fs::canonicalize(registered)
            .await
            .map_err(|_| InventoryError::InvalidPath)?;
        if path_text(&root)? != stored_path
            || !safe_metadata(&root).await.is_some_and(|m| m.is_dir())
        {
            return Err(InventoryError::InvalidPath);
        }
        Ok(root)
    })
    .await
    .map_err(|_| InventoryError::InvalidPath)??;
    let deadline = Instant::now() + SCAN_TIME;
    let mut pending = vec![(root.clone(), 0)];
    let mut repositories = Vec::new();
    let mut report = ScanReport::default();
    let mut visited = 0;

    let walk = tokio::time::timeout(SCAN_TIME, async {
        'walk: while let Some((directory, depth)) = pending.pop() {
            if deadline <= Instant::now() || repositories.len() >= MAX_REPOSITORIES {
                report.partial = true;
                break;
            }
            let Ok(text) = path_text(&directory) else {
                report.skipped += 1;
                continue;
            };
            if exclusions
                .iter()
                .any(|excluded| Path::new(&text).starts_with(excluded))
            {
                continue;
            }
            // Refuse paths swapped to an alias between enumeration and inspection.
            if !safe_metadata(&directory).await.is_some_and(|m| m.is_dir())
                || !fs::canonicalize(&directory)
                    .await
                    .is_ok_and(|p| p == directory && p.starts_with(&root))
            {
                report.skipped += 1;
                continue;
            }
            let git_marker = directory.join(".git");
            let marker = safe_metadata(&git_marker).await;
            let bare = marker.is_none()
                && safe_metadata(&directory.join("HEAD"))
                    .await
                    .is_some_and(|m| m.is_file())
                && safe_metadata(&directory.join("objects"))
                    .await
                    .is_some_and(|m| m.is_dir());
            if bare || marker.as_ref().is_some_and(|m| m.is_file() || m.is_dir()) {
                match probe(
                    &executable,
                    &directory,
                    deadline,
                    bare,
                    marker.is_some_and(|m| m.is_file()),
                )
                .await
                {
                    Ok(repo) => repositories.push(repo),
                    Err(_) => report.skipped += 1,
                }
            }
            if bare {
                continue;
            }
            let Ok(mut children) = fs::read_dir(&directory).await else {
                report.skipped += 1;
                continue;
            };
            loop {
                match children.next_entry().await {
                    Ok(Some(child)) => {
                        visited += 1;
                        if visited >= MAX_ENTRIES || deadline <= Instant::now() {
                            report.partial = true;
                            break 'walk;
                        }
                        if child.file_name().eq_ignore_ascii_case(".git") {
                            continue;
                        }
                        let child = child.path();
                        match fs::symlink_metadata(&child).await {
                            Ok(m) if !is_link(&m) && m.is_dir() => {
                                if depth < MAX_DEPTH {
                                    pending.push((child, depth + 1));
                                } else {
                                    report.partial = true;
                                }
                            }
                            Err(_) => report.skipped += 1,
                            _ => {}
                        }
                    }
                    Ok(None) => break,
                    Err(_) => {
                        report.skipped += 1;
                        break;
                    }
                }
            }
        }
    })
    .await;
    report.found = repositories.len();
    report.partial |= walk.is_err() || report.skipped > 0;
    Ok((repositories, report))
}

async fn read(
    executable: &Path,
    path: &Path,
    deadline: Instant,
    args: &[&str],
) -> Result<(std::process::ExitStatus, String), InventoryError> {
    let time = deadline
        .saturating_duration_since(Instant::now())
        .min(GIT_TIME);
    if time.is_zero() {
        return Err(InventoryError::GitReadFailed);
    }
    let mut command = git::command(executable, path);
    let (status, bytes) = git::output(command.args(args), time)
        .await
        .map_err(|_| InventoryError::GitReadFailed)?;
    Ok((
        status,
        String::from_utf8(bytes).map_err(|_| InventoryError::GitReadFailed)?,
    ))
}

async fn probe(
    executable: &Path,
    directory: &Path,
    deadline: Instant,
    bare: bool,
    git_file: bool,
) -> Result<DiscoveredRepository, InventoryError> {
    let (status, output) = read(
        executable,
        directory,
        deadline,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
            "--is-bare-repository",
        ],
    )
    .await?;
    if !status.success() {
        return Err(InventoryError::GitReadFailed);
    }
    let mut lines = output.lines();
    let (Some(git_dir), Some(common_dir), Some(is_bare), None) =
        (lines.next(), lines.next(), lines.next(), lines.next())
    else {
        return Err(InventoryError::GitReadFailed);
    };
    if !Path::new(git_dir).is_absolute()
        || !Path::new(common_dir).is_absolute()
        || is_bare != if bare { "true" } else { "false" }
    {
        return Err(InventoryError::GitReadFailed);
    }
    let git_dir = directory_path(Path::new(git_dir)).await?;
    let common_dir = directory_path(Path::new(common_dir)).await?;
    let path = path_text(directory)?;
    if bare {
        if git_dir != path {
            return Err(InventoryError::GitReadFailed);
        }
    } else {
        let (status, top) = read(
            executable,
            directory,
            deadline,
            &["rev-parse", "--show-toplevel"],
        )
        .await?;
        if !status.success()
            || directory_path(Path::new(top.trim_end_matches(['\r', '\n']))).await? != path
        {
            return Err(InventoryError::GitReadFailed);
        }
    }
    let kind = if bare {
        "bare"
    } else if git_dir != common_dir {
        "worktree"
    } else if git_file {
        "separate"
    } else {
        "ordinary"
    };
    Ok(DiscoveredRepository {
        path,
        kind,
        git_dir,
        common_dir,
    })
}

pub(crate) async fn observe(path: &str, git_dir: &str) -> Result<ObservedIdentity, InventoryError> {
    let executable = git::executable().ok_or(InventoryError::GitUnavailable)?;
    let directory = Path::new(path);
    if directory_path(directory).await? != path {
        return Err(InventoryError::InvalidPath);
    }
    let deadline = Instant::now() + Duration::from_secs(6);
    let (status, resolved) = read(
        &executable,
        directory,
        deadline,
        &["rev-parse", "--absolute-git-dir"],
    )
    .await?;
    if !status.success()
        || directory_path(Path::new(resolved.trim_end_matches(['\r', '\n']))).await? != git_dir
    {
        return Err(InventoryError::GitReadFailed);
    }
    let mut observed = read_config(&executable, directory, None).await?;
    for (key, slot) in [
        ("GIT_AUTHOR_IDENT", &mut observed.author),
        ("GIT_COMMITTER_IDENT", &mut observed.committer),
    ] {
        if let Ok((status, output)) = read(&executable, directory, deadline, &["var", key]).await {
            if status.success() {
                *slot = output
                    .rfind('>')
                    .and_then(|end| safe_value(&output[..=end]));
            }
        }
    }
    Ok(observed)
}

pub(super) async fn read_config(
    executable: &Path,
    directory: &Path,
    simulated_global: Option<&Path>,
) -> Result<ObservedIdentity, InventoryError> {
    let mut command = git::command(executable, directory);
    if let Some(global) = simulated_global {
        command.env("GIT_CONFIG_GLOBAL", global);
    }
    command.args([
        "config",
        "--null",
        "--show-scope",
        "--show-origin",
        "--get-regexp",
        r"^user\.(name|email)$",
    ]);
    let (status, output) = git::output(&mut command, GIT_TIME)
        .await
        .map_err(|_| InventoryError::GitReadFailed)?;
    if !status.success() && (status.code() != Some(1) || !output.is_empty()) {
        return Err(InventoryError::GitReadFailed);
    }
    parse_config(std::str::from_utf8(&output).map_err(|_| InventoryError::GitReadFailed)?)
}

fn safe_value(value: &str) -> Option<String> {
    (!value.is_empty() && value.len() <= 4096 && !value.chars().any(char::is_control))
        .then(|| value.to_owned())
}

fn parse_config(output: &str) -> Result<ObservedIdentity, InventoryError> {
    let mut observed = ObservedIdentity::default();
    let mut fields = output.split_terminator('\0');
    while let Some(scope) = fields.next() {
        let origin = fields.next().ok_or(InventoryError::GitReadFailed)?;
        let field = fields.next().ok_or(InventoryError::GitReadFailed)?;
        let (key, value) = field
            .split_once('\n')
            .ok_or(InventoryError::GitReadFailed)?;
        let item = safe_value(value).map(|value| ConfigValue {
            value,
            scope: safe_value(scope).unwrap_or_default(),
            origin: safe_value(origin).unwrap_or_default(),
        });
        match key {
            "user.name" => observed.name = item,
            "user.email" => observed.email = item,
            _ => return Err(InventoryError::GitReadFailed),
        }
    }
    Ok(observed)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fixture_git(executable: &Path, path: &Path, args: &[&str]) -> String {
        let mut command = git::command(executable, path);
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", path.join("empty-config"))
            .args(args);
        let (status, output) = git::output(&mut command, Duration::from_secs(8))
            .await
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn discovers_real_nested_bare_and_linked_repositories_with_exclusions() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("Proyectos [test]");
            std::fs::create_dir(&root).unwrap();
            let git = git::executable().unwrap();
            for path in ["repo", "repo/nested", "excluded", "excluded-sibling"] {
                fixture_git(&git, &root, &["init", "-q", path]).await;
            }
            fixture_git(&git, &root, &["init", "-q", "--bare", "archive.git"]).await;
            fixture_git(
                &git,
                &root.join("repo"),
                &[
                    "-c",
                    "user.name=Fixture",
                    "-c",
                    "user.email=fixture@example.test",
                    "-c",
                    "commit.gpgsign=false",
                    "commit",
                    "--allow-empty",
                    "-qm",
                    "fixture",
                ],
            )
            .await;
            fixture_git(
                &git,
                &root.join("repo"),
                &["worktree", "add", "--detach", "-q", "../linked"],
            )
            .await;
            #[cfg(unix)]
            {
                fixture_git(&git, temp.path(), &["init", "-q", "outside"]).await;
                std::os::unix::fs::symlink(temp.path().join("outside"), root.join("alias"))
                    .unwrap();
            }
            let root_path = directory_path(&root).await.unwrap();
            let exclude = directory_path(&root.join("excluded")).await.unwrap();
            let (repos, report) = scan(&root_path, &[exclude]).await.unwrap();
            assert!(!report.partial, "skipped {}", report.skipped);
            assert_eq!(report.found, 5);
            assert!(repos
                .iter()
                .any(|r| r.kind == "worktree" && r.git_dir != r.common_dir));
            assert!(repos.iter().any(|r| r.kind == "bare"));
            assert!(repos.iter().any(|r| r.path.ends_with("excluded-sibling")));
            assert!(!repos
                .iter()
                .any(|r| r.path.ends_with("excluded") || r.path.ends_with("alias")));
        });
    }

    #[test]
    fn reads_effective_config_and_preserves_the_last_scope_per_key() {
        tauri::async_runtime::block_on(async {
            let root = tempfile::tempdir().unwrap();
            let git = git::executable().unwrap();
            fixture_git(&git, root.path(), &["init", "-q"]).await;
            let cfg = root.path().join(".git/config");
            std::fs::write(
                root.path().join("empty-config"),
                "[user]\nname = Global\nemail = global@example.test\n",
            )
            .unwrap();
            fixture_git(
                &git,
                root.path(),
                &["config", "--local", "user.email", "local@example.test"],
            )
            .await;
            let original = std::fs::read(&cfg).unwrap();
            let raw = fixture_git(
                &git,
                root.path(),
                &[
                    "config",
                    "--null",
                    "--show-scope",
                    "--show-origin",
                    "--get-regexp",
                    r"^user\.(name|email)$",
                ],
            )
            .await;
            let observed = parse_config(&raw).unwrap();
            assert_eq!(observed.name.unwrap().scope, "global");
            let email = observed.email.unwrap();
            assert_eq!(email.value, "local@example.test");
            assert_eq!(email.scope, "local");
            assert!(email.origin.starts_with("file:"));
            assert_eq!(std::fs::read(cfg).unwrap(), original);
        });
    }
}
