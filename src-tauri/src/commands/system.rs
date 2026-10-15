//! System integration commands: reveal in the file manager and permissions (FR-010, FR-011).

use std::path::PathBuf;

use tauri::State;

use crate::dto::PermissionsStatusDto;
use crate::error::{AppError, CommandResult, ErrorCode};
use crate::state::AppState;

/// Selects the item in Finder, Explorer or the Linux file manager.
#[tauri::command]
pub fn reveal_in_finder(path: String) -> CommandResult<()> {
    tauri_plugin_opener::reveal_item_in_dir(PathBuf::from(path))
        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))
}

/// Only macOS gates reading other apps' data behind "Full Disk Access".
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn get_permissions_status(state: State<'_, AppState>) -> PermissionsStatusDto {
    use cleaner_infra::macos;
    PermissionsStatusDto {
        full_disk_access: macos::has_full_disk_access(&state.home),
        target: macos::permission_target().map(Into::into),
    }
}

/// Windows and Linux have no "Full Disk Access" switch: unreadable folders are simply listed.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn get_permissions_status(_state: State<'_, AppState>) -> PermissionsStatusDto {
    PermissionsStatusDto {
        full_disk_access: true,
        target: None,
    }
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn open_full_disk_access_settings() -> CommandResult<()> {
    Ok(cleaner_infra::macos::open_full_disk_access_settings()?)
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn open_full_disk_access_settings() -> CommandResult<()> {
    Err(AppError::invalid_input(
        "Full Disk Access only exists on macOS",
    ))
}
