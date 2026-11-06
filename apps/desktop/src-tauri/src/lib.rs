mod application;
mod commands;
mod error;
mod identity;
mod infrastructure;
mod repository;
mod repository_commands;

use tauri::Manager;
use tracing_subscriber::{filter::Targets, layer::SubscriberExt, util::SubscriberInitExt, Layer};

pub fn run() -> tauri::Result<()> {
    let logging = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_writer(std::io::stderr)
        .with_filter(Targets::new().with_target("gitpilot_desktop", tracing::Level::WARN));
    let _ = tracing_subscriber::registry().with(logging).try_init();

    tauri::Builder::default()
        .setup(|app| {
            let directory = app.path().app_local_data_dir().ok();
            let foundation =
                tauri::async_runtime::block_on(application::Foundation::initialize(directory));
            if let (Ok(pool), Some(directory)) = (foundation.database(), &foundation.directory) {
                let pool = pool.clone();
                let directory = directory.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) =
                        infrastructure::config_apply::reconcile(&pool, &directory).await
                    {
                        tracing::warn!(?error, "native configuration reconciliation pending");
                    }
                });
            }
            app.manage(foundation);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::foundation_status,
            commands::list_identities,
            commands::create_identity,
            commands::update_identity,
            commands::duplicate_identity,
            commands::set_identity_default,
            commands::delete_identity,
            commands::detect_git_identity,
            repository_commands::list_directories,
            repository_commands::add_directory,
            repository_commands::remove_directory,
            repository_commands::list_directory_contents,
            repository_commands::set_directory_exclusion,
            repository_commands::scan_directory,
            repository_commands::set_inventory_identity,
            repository_commands::diagnose_repository,
            repository_commands::preview_native_config,
            repository_commands::native_config_status,
            repository_commands::confirm_native_config,
            repository_commands::native_activity,
            repository_commands::setup_status,
            repository_commands::keep_repository_identity,
            repository_commands::preview_local_identity,
        ])
        .run(tauri::generate_context!())
}
