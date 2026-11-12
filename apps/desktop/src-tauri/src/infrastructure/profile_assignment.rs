use std::path::Path;

use serde::Serialize;
use sqlx::SqlitePool;

use super::{
    config_apply::{self, ApplyError},
    config_preview, conflicts, inventory, native_file,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssignmentPreview {
    pub(crate) confirmation: Option<String>,
    global: config_preview::ConfigPreview,
    repositories: Vec<RepositoryPlan>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        identity::IdentityInput,
        infrastructure::{discovery, git, identities, inventory_scan::ScanJobs, storage},
    };
    use std::{env, fs, process::Command};

    #[test]
    fn assigned_profiles_replace_inherited_keys_and_restore_in_order() {
        const ROOT: &str = "GITPILOT_ASSIGNMENT_TEST_ROOT";
        let Some(root) = env::var_os(ROOT) else {
            let home = tempfile::tempdir().unwrap();
            // Windows runners may expose TEMP through an 8.3 alias. Git and the
            // guarded writer must see the same canonical home, not two aliases.
            let home_path =
                tauri::async_runtime::block_on(discovery::directory_path(home.path())).unwrap();
            let home_path = Path::new(&home_path);
            let mut child = Command::new(env::current_exe().unwrap());
            for (key, _) in env::vars_os() {
                if key
                    .to_string_lossy()
                    .to_ascii_uppercase()
                    .starts_with("GIT_")
                {
                    child.env_remove(key);
                }
            }
            let status = child.args(["--exact", "infrastructure::profile_assignment::tests::assigned_profiles_replace_inherited_keys_and_restore_in_order", "--nocapture"])
                .env(ROOT, home_path).env("HOME", home_path).env("USERPROFILE", home_path).env("XDG_CONFIG_HOME", home_path.join("xdg")).env("GIT_CONFIG_NOSYSTEM", "1").status().unwrap();
            assert!(status.success());
            return;
        };
        tauri::async_runtime::block_on(async {
            let root = std::path::PathBuf::from(
                discovery::directory_path(Path::new(&root)).await.unwrap(),
            );
            let app = root.join("app");
            let pool = storage::open(&app).await.unwrap();
            let global = root.join(".gitconfig");
            let original = "[user]\nname = Global\nemail = global@example.test\nsigningkey = other-key\n[commit]\ngpgsign = true\n[core]\nsshCommand = other-identity-ssh\n";
            fs::write(&global, original).unwrap();
            native_file::private_directory(&app.join("native")).unwrap();
            let workspace = root.join("Proyectos [equipo]");
            fs::create_dir(&workspace).unwrap();
            let repo = workspace.join("repo");
            fs::create_dir(&repo).unwrap();
            let exe = git::executable().unwrap();
            let run = |args: &[&str]| {
                let output = Command::new(&exe)
                    .current_dir(&repo)
                    .args(args)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                String::from_utf8(output.stdout).unwrap().trim().to_owned()
            };
            run(&["init", "--quiet"]);
            let shared = root.join("shared.config");
            fs::write(&shared, "[user]\nname = Previous\nemail = previous@example.test\n[core]\nsshCommand = previous-ssh\n[gpg]\nformat = ssh\n").unwrap();
            run(&["config", "include.path", shared.to_str().unwrap()]);
            let local_original = fs::read(repo.join(".git/config")).unwrap();
            let key = root.join("key with spaces %n 'quoted' $(exit 99)");
            fs::write(&key, "synthetic key reference").unwrap();
            let input = |label: &str, with_keys: bool| IdentityInput {
                label: label.into(),
                git_name: "Selected Author".into(),
                email: "selected@example.test".into(),
                signing_key: with_keys.then(|| key.to_str().unwrap().into()),
                signing_format: with_keys.then(|| "ssh".into()),
                ssh_key: with_keys.then(|| key.to_str().unwrap().into()),
                signing_required: false,
            };
            let first = identities::create(&pool, input("With keys", true))
                .await
                .unwrap();
            let second = identities::create(&pool, input("Without keys", false))
                .await
                .unwrap();
            let directory_id = inventory::add(&pool, workspace.to_str().unwrap())
                .await
                .unwrap();
            ScanJobs::default()
                .run(&pool, directory_id, "assignment-test".into(), |_| {})
                .await
                .unwrap();
            let directory = inventory::directory(&pool, directory_id).await.unwrap();
            inventory::assign(&pool, false, directory_id, directory.revision, Some(first))
                .await
                .unwrap();
            let first_plan = prepare(&pool, &app, directory_id, false).await.unwrap();
            assert!(
                first_plan.confirmation.is_some(),
                "{}",
                serde_json::to_string(&first_plan).unwrap()
            );
            assert_eq!(fs::read_to_string(&global).unwrap(), original);
            assert_eq!(fs::read(repo.join(".git/config")).unwrap(), local_original);
            fs::remove_file(&key).unwrap();
            assert!(matches!(
                prepare(&pool, &app, directory_id, false).await,
                Err(ApplyError::KeyUnavailable)
            ));
            fs::write(&key, "synthetic key reference").unwrap();
            apply(
                &pool,
                &app,
                directory_id,
                false,
                first_plan.confirmation.as_deref().unwrap(),
            )
            .await
            .unwrap();
            assert_eq!(run(&["config", "user.name"]), "Selected Author");
            assert_eq!(run(&["config", "commit.gpgsign"]), "false");
            assert!(run(&["config", "core.sshCommand"]).contains("key with spaces"));
            let first_ssh = run(&["config", "core.sshCommand"]);
            let first_alias = format!("alias.profile-ssh=!{first_ssh} -G example.invalid");
            let resolved = run(&["-c", &first_alias, "profile-ssh"]);
            assert!(resolved.contains("identityfile none"));
            assert_eq!(
                resolved
                    .lines()
                    .filter(|line| line.starts_with("identityfile "))
                    .count(),
                2
            );
            let previous_global = config_apply::status(&pool, None).await.unwrap().unwrap();
            let directory = inventory::directory(&pool, directory_id).await.unwrap();
            inventory::assign(&pool, false, directory_id, directory.revision, Some(second))
                .await
                .unwrap();
            let plan = prepare(&pool, &app, directory_id, false).await.unwrap();
            apply(
                &pool,
                &app,
                directory_id,
                false,
                first_plan.confirmation.as_deref().unwrap(),
            )
            .await
            .unwrap_err();
            apply(
                &pool,
                &app,
                directory_id,
                false,
                plan.confirmation.as_deref().unwrap(),
            )
            .await
            .unwrap();
            assert_eq!(run(&["config", "user.signingkey"]), "");
            assert_eq!(run(&["config", "commit.gpgsign"]), "false");
            let ssh = run(&["config", "core.sshCommand"]);
            assert!(ssh.contains("PubkeyAuthentication=no"));
            assert!(!ssh.contains("key with spaces"));
            assert_eq!(
                config_apply::restore(
                    &pool,
                    &app,
                    previous_global.confirmation.as_deref().unwrap(),
                    Some(previous_global.id)
                )
                .await,
                Err(ApplyError::ActiveOperation)
            );
            let old_status = serde_json::to_value(
                config_apply::status(&pool, Some(previous_global.id))
                    .await
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(old_status["state"], "superseded");
            let alias = format!("alias.profile-ssh=!{ssh} -G example.invalid");
            let resolved = run(&["-c", &alias, "profile-ssh"]);
            assert!(resolved.contains("identityfile none"));
            assert!(resolved.contains("identityagent none"));
            assert!(
                resolved.contains("pubkeyauthentication false")
                    || resolved.contains("pubkeyauthentication no")
            );
            run(&["commit", "--allow-empty", "-m", "isolated author"]);
            assert_eq!(
                run(&["log", "-1", "--format=%an <%ae>|%cn <%ce>"]),
                "Selected Author <selected@example.test>|Selected Author <selected@example.test>"
            );
            // Future repositories inherit the latest profile with GitPilot closed.
            let future = workspace.join("future");
            fs::create_dir(&future).unwrap();
            assert!(Command::new(&exe)
                .current_dir(&future)
                .args(["init", "--quiet"])
                .status()
                .unwrap()
                .success());
            let output = Command::new(&exe)
                .current_dir(&future)
                .args(["config", "core.sshCommand"])
                .output()
                .unwrap();
            assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), ssh);
            // An interrupted final publication rolls back to the preceding file
            // and keeps that older application's restoration usable.
            let latest = config_apply::status(&pool, None).await.unwrap().unwrap();
            sqlx::query("UPDATE config_operations SET state = 'preparing' WHERE rowid = ?")
                .bind(latest.id)
                .execute(&pool)
                .await
                .unwrap();
            config_apply::reconcile(&pool, &app).await.unwrap();
            let fresh = config_preview::prepare(&pool, &app).await.unwrap();
            config_apply::apply(&pool, &app, fresh.confirmation().unwrap(), None, None)
                .await
                .unwrap();
            let repo_id = plan.repositories[0].id;
            // Four operations: two local + two global; restore newest first.
            for _ in 0..2 {
                let status = config_apply::status(&pool, None).await.unwrap().unwrap();
                config_apply::restore(
                    &pool,
                    &app,
                    status.confirmation.as_deref().unwrap(),
                    Some(status.id),
                )
                .await
                .unwrap();
                let operation_id: i64 = sqlx::query_scalar("SELECT rowid FROM config_operations WHERE repository_id = ? AND state = 'applied' ORDER BY rowid DESC LIMIT 1").bind(repo_id).fetch_one(&pool).await.unwrap();
                let status = config_apply::status(&pool, Some(operation_id))
                    .await
                    .unwrap()
                    .unwrap();
                config_apply::restore(
                    &pool,
                    &app,
                    status.confirmation.as_deref().unwrap(),
                    Some(status.id),
                )
                .await
                .unwrap();
            }
            assert_eq!(fs::read_to_string(&global).unwrap(), original);
            assert_eq!(fs::read(repo.join(".git/config")).unwrap(), local_original);
            assert_eq!(fs::read_to_string(&shared).unwrap(), "[user]\nname = Previous\nemail = previous@example.test\n[core]\nsshCommand = previous-ssh\n[gpg]\nformat = ssh\n");
            pool.close().await;
        });
    }
}

#[derive(Serialize)]
struct RepositoryPlan {
    id: i64,
    path: String,
    preview: conflicts::LocalPreview,
}

pub(crate) async fn prepare(
    pool: &SqlitePool,
    app: &Path,
    id: i64,
    repository: bool,
) -> Result<AssignmentPreview, ApplyError> {
    let repositories = if repository {
        vec![inventory::repository(pool, id)
            .await
            .map_err(|_| ApplyError::StalePreview)?]
    } else {
        let root = inventory::directory(pool, id)
            .await
            .map_err(|_| ApplyError::StalePreview)?;
        let all: Vec<crate::repository::Repository> = sqlx::query_as("SELECT r.id, r.path, r.kind, r.git_dir, r.common_dir, r.revision, 1 AS present, rule.identity_id FROM repositories r LEFT JOIN identity_rules rule ON rule.repository_id = r.id WHERE EXISTS (SELECT 1 FROM directory_repositories dr WHERE dr.repository_id = r.id AND dr.present = 1) ORDER BY r.path").fetch_all(pool).await?;
        all.into_iter()
            .filter(|repo| Path::new(&repo.path).starts_with(&root.path))
            .collect()
    };
    let mut plans = Vec::new();
    for repo in repositories {
        let preview = conflicts::prepare(pool, repo.id).await?;
        plans.push(RepositoryPlan {
            id: repo.id,
            path: repo.path,
            preview,
        });
    }
    let global = config_preview::prepare(pool, app)
        .await
        .map_err(|_| ApplyError::StalePreview)?;
    let mut result = AssignmentPreview {
        confirmation: None,
        global,
        repositories: plans,
    };
    if result.global.confirmation().is_some() {
        let bytes = serde_json::to_vec(&(id, repository, &result))
            .map_err(|_| ApplyError::StorageUnavailable)?;
        result.confirmation = Some(hex::encode(native_file::digest(&bytes)));
    }
    Ok(result)
}

pub(crate) async fn apply(
    pool: &SqlitePool,
    app: &Path,
    id: i64,
    repository: bool,
    confirmation: &str,
) -> Result<(), ApplyError> {
    let plan = prepare(pool, app, id, repository).await?;
    if plan.confirmation.as_deref() != Some(confirmation) {
        return Err(ApplyError::StalePreview);
    }
    // Every file uses the existing verified backup/journal protocol. If a later file
    // fails, earlier verified operations stay visible and restorable in Activity.
    for repo in &plan.repositories {
        config_apply::apply(pool, app, &repo.preview.confirmation, Some(repo.id), None).await?;
    }
    // Local overrides change the simulation, but neither the approved global bytes
    // nor its destination may change between the review and this final step.
    let global = config_preview::prepare(pool, app)
        .await
        .map_err(|_| ApplyError::StalePreview)?;
    let approved = plan
        .global
        .candidate
        .as_ref()
        .ok_or(ApplyError::StalePreview)?;
    let current = global.candidate.as_ref().ok_or(ApplyError::StalePreview)?;
    if current.target != approved.target
        || current.before != approved.before
        || current.append != approved.append
    {
        return Err(ApplyError::StalePreview);
    }
    config_apply::apply(
        pool,
        app,
        global.confirmation().ok_or(ApplyError::StalePreview)?,
        None,
        None,
    )
    .await?;
    for repo in &plan.repositories {
        repo.preview.verify().await?;
    }
    Ok(())
}
