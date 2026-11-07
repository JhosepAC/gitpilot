use tauri::State;

use crate::{
    application::Foundation,
    infrastructure::github::{self, AccountStatus, FlowStatus, GithubError},
};

fn app(foundation: &Foundation) -> Result<&std::path::Path, GithubError> {
    foundation.directory.as_deref().ok_or(GithubError::Storage)
}

#[tauri::command]
pub(crate) async fn github_account(
    foundation: State<'_, Foundation>,
) -> Result<AccountStatus, GithubError> {
    foundation
        .github
        .account(
            foundation.database().map_err(|_| GithubError::Storage)?,
            app(&foundation)?,
        )
        .await
}

#[tauri::command]
pub(crate) async fn github_session(
    foundation: State<'_, Foundation>,
) -> Result<Option<FlowStatus>, GithubError> {
    Ok(foundation.github.current().await)
}

#[tauri::command]
pub(crate) async fn github_start(
    foundation: State<'_, Foundation>,
) -> Result<FlowStatus, GithubError> {
    foundation.database().map_err(|_| GithubError::Storage)?;
    foundation.github.start().await
}

#[tauri::command]
pub(crate) async fn github_poll(
    foundation: State<'_, Foundation>,
    session_id: String,
) -> Result<FlowStatus, GithubError> {
    foundation
        .github
        .poll(
            foundation.database().map_err(|_| GithubError::Storage)?,
            app(&foundation)?,
            &session_id,
        )
        .await
}

#[tauri::command]
pub(crate) async fn github_cancel(
    foundation: State<'_, Foundation>,
    session_id: String,
) -> Result<(), GithubError> {
    foundation.github.cancel(&session_id).await
}

#[tauri::command]
pub(crate) async fn github_check(
    foundation: State<'_, Foundation>,
) -> Result<AccountStatus, GithubError> {
    foundation
        .github
        .check(
            foundation.database().map_err(|_| GithubError::Storage)?,
            app(&foundation)?,
        )
        .await
}

#[tauri::command]
pub(crate) async fn github_open_verification(
    foundation: State<'_, Foundation>,
    session_id: String,
) -> Result<(), GithubError> {
    if !foundation.github.can_open(&session_id).await {
        return Err(GithubError::Stale);
    }
    let mut command = if cfg!(windows) {
        let system = std::env::var_os("SystemRoot").ok_or(GithubError::Unavailable)?;
        let mut command = tokio::process::Command::new(
            std::path::PathBuf::from(system).join("System32/rundll32.exe"),
        );
        command.args(["url.dll,FileProtocolHandler", github::VERIFICATION_URI]);
        command
    } else {
        let mut command = tokio::process::Command::new("/usr/bin/xdg-open");
        command.arg(github::VERIFICATION_URI);
        command
    };
    let (status, _) =
        crate::infrastructure::git::output(&mut command, std::time::Duration::from_secs(5))
            .await
            .map_err(|_| GithubError::Unavailable)?;
    status
        .success()
        .then_some(())
        .ok_or(GithubError::Unavailable)
}
