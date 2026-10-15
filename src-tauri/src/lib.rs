//! macOS Cleaner desktop app: Tauri composition root and IPC commands.
//!
//! This crate only wires things together. Business rules live in `cleaner-core`,
//! I/O in `cleaner-infra` (see `specs/001-disk-space-cleaner/plan.md`).

mod commands;
pub mod dto;
pub mod error;
mod state;

use tauri::Manager;
use tracing_subscriber::EnvFilter;

use crate::state::AppState;

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,macos_cleaner_lib=debug,cleaner_core=debug"));
    // Ignore the error if a subscriber is already installed (e.g. in tests).
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let paths = app.path();
    let bundle_id = app.config().identifier.clone();
    let state = AppState::build(&paths.app_data_dir()?, paths.home_dir()?, &bundle_id)?;
    app.manage(state);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            commands::scan::get_scan_defaults,
            commands::scan::start_scan,
            commands::scan::cancel_scan,
            commands::scan::get_latest_scan,
            commands::catalog::list_files,
            commands::catalog::list_folder_children,
            commands::system::reveal_in_finder,
            commands::system::get_permissions_status,
            commands::system::open_full_disk_access_settings,
            commands::analysis::analyze_file,
            commands::analysis::analyze_files,
            commands::analysis::get_analyses,
            commands::analysis::get_ai_settings,
            commands::analysis::save_ai_settings,
            commands::analysis::test_ai_connection,
            commands::analysis::list_ai_models,
            commands::diagnosis::get_ai_diagnosis,
            commands::diagnosis::run_ai_diagnosis,
            commands::preferences::get_preferences,
            commands::preferences::set_locale,
            commands::uninstall::get_app_footprint,
            commands::uninstall::uninstall_app,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        tracing::error!(error = %e, "application terminated with an error");
        std::process::exit(1);
    }
}
