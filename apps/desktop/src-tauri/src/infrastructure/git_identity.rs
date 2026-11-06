use std::{env, path::Path, process::Stdio, time::Duration};

use tokio::{io::AsyncReadExt, process::Command, time::timeout};

use crate::identity::{IdentityError, IdentityInput};

pub(crate) async fn detect(directory: &Path) -> Result<IdentityInput, IdentityError> {
    let path = env::var_os("PATH").ok_or(IdentityError::GitUnavailable)?;
    let executable = env::split_paths(&path)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(if cfg!(windows) { "git.exe" } else { "git" }))
        .find(|file| file.is_file())
        .ok_or(IdentityError::GitUnavailable)?;
    let mut command = Command::new(executable);
    command.current_dir(directory);
    for key in [
        "GIT_DIR",
        "GIT_COMMON_DIR",
        "GIT_WORK_TREE",
        "GIT_CONFIG",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_PARAMETERS",
    ] {
        command.env_remove(key);
    }
    for (key, _) in env::vars_os() {
        if key
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("GIT_TRACE")
        {
            command.env_remove(key);
        }
    }
    read_global_identity(&mut command).await
}

async fn read_global_identity(command: &mut Command) -> Result<IdentityInput, IdentityError> {
    const OUTPUT_LIMIT: u64 = 16_384;

    #[cfg(windows)]
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = command
        .args([
            "--no-pager",
            "config",
            "--global",
            "--includes",
            "--null",
            "--get-regexp",
            r"^user\.(name|email)$",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| IdentityError::GitUnavailable)?;
    timeout(Duration::from_secs(3), async {
        let stdout = child.stdout.take().ok_or(IdentityError::GitReadFailed)?;
        let mut output = Vec::new();
        stdout
            .take(OUTPUT_LIMIT + 1)
            .read_to_end(&mut output)
            .await
            .map_err(|_| IdentityError::GitReadFailed)?;
        if output.len() as u64 > OUTPUT_LIMIT {
            return Err(IdentityError::GitReadFailed);
        }
        let status = child
            .wait()
            .await
            .map_err(|_| IdentityError::GitReadFailed)?;
        if status.code() == Some(1) && output.is_empty() {
            return Err(IdentityError::GitIdentityMissing);
        }
        if !status.success() {
            return Err(IdentityError::GitReadFailed);
        }
        parse_identity(&output)
    })
    .await
    .map_err(|_| IdentityError::GitReadFailed)?
}

fn parse_identity(output: &[u8]) -> Result<IdentityInput, IdentityError> {
    let text = std::str::from_utf8(output).map_err(|_| IdentityError::GitReadFailed)?;
    if !text.ends_with('\0') {
        return Err(IdentityError::GitReadFailed);
    }
    let mut name = "";
    let mut email = "";
    for entry in text.split_terminator('\0') {
        match entry.split_once('\n') {
            Some(("user.name", value)) => name = value,
            Some(("user.email", value)) => email = value,
            _ => return Err(IdentityError::GitReadFailed),
        }
    }
    if name.trim().is_empty() || email.trim().is_empty() {
        return Err(IdentityError::GitIdentityMissing);
    }
    Ok(IdentityInput {
        label: "Identidad de Git".into(),
        git_name: name.into(),
        email: email.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_last_global_values_and_leaves_config_untouched() {
        tauri::async_runtime::block_on(async {
            let folder = tempfile::tempdir().unwrap();
            let path = folder.path().join("gitconfig");
            let content = "[user]\nname = First\nemail = old@example.test\nname = Ana\nemail = ana@example.test\n";
            std::fs::write(&path, content).unwrap();
            let mut command = Command::new("git");
            command
                .current_dir(folder.path())
                .env("GIT_CONFIG_GLOBAL", &path);
            let profile = read_global_identity(&mut command).await.unwrap();
            assert_eq!(profile.git_name, "Ana");
            assert_eq!(profile.email, "ana@example.test");
            assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        });
    }

    #[test]
    fn rejects_missing_or_malformed_records() {
        assert!(matches!(
            parse_identity(b"user.name\nAna\0"),
            Err(IdentityError::GitIdentityMissing)
        ));
        assert!(matches!(
            parse_identity(b"user.email\0"),
            Err(IdentityError::GitReadFailed)
        ));
    }
}
