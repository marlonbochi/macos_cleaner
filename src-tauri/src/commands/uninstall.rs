//! Complete uninstall: app data, Keychain entries and the app itself (feature 009).

use std::time::Duration;

use tauri::{AppHandle, State};

use crate::dto::{AppFootprintDto, UninstallReportDto};
use crate::error::{AppError, CommandResult, ErrorCode};
use crate::state::AppState;

/// Time for the UI to show the result before the app quits.
const QUIT_DELAY: Duration = Duration::from_secs(3);

/// What a complete uninstall would remove right now, with sizes. Changes nothing.
#[tauri::command]
pub async fn get_app_footprint(state: State<'_, AppState>) -> CommandResult<AppFootprintDto> {
    let service = state.uninstall.clone();
    tauri::async_runtime::spawn_blocking(move || service.footprint())
        .await
        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?
        .map(Into::into)
        .map_err(Into::into)
}

/// Removes the API keys and moves the app's data and the app itself to the Trash, then quits.
/// `confirmation` must be the exact phrase shown to the user (checked again in the core).
#[tauri::command]
pub async fn uninstall_app(
    app: AppHandle,
    state: State<'_, AppState>,
    confirmation: String,
) -> CommandResult<UninstallReportDto> {
    let service = state.uninstall.clone();
    let report = tauri::async_runtime::spawn_blocking(move || service.uninstall(&confirmation))
        .await
        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))??;
    // Once anything is gone the app can no longer work reliably, so it quits.
    let quitting = !report.trashed.is_empty() || report.keys_removed > 0;
    if quitting {
        std::thread::spawn(move || {
            std::thread::sleep(QUIT_DELAY);
            app.exit(0);
        });
    }
    Ok(UninstallReportDto::new(report, quitting))
}
