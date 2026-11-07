use std::{
    env,
    path::{Path, PathBuf},
};

use serde::Serialize;
use tokio::{fs, process::Command};

use super::{git, inspector, inspector_probe};
use inspector_probe::ProbeError;

#[derive(Serialize)]
pub(super) struct OpenPgp {
    program: PathBuf,
    home: Option<PathBuf>,
    key: String,
}

pub(super) fn executable(git_exe: &Path) -> Option<PathBuf> {
    git::find_executable("gpg").or_else(|| {
        if cfg!(windows) {
            let bundled = git_exe.parent()?.parent()?.join("usr/bin/gpg.exe");
            return bundled.is_file().then_some(bundled);
        }
        None
    })
}

fn key_reference(value: &str) -> Option<String> {
    let digits = value.strip_suffix('!').unwrap_or(value);
    let digits = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
        .unwrap_or(digits);
    (matches!(digits.len(), 16 | 40 | 64) && digits.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| value.to_ascii_uppercase().replace("0X", "0x"))
}

pub(super) async fn prepare(exe: &Path, cwd: &Path) -> Result<OpenPgp, ProbeError> {
    let (status, settings) = inspector::run(
        exe,
        cwd,
        &[
            "config",
            "--null",
            "--get-regexp",
            r"^(gpg\.(format|program|openpgp\.program)|user\.signingkey)$",
        ],
    )
    .await?;
    if !status.success() || !settings.ends_with(b"\0") {
        return Err(ProbeError::Unsupported);
    }
    let text = std::str::from_utf8(&settings).map_err(|_| ProbeError::Unsupported)?;
    let (mut format, mut program, mut key) = ("openpgp", None, None);
    for record in text.split_terminator('\0') {
        let (name, value) = record.split_once('\n').ok_or(ProbeError::Unsupported)?;
        match name {
            "gpg.format" => format = value,
            "gpg.program" | "gpg.openpgp.program" => program = Some(value),
            "user.signingkey" => key = key_reference(value),
            _ => {}
        }
    }
    let key = key
        .filter(|_| format == "openpgp")
        .ok_or(ProbeError::Unsupported)?;
    let trusted = executable(exe).ok_or(ProbeError::Unavailable)?;
    if let Some(value) = program {
        let configured = Path::new(value);
        let recognized = value == "gpg"
            || cfg!(windows) && value == "gpg.exe"
            || configured.is_absolute()
                && fs::canonicalize(configured)
                    .await
                    .is_ok_and(|path| trusted.canonicalize().is_ok_and(|trusted| trusted == path));
        if !recognized {
            return Err(ProbeError::Unsupported);
        }
    }
    let home = env::var_os("GNUPGHOME")
        .map(|home| gpg_home(&trusted, PathBuf::from(home)))
        .transpose()?;
    Ok(OpenPgp {
        program: trusted,
        home,
        key,
    })
}

fn gpg_home(program: &Path, home: PathBuf) -> Result<PathBuf, ProbeError> {
    if cfg!(windows)
        && program
            .parent()
            .is_some_and(|dir| dir.join("msys-2.0.dll").is_file())
    {
        // The GnuPG bundled with Git for Windows interprets homedir as an MSYS path.
        let path = home
            .to_str()
            .ok_or(ProbeError::Unsupported)?
            .replace('\\', "/");
        if let Some((drive, tail)) = path.split_once(":/") {
            if drive.len() == 1 && drive.as_bytes()[0].is_ascii_alphabetic() {
                return Ok(PathBuf::from(format!(
                    "/{}/{tail}",
                    drive.to_ascii_lowercase()
                )));
            }
        }
        if path.starts_with('/') && !path.starts_with("//") {
            return Ok(PathBuf::from(path));
        }
        return Err(ProbeError::Unsupported);
    }
    home.is_absolute()
        .then_some(home)
        .ok_or(ProbeError::Unsupported)
}

impl OpenPgp {
    pub(super) fn key(&self) -> &str {
        &self.key
    }

    fn command(&self, scratch: &Path) -> Command {
        let mut command = Command::new(&self.program);
        command.current_dir(scratch).args([
            "--no-options",
            "--batch",
            "--no-tty",
            "--pinentry-mode",
            "error",
            "--no-auto-key-retrieve",
            "--auto-key-locate",
            "local",
            "--status-fd",
            "1",
        ]);
        if let Some(home) = &self.home {
            command.env("GNUPGHOME", home).arg("--homedir").arg(home);
        }
        command
    }

    pub(super) async fn verify(&self, scratch: &Path) -> Result<(), ProbeError> {
        let mut nonce = [0u8; 32];
        getrandom::fill(&mut nonce).map_err(|_| ProbeError::Unavailable)?;
        fs::write(
            scratch.join("challenge"),
            format!("GitPilot OpenPGP test {}\n", hex::encode(nonce)),
        )
        .await
        .map_err(|_| ProbeError::Unavailable)?;
        let (status, signed) = inspector_probe::bounded(
            self.command(scratch).args([
                "--local-user",
                &self.key,
                "--output",
                "challenge.sig",
                "--detach-sign",
                "challenge",
            ]),
            None,
        )
        .await?;
        let signer = status_fingerprint(&signed, "[GNUPG:] SIG_CREATED ", 5);
        if !status.success() || signer.is_none() {
            return Err(ProbeError::SigningUnavailable);
        }
        let (status, verified) = inspector_probe::bounded(
            self.command(scratch)
                .args(["--verify", "challenge.sig", "challenge"]),
            None,
        )
        .await?;
        if !status.success() || status_fingerprint(&verified, "[GNUPG:] VALIDSIG ", 0) != signer {
            return Err(ProbeError::Failed);
        }
        Ok(())
    }
}

fn status_fingerprint<'a>(bytes: &'a [u8], prefix: &str, index: usize) -> Option<&'a str> {
    std::str::from_utf8(bytes)
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix(prefix)?
                .split_ascii_whitespace()
                .nth(index)
        })
        .filter(|value| {
            matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_key_identifiers_are_accepted() {
        assert_eq!(
            key_reference("0x001122334455aabb!").as_deref(),
            Some("0x001122334455AABB!")
        );
        assert!(key_reference("user@example.test").is_none());
        assert!(key_reference("--list-keys").is_none());
    }
}
