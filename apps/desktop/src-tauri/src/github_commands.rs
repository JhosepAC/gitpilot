use tauri::State;

use crate::{
    application::Foundation,
    infrastructure::github::{
        self, AccountStatus, ActivityItem, ActivityKind, FlowStatus, GithubError, Installation,
        RemoteCandidates, RemoteReview, Repository, RepositoryScope, ResourceError, ResourcePage,
    },
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
    github_id: Option<i64>,
) -> Result<FlowStatus, GithubError> {
    foundation
        .github
        .begin(
            foundation.database().map_err(|_| GithubError::Storage)?,
            app(&foundation)?,
            github_id,
        )
        .await
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
    github_id: i64,
) -> Result<AccountStatus, GithubError> {
    foundation
        .github
        .check(
            foundation.database().map_err(|_| GithubError::Storage)?,
            app(&foundation)?,
            github_id,
        )
        .await
}

#[tauri::command]
pub(crate) async fn github_installations(
    foundation: State<'_, Foundation>,
    github_id: i64,
    page: u32,
) -> Result<ResourcePage<Installation>, ResourceError> {
    foundation
        .github
        .installations(
            foundation.database().map_err(|_| GithubError::Storage)?,
            app(&foundation)?,
            github_id,
            page,
        )
        .await
}

#[tauri::command]
pub(crate) async fn github_repositories(
    foundation: State<'_, Foundation>,
    github_id: i64,
    installation_id: i64,
    page: u32,
) -> Result<ResourcePage<Repository>, ResourceError> {
    foundation
        .github
        .repositories(
            foundation.database().map_err(|_| GithubError::Storage)?,
            app(&foundation)?,
            github_id,
            installation_id,
            page,
        )
        .await
}

#[tauri::command]
pub(crate) async fn github_repository_activity(
    foundation: State<'_, Foundation>,
    scope: RepositoryScope,
    kind: ActivityKind,
    page: u32,
) -> Result<ResourcePage<ActivityItem>, ResourceError> {
    foundation
        .github
        .repository_activity(
            foundation.database().map_err(|_| GithubError::Storage)?,
            app(&foundation)?,
            scope,
            kind,
            page,
        )
        .await
}

#[tauri::command]
pub(crate) async fn github_remote_candidates(
    foundation: State<'_, Foundation>,
    local_id: i64,
    scope: Option<RepositoryScope>,
) -> Result<RemoteCandidates, ResourceError> {
    github::candidates(
        foundation.database().map_err(|_| GithubError::Storage)?,
        local_id,
        scope,
    )
    .await
}

#[tauri::command]
pub(crate) async fn github_link_remote(
    foundation: State<'_, Foundation>,
    scope: RepositoryScope,
    review: RemoteReview,
) -> Result<(), ResourceError> {
    foundation
        .github
        .link_remote(
            foundation.database().map_err(|_| GithubError::Storage)?,
            app(&foundation)?,
            scope,
            review,
        )
        .await
}

#[tauri::command]
pub(crate) async fn github_unlink_remote(
    foundation: State<'_, Foundation>,
    local_id: i64,
    remote: String,
    version: String,
) -> Result<(), ResourceError> {
    github::unlink_remote(
        foundation.database().map_err(|_| GithubError::Storage)?,
        local_id,
        &remote,
        &version,
    )
    .await
}

#[tauri::command]
pub(crate) async fn github_disconnect(
    foundation: State<'_, Foundation>,
    github_id: i64,
) -> Result<AccountStatus, GithubError> {
    foundation
        .github
        .disconnect(
            foundation.database().map_err(|_| GithubError::Storage)?,
            app(&foundation)?,
            github_id,
        )
        .await
}

#[tauri::command]
pub(crate) async fn github_revocation_reviewed(
    foundation: State<'_, Foundation>,
    github_id: i64,
) -> Result<AccountStatus, GithubError> {
    foundation
        .github
        .acknowledge_revocation(
            foundation.database().map_err(|_| GithubError::Storage)?,
            app(&foundation)?,
            github_id,
        )
        .await
}

#[tauri::command]
pub(crate) async fn github_open_authorizations() -> Result<(), GithubError> {
    open_github(github::AUTHORIZATIONS_URI).await
}

#[tauri::command]
pub(crate) async fn github_open_verification(
    foundation: State<'_, Foundation>,
    session_id: String,
) -> Result<(), GithubError> {
    if !foundation.github.can_open(&session_id).await {
        return Err(GithubError::Stale);
    }
    open_github(github::VERIFICATION_URI).await
}

async fn open_github(url: &'static str) -> Result<(), GithubError> {
    let mut command = if cfg!(windows) {
        let system = std::env::var_os("SystemRoot").ok_or(GithubError::Unavailable)?;
        let mut command = tokio::process::Command::new(
            std::path::PathBuf::from(system).join("System32/rundll32.exe"),
        );
        command.args(["url.dll,FileProtocolHandler", url]);
        command
    } else {
        let mut command = tokio::process::Command::new("/usr/bin/xdg-open");
        command.arg(url);
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
