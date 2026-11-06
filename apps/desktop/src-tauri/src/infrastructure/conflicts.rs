use std::path::Path;

use serde::Serialize;
use sqlx::SqlitePool;

use crate::repository::{
    ConfigValue, IdentityDiagnosis, InventoryError, ObservedIdentity, Repository,
};

use super::{
    config_apply::ApplyError,
    config_preview::{self, Candidate},
    discovery, git, inventory, native_file,
};

pub(crate) async fn diagnose(
    pool: &SqlitePool,
    repo: &Repository,
) -> Result<IdentityDiagnosis, InventoryError> {
    let expected = inventory::expected(pool, repo).await?;
    let (observed, observation_error) = if repo.present {
        match discovery::observe(&repo.path, &repo.git_dir).await {
            Ok(observed) => (Some(observed), None),
            Err(error) => (None, Some(error)),
        }
    } else {
        (None, None)
    };
    let mut fingerprint = None;
    let mut kept = false;
    if let Some((expected, observed)) = expected.as_ref().zip(observed.as_ref()) {
        if differs(observed.name.as_ref(), &expected.git_name)
            || differs(observed.email.as_ref(), &expected.email)
        {
            let record =
                serde_json::to_vec(&(&repo.path, &repo.git_dir, repo.revision, expected, observed))
                    .map_err(|_| InventoryError::StorageUnavailable)?;
            let hash = hex::encode(native_file::digest(&record));
            kept = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM conflict_decisions WHERE repository_id = ? AND fingerprint = ?)")
                .bind(repo.id).bind(&hash).fetch_one(pool).await?;
            fingerprint = Some(hash);
        }
    }
    Ok(IdentityDiagnosis {
        expected,
        observed,
        observation_error,
        conflict: fingerprint.is_some(),
        conflict_fingerprint: fingerprint,
        kept,
    })
}

fn differs(current: Option<&ConfigValue>, expected: &str) -> bool {
    current.is_none_or(|current| current.value != expected)
}

pub(crate) async fn keep(
    pool: &SqlitePool,
    id: i64,
    fingerprint: &str,
) -> Result<(), InventoryError> {
    let repo = inventory::repository(pool, id).await?;
    let diagnosis = diagnose(pool, &repo).await?;
    if diagnosis.conflict_fingerprint.as_deref() != Some(fingerprint) {
        return Err(InventoryError::Conflict);
    }
    sqlx::query(
        "INSERT INTO conflict_decisions (repository_id, path, fingerprint)
                 VALUES (?, ?, ?) ON CONFLICT(repository_id, fingerprint) DO NOTHING",
    )
    .bind(id)
    .bind(&repo.path)
    .bind(fingerprint)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalPreview {
    target: String,
    pub(super) confirmation: String,
    append: String,
    name: String,
    email: String,
    #[serde(skip)]
    pub(super) candidate: Candidate,
    #[serde(skip)]
    repository: Repository,
}

fn direct_local(value: &ConfigValue, target: &Path) -> bool {
    value.scope == "local"
        && value.origin.strip_prefix("file:").is_some_and(|origin| {
            origin == ".git/config" || origin == r".git\config" || Path::new(origin) == target
        })
}

fn commit_identity_is(observed: &ObservedIdentity, name: &str, email: &str) -> bool {
    let identity = format!("{name} <{email}>");
    observed.author.as_ref() == Some(&identity) && observed.committer.as_ref() == Some(&identity)
}

pub(crate) async fn prepare(pool: &SqlitePool, id: i64) -> Result<LocalPreview, ApplyError> {
    let repo = inventory::repository(pool, id)
        .await
        .map_err(|_| ApplyError::StalePreview)?;
    let workdir = Path::new(&repo.path);
    let git_dir = Path::new(&repo.git_dir);
    if !repo.present
        || repo.kind != "ordinary"
        || repo.git_dir != repo.common_dir
        || workdir.join(".git") != git_dir
        || [
            "GIT_CONFIG_GLOBAL",
            "GIT_CONFIG",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
            "GIT_DIR",
            "GIT_COMMON_DIR",
            "GIT_WORK_TREE",
            "GIT_AUTHOR_NAME",
            "GIT_AUTHOR_EMAIL",
            "GIT_COMMITTER_NAME",
            "GIT_COMMITTER_EMAIL",
        ]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
    {
        return Err(ApplyError::UnsafeTarget);
    }
    let exe = git::executable().ok_or(ApplyError::UnsafeTarget)?;
    let top = config_preview::git_line(&exe, workdir, &["rev-parse", "--show-toplevel"])
        .await
        .ok_or(ApplyError::UnsafeTarget)?;
    if !discovery::directory_path(Path::new(&top))
        .await
        .is_ok_and(|path| path == repo.path)
    {
        return Err(ApplyError::UnsafeTarget);
    }
    let version = config_preview::git_line(&exe, workdir, &["--version"])
        .await
        .ok_or(ApplyError::UnsafeTarget)?;
    if !config_preview::compatible(&version)
        || !config_preview::worktrees(&exe, workdir)
            .await
            .is_ok_and(|paths| paths.is_empty())
    {
        return Err(ApplyError::UnsafeTarget);
    }
    let target = git_dir.join("config");
    let parent = native_file::parent_id(&target).map_err(|_| ApplyError::UnsafeTarget)?;
    let snapshot = native_file::snapshot(&target)
        .map_err(|_| ApplyError::UnsafeTarget)?
        .ok_or(ApplyError::UnsafeTarget)?;
    let diagnosis = diagnose(pool, &repo)
        .await
        .map_err(|_| ApplyError::StalePreview)?;
    let (Some(expected), Some(observed), Some(fingerprint)) = (
        diagnosis.expected,
        diagnosis.observed,
        diagnosis.conflict_fingerprint,
    ) else {
        return Err(ApplyError::StalePreview);
    };
    let (Some(name), Some(email)) = (&observed.name, &observed.email) else {
        return Err(ApplyError::UnsafeTarget);
    };
    let fields = [
        (name, "name", &expected.git_name),
        (email, "email", &expected.email),
    ];
    if !commit_identity_is(&observed, &name.value, &email.value)
        || fields
            .iter()
            .any(|(field, _, expected)| field.value != **expected && !direct_local(field, &target))
    {
        return Err(ApplyError::UnsafeTarget);
    }
    let mut append = String::from("[user]\n");
    for (field, key, value) in fields {
        if field.value != *value {
            append.push_str(&format!("\t{key} = {}\n", config_preview::quote(value)));
        }
    }
    let record = serde_json::to_vec(&(&target, parent, &snapshot.stamp, fingerprint, &append))
        .map_err(|_| ApplyError::StorageUnavailable)?;
    Ok(LocalPreview {
        target: target.to_str().ok_or(ApplyError::UnsafeTarget)?.into(),
        confirmation: hex::encode(native_file::digest(&record)),
        append: append.clone(),
        name: expected.git_name,
        email: expected.email,
        candidate: Candidate {
            target,
            parent,
            before: Some(snapshot.stamp),
            generated: None,
            append,
        },
        repository: repo,
    })
}

impl LocalPreview {
    pub(super) async fn verify(&self) -> Result<(), ApplyError> {
        let exe = git::executable().ok_or(ApplyError::VerificationFailed)?;
        if !config_preview::worktrees(&exe, Path::new(&self.repository.path))
            .await
            .is_ok_and(|paths| paths.is_empty())
        {
            return Err(ApplyError::VerificationFailed);
        }
        let observed = discovery::observe(&self.repository.path, &self.repository.git_dir)
            .await
            .map_err(|_| ApplyError::VerificationFailed)?;
        if differs(observed.name.as_ref(), &self.name)
            || differs(observed.email.as_ref(), &self.email)
            || !commit_identity_is(&observed, &self.name, &self.email)
        {
            return Err(ApplyError::VerificationFailed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_scope_does_not_authorize_editing_an_included_file() {
        let target = Path::new("repo").join(".git").join("config");
        let record = |scope: &str, origin: &str| ConfigValue {
            value: "fixture@example.test".into(),
            scope: scope.into(),
            origin: origin.into(),
        };
        assert!(direct_local(&record("local", "file:.git/config"), &target));
        assert!(!direct_local(
            &record("local", "file:../shared.config"),
            &target
        ));
        assert!(!direct_local(
            &record("global", "file:.git/config"),
            &target
        ));
        assert!(!direct_local(
            &record("worktree", "file:.git/config.worktree"),
            &target
        ));
    }
}
