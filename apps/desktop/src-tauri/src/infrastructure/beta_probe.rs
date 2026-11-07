use std::{
    env, fs,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

use super::{conflicts, discovery, identities, inventory, inventory_scan::ScanJobs, storage};
use crate::{identity::IdentityInput, repository::ScanPhase};

const ISOLATED_ROOT: &str = "GITPILOT_BETA_PROBE_ROOT";
const REPOSITORIES: usize = 100;
const DIAGNOSES: usize = 30;

#[test]
#[ignore = "measures local filesystem and Git performance on demand"]
fn scan_and_diagnosis_budget() {
    let Some(root) = env::var_os(ISOLATED_ROOT) else {
        let home = tempfile::tempdir().unwrap();
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
        let status = child
            .args([
                "--exact",
                "infrastructure::beta_probe::scan_and_diagnosis_budget",
                "--ignored",
                "--nocapture",
            ])
            .env(ISOLATED_ROOT, home.path())
            .env("HOME", home.path())
            .env("USERPROFILE", home.path())
            .env("XDG_CONFIG_HOME", home.path().join("xdg"))
            .env("GIT_CONFIG_GLOBAL", home.path().join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    };
    tauri::async_runtime::block_on(measure(Path::new(&root)));
}

async fn measure(root: &Path) {
    let workspace = root.join("Áreas de trabajo");
    for index in 0..REPOSITORIES {
        let git = workspace
            .join(format!("group-{:02}", index / 10))
            .join(format!("repo-{index:02}"))
            .join(".git");
        fs::create_dir_all(git.join("objects")).unwrap();
        fs::create_dir_all(git.join("refs/heads")).unwrap();
        fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(
            git.join("config"),
            "[core]\nrepositoryformatversion = 0\n[user]\nname = Beta Fixture\nemail = fixture@example.test\n",
        )
        .unwrap();
    }
    for index in 0..10 {
        let assets = workspace.join(format!("assets-{index:02}"));
        fs::create_dir_all(&assets).unwrap();
        for entry in 0..480 {
            fs::write(assets.join(format!("file-{entry:03}")), []).unwrap();
        }
    }

    let pool = storage::open(&root.join("database")).await.unwrap();
    let profile = identities::create(
        &pool,
        IdentityInput {
            label: "Beta profile".into(),
            git_name: "Expected Fixture".into(),
            email: "expected@example.test".into(),
        },
    )
    .await
    .unwrap();
    identities::set_default(&pool, profile, 1, true)
        .await
        .unwrap();
    let path = discovery::directory_path(&workspace).await.unwrap();
    let id = inventory::add(&pool, &path).await.unwrap();
    let jobs = ScanJobs::default();
    let start = Instant::now();
    let mut saving = None;
    let report = jobs
        .run(&pool, id, "beta-full".into(), |progress| {
            if matches!(progress.phase, ScanPhase::Saving) {
                saving = Some(start.elapsed());
            }
        })
        .await
        .unwrap();
    let scan = start.elapsed();
    let walking = saving.unwrap();
    assert_eq!(report.found, REPOSITORIES);
    assert!(!report.partial);
    let contents = inventory::contents(&pool, id).await.unwrap();
    assert_eq!(contents.repositories.len(), REPOSITORIES);

    let mut samples = Vec::with_capacity(DIAGNOSES);
    for entry in contents.repositories.iter().take(DIAGNOSES) {
        let start = Instant::now();
        let repo = inventory::repository(&pool, entry.id).await.unwrap();
        let diagnosis = conflicts::diagnose(&pool, &repo).await.unwrap();
        samples.push(start.elapsed());
        assert!(diagnosis.conflict);
        let observed = diagnosis.observed.unwrap();
        assert_eq!(observed.email.unwrap().value, "fixture@example.test");
    }
    samples.sort_unstable();
    let p95 = samples[(DIAGNOSES * 95).div_ceil(100) - 1];

    let mut requested = None;
    let cancelled = jobs
        .run(&pool, id, "beta-cancel".into(), |progress| {
            if requested.is_none() && matches!(progress.phase, ScanPhase::Walking) {
                requested = Some(Instant::now());
                assert!(jobs.cancel("beta-cancel"));
            }
        })
        .await
        .unwrap();
    let cancellation = requested.unwrap().elapsed();
    assert!(cancelled.cancelled && cancelled.partial);
    assert_eq!(
        inventory::contents(&pool, id)
            .await
            .unwrap()
            .repositories
            .len(),
        REPOSITORIES
    );
    pool.close().await;

    println!(
        "beta_probe repositories={} entries={} scan_ms={} publish_ms={} diagnosis_p95_ms={} cancel_ms={}",
        report.found,
        report.visited,
        scan.as_millis(),
        (scan - walking).as_millis(),
        p95.as_millis(),
        cancellation.as_millis()
    );
    assert!(scan <= Duration::from_secs(15), "scan: {scan:?}");
    assert!(p95 <= Duration::from_secs(1), "diagnosis p95: {p95:?}");
    assert!(
        cancellation <= Duration::from_secs(1),
        "cancellation: {cancellation:?}"
    );
}
