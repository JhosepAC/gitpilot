use std::path::Path;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::repository::Repository;

use super::{
    config_apply::ApplyError,
    config_preview::{self, Candidate},
    discovery, inspector, inventory, native_file,
};

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HookMode {
    Warn,
    Block,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HookPreview {
    target: String,
    mode: HookMode,
    identity: String,
    pub(super) confirmation: String,
    #[serde(skip)]
    pub(super) candidate: Candidate,
    #[serde(skip)]
    repository: Repository,
}

fn hook_script(identity: &str, mode: HookMode) -> String {
    let escaped = identity.replace('\'', "'\\''");
    let mismatch = match mode {
        HookMode::Warn => 0,
        HookMode::Block => 1,
    };
    format!(
        r#"#!/bin/sh
# Managed by GitPilot
expected='{escaped}'
identity() {{
    value=$(git var "$1" 2>/dev/null) || return 1
    value=${{value% *}}
    printf '%s\n' "${{value% *}}"
}}
author=$(identity GIT_AUTHOR_IDENT) || author=
committer=$(identity GIT_COMMITTER_IDENT) || committer=
if [ "$author" = "$expected" ] && [ "$committer" = "$expected" ]; then
    exit 0
fi
printf '%s\n' 'GitPilot: el autor o el committer difiere de la identidad revisada.' >&2
exit {mismatch}
"#
    )
}

async fn target(repository: &Repository) -> Result<std::path::PathBuf, ApplyError> {
    if repository.kind != "ordinary"
        || repository.git_dir != repository.common_dir
        || Path::new(&repository.path).join(".git") != Path::new(&repository.git_dir)
        || [
            "GIT_CONFIG_GLOBAL",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_COMMON_DIR",
        ]
        .iter()
        .any(|key| std::env::var_os(key).is_some())
    {
        return Err(ApplyError::UnsafeTarget);
    }
    let (exe, cwd) = inspector::context(repository)
        .await
        .map_err(|_| ApplyError::UnsafeTarget)?;
    let top = inspector::line(&exe, cwd, &["rev-parse", "--show-toplevel"])
        .await
        .ok_or(ApplyError::UnsafeTarget)?;
    if !discovery::directory_path(Path::new(&top))
        .await
        .is_ok_and(|top| top == repository.path)
        || !config_preview::worktrees(&exe, cwd)
            .await
            .is_ok_and(|paths| paths.is_empty())
    {
        return Err(ApplyError::UnsafeTarget);
    }
    let (status, _) = inspector::run(&exe, cwd, &["config", "--get", "core.hooksPath"])
        .await
        .map_err(|_| ApplyError::UnsafeTarget)?;
    if status.code() != Some(1) {
        return Err(ApplyError::UnsafeTarget);
    }
    let actual = inspector::line(
        &exe,
        cwd,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "hooks/pre-commit",
        ],
    )
    .await
    .ok_or(ApplyError::UnsafeTarget)?;
    let expected = Path::new(&repository.git_dir)
        .join("hooks")
        .join("pre-commit");
    if Path::new(&actual) != expected {
        return Err(ApplyError::UnsafeTarget);
    }
    Ok(expected)
}

pub(crate) async fn preview(
    pool: &SqlitePool,
    id: i64,
    mode: HookMode,
) -> Result<HookPreview, ApplyError> {
    let repository = inventory::repository(pool, id)
        .await
        .map_err(|_| ApplyError::StalePreview)?;
    let target = target(&repository).await?;
    let parent = native_file::parent_id(&target).map_err(|_| ApplyError::UnsafeTarget)?;
    if native_file::snapshot(&target)
        .map_err(|_| ApplyError::UnsafeTarget)?
        .is_some()
    {
        return Err(ApplyError::ActiveOperation);
    }
    let expected = inventory::expected(pool, &repository)
        .await
        .map_err(|_| ApplyError::StalePreview)?
        .ok_or(ApplyError::IdentityRequired)?;
    let identity = format!("{} <{}>", expected.git_name, expected.email);
    let append = hook_script(&identity, mode);
    let record = serde_json::to_vec(&(
        &target,
        parent,
        repository.revision,
        mode,
        &expected,
        &append,
    ))
    .map_err(|_| ApplyError::StorageUnavailable)?;
    Ok(HookPreview {
        target: target.to_str().ok_or(ApplyError::UnsafeTarget)?.into(),
        confirmation: hex::encode(native_file::digest(&record)),
        identity,
        mode,
        candidate: Candidate {
            target,
            parent,
            before: None,
            generated: None,
            append,
        },
        repository,
    })
}

impl HookPreview {
    pub(super) async fn verify(&self) -> Result<(), ApplyError> {
        if target(&self.repository).await? == self.candidate.target {
            Ok(())
        } else {
            Err(ApplyError::VerificationFailed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::git;
    use std::time::Duration;

    #[test]
    fn real_commits_warn_block_and_accept_the_reviewed_identity() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let exe = git::executable().unwrap();
            let cwd = temp.path();
            let execute = |args: &[&str], name: &str| {
                let mut command = git::isolated(&exe, cwd);
                command
                    .env_remove("GIT_AUTHOR_NAME")
                    .env_remove("GIT_AUTHOR_EMAIL")
                    .env_remove("GIT_COMMITTER_NAME")
                    .env_remove("GIT_COMMITTER_EMAIL")
                    .env_remove("GIT_AUTHOR_DATE")
                    .env_remove("GIT_COMMITTER_DATE")
                    .arg("-c")
                    .arg(format!("user.name={name}"))
                    .args([
                        "-c",
                        "user.email=fixture@example.test",
                        "-c",
                        "commit.gpgSign=false",
                    ])
                    .args(args);
                command
            };
            assert!(git::output(
                &mut execute(&["init", "-q"], "Fixture"),
                Duration::from_secs(8)
            )
            .await
            .unwrap()
            .0
            .success());
            let hook = cwd.join(".git/hooks/pre-commit");
            let commit = ["commit", "--allow-empty", "-qm", "Fixture"];
            for (mode, name, allowed) in [
                (HookMode::Warn, "Different", true),
                (HookMode::Block, "Different", false),
                (HookMode::Block, "O'Neil", true),
            ] {
                std::fs::write(&hook, hook_script("O'Neil <fixture@example.test>", mode)).unwrap();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o700))
                        .unwrap();
                }
                let result = git::output(&mut execute(&commit, name), Duration::from_secs(8))
                    .await
                    .unwrap()
                    .0;
                assert_eq!(result.success(), allowed);
            }
        });
    }
}
