use std::{env, path::Path};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use zeroize::Zeroizing;

use super::{git, inspector, inspector_probe};
use inspector_probe::ProbeError;

#[derive(Serialize)]
pub(super) struct Credentials {
    entries: Vec<(String, String)>,
}

fn compatible_helper(value: &str) -> bool {
    matches!(value, "manager" | "manager-core" | "cache")
        || cfg!(windows) && value == "wincred"
        || cfg!(target_os = "linux") && value == "libsecret"
}

pub(super) async fn prepare(exe: &Path, cwd: &Path) -> Result<Credentials, ProbeError> {
    let (status, bytes) = inspector::run(
        exe,
        cwd,
        &[
            "config",
            "--null",
            "--get-regexp",
            r"^credential(\..*)?\.(helper|username|usehttppath)$",
        ],
    )
    .await?;
    if !status.success() || !bytes.ends_with(b"\0") {
        return Err(ProbeError::CredentialsUnavailable);
    }
    let config = std::str::from_utf8(&bytes).map_err(|_| ProbeError::Unsupported)?;
    let mut entries = Vec::new();
    for record in config.split_terminator('\0') {
        let (key, value) = record.split_once('\n').ok_or(ProbeError::Unsupported)?;
        if key.contains('=') || key.chars().chain(value.chars()).any(char::is_control) {
            return Err(ProbeError::Unsupported);
        }
        // Git applies the original order, URL matching and empty-list resets. If it
        // reaches an unsupported helper, stop lookup without running that helper.
        let value = if key.ends_with(".helper") && !value.is_empty() && !compatible_helper(value) {
            "!f() { printf 'quit=true\\n\\n'; }; f"
        } else {
            value
        };
        entries.push((key.to_owned(), value.to_owned()));
    }
    if !entries
        .iter()
        .any(|(key, value)| key.ends_with(".helper") && compatible_helper(value))
    {
        return Err(ProbeError::CredentialsUnavailable);
    }
    Ok(Credentials { entries })
}

fn basic_header(output: &str) -> Option<Zeroizing<String>> {
    let mut user = None;
    let mut password = None;
    for line in output.lines().take_while(|line| !line.is_empty()) {
        let (key, value) = line.split_once('=')?;
        if value.chars().any(char::is_control) {
            return None;
        }
        match key {
            "username" if user.is_none() => user = Some(value),
            "password" if password.is_none() => password = Some(value),
            "username" | "password" | "authtype" | "credential" => return None,
            _ => {}
        }
    }
    let (user, password) = (user?, password?);
    if user.is_empty() || user.contains(':') || password.is_empty() {
        return None;
    }
    let pair = Zeroizing::new(format!("{user}:{password}"));
    let mut header = Zeroizing::new(String::from("Authorization: Basic "));
    STANDARD.encode_string(pair.as_bytes(), &mut header);
    Some(header)
}

impl Credentials {
    pub(super) async fn header(
        &self,
        exe: &Path,
        scratch: &Path,
        url: &str,
    ) -> Result<Zeroizing<String>, ProbeError> {
        let mut command = git::isolated(exe, scratch);
        for (key, _) in env::vars_os() {
            if key
                .to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("GCM_")
            {
                command.env_remove(key);
            }
        }
        command
            .env("GCM_INTERACTIVE", "never")
            .env(
                "GCM_CREDENTIAL_STORE",
                if cfg!(windows) {
                    "wincredman"
                } else {
                    "secretservice"
                },
            )
            .arg("-c")
            .arg("credential.interactive=false");
        command.env("GIT_CONFIG_COUNT", self.entries.len().to_string());
        for (index, (key, value)) in self.entries.iter().enumerate() {
            command
                .env(format!("GIT_CONFIG_KEY_{index}"), key)
                .env(format!("GIT_CONFIG_VALUE_{index}"), value);
        }
        let input = format!("url={url}\n\n");
        let (status, output) =
            inspector_probe::bounded(command.args(["credential", "fill"]), Some(input.as_bytes()))
                .await?;
        let output = Zeroizing::new(output);
        if !status.success() {
            return Err(ProbeError::CredentialsUnavailable);
        }
        std::str::from_utf8(&output)
            .ok()
            .and_then(basic_header)
            .ok_or(ProbeError::CredentialsUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_complete_basic_credentials_and_named_helpers() {
        assert_eq!(
            &**basic_header("username=qa\npassword=demo=token\n")
                .as_ref()
                .unwrap(),
            "Authorization: Basic cWE6ZGVtbz10b2tlbg=="
        );
        for output in [
            "username=qa\n",
            "username=qa\npassword=a\npassword=b",
            "username=qa\npassword=a\nauthtype=Bearer",
            "username=qa\npassword=a\0b",
            "username=qa:other\npassword=a",
        ] {
            assert!(basic_header(output).is_none());
        }
        assert!(compatible_helper("manager"));
        assert!(!compatible_helper("manager --flag"));
        assert!(!compatible_helper("!malicious"));
        assert!(!compatible_helper("store"));
    }
}
