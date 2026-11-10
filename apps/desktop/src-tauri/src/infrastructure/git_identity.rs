use std::{path::Path, time::Duration};

use tokio::process::Command;

use crate::identity::{IdentityError, IdentityInput};

use super::git;

pub(crate) async fn detect(directory: &Path) -> Result<IdentityInput, IdentityError> {
    let executable = git::executable().ok_or(IdentityError::GitUnavailable)?;
    read_global_identity(&mut git::command(&executable, directory)).await
}

async fn read_global_identity(command: &mut Command) -> Result<IdentityInput, IdentityError> {
    command.args([
        "config",
        "--global",
        "--includes",
        "--null",
        "--get-regexp",
        r"^user\.(name|email)$",
    ]);
    let (status, output) = git::output(command, Duration::from_secs(3))
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                IdentityError::GitUnavailable
            } else {
                IdentityError::GitReadFailed
            }
        })?;
    if status.code() == Some(1) && output.is_empty() {
        return Err(IdentityError::GitIdentityMissing);
    }
    if !status.success() || output.len() > 16_384 {
        return Err(IdentityError::GitReadFailed);
    }
    parse_identity(&output)
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
        signing_key: None,
        signing_format: None,
        ssh_key: None,
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
