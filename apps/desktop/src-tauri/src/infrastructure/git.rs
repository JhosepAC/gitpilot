use std::{
    env,
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    time::timeout,
};

pub(crate) fn executable() -> Option<PathBuf> {
    find_executable("git")
}

pub(crate) fn find_executable(name: &str) -> Option<PathBuf> {
    let filename = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    };
    env::split_paths(&env::var_os("PATH")?)
        .filter(|path| path.is_absolute())
        .map(|path| path.join(&filename))
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
    output_with_input(command, deadline, None).await
}

pub(super) fn isolated(executable: &Path, directory: &Path) -> Command {
    let mut command = command(executable, directory);
    for (key, _) in env::vars_os() {
        let key_name = key.to_string_lossy().to_ascii_uppercase();
        if key_name.starts_with("GIT_") || key_name.ends_with("_PROXY") || key_name == "SSH_ASKPASS"
        {
            command.env_remove(key);
        }
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", directory.join("gitpilot-empty-config"))
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_ALLOW_PROTOCOL", "")
        .env(
            "GIT_CEILING_DIRECTORIES",
            directory.parent().unwrap_or(directory),
        );
    command
}

pub(super) async fn output_with_input(
    command: &mut Command,
    deadline: Duration,
    input: Option<&[u8]>,
) -> std::io::Result<(ExitStatus, Vec<u8>)> {
    const OUTPUT_LIMIT: u64 = 32_768;
    #[cfg(windows)]
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    timeout(deadline, async {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("Git stdout unavailable"))?;
        let stdin = child.stdin.take();
        let (read, written) = tokio::join!(
            async {
                let mut output = Vec::new();
                stdout
                    .take(OUTPUT_LIMIT + 1)
                    .read_to_end(&mut output)
                    .await?;
                Ok::<_, std::io::Error>(output)
            },
            async {
                if let (Some(mut stdin), Some(input)) = (stdin, input) {
                    stdin.write_all(input).await?;
                    stdin.shutdown().await?;
                }
                Ok::<_, std::io::Error>(())
            }
        );
        let output = read?;
        written?;
        if output.len() as u64 > OUTPUT_LIMIT {
            return Err(std::io::Error::other("Git output limit exceeded"));
        }
        Ok((child.wait().await?, output))
    })
    .await
    .map_err(|error| std::io::Error::new(std::io::ErrorKind::TimedOut, error))?
}
