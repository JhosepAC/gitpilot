mod application;
mod commands;
mod error;
mod identity;
mod infrastructure;

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
        ])
        .run(tauri::generate_context!())
}
