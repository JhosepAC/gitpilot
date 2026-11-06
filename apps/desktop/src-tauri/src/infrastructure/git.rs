use std::{
    env,
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command, time::timeout};

pub(crate) fn executable() -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .filter(|path| path.is_absolute())
        .map(|path| path.join(if cfg!(windows) { "git.exe" } else { "git" }))
        .find(|path| path.is_file())
}

pub(crate) fn command(executable: &Path, directory: &Path) -> Command {
    let mut command = Command::new(executable);
    command.current_dir(directory).arg("--no-pager");
    for key in [
        "GIT_DIR",
        "GIT_COMMON_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
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
    command
}

pub(crate) async fn output(
    command: &mut Command,
    deadline: Duration,
) -> std::io::Result<(ExitStatus, Vec<u8>)> {
    const OUTPUT_LIMIT: u64 = 32_768;
    #[cfg(windows)]
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    timeout(deadline, async {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("Git stdout unavailable"))?;
        let mut output = Vec::new();
        stdout
            .take(OUTPUT_LIMIT + 1)
            .read_to_end(&mut output)
            .await?;
        if output.len() as u64 > OUTPUT_LIMIT {
            return Err(std::io::Error::other("Git output limit exceeded"));
        }
        Ok((child.wait().await?, output))
    })
    .await
    .map_err(|error| std::io::Error::new(std::io::ErrorKind::TimedOut, error))?
}
