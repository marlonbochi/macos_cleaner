//! macOS integration commands: Finder and permissions (FR-010, FR-011).

use std::path::PathBuf;

use cleaner_infra::macos;
use tauri::State;

use crate::dto::PermissionsStatusDto;
use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub fn reveal_in_finder(path: String) -> CommandResult<()> {
    Ok(macos::reveal_in_finder(&PathBuf::from(path))?)
}

#[tauri::command]
pub fn get_permissions_status(state: State<'_, AppState>) -> PermissionsStatusDto {
    PermissionsStatusDto {
        full_disk_access: macos::has_full_disk_access(&state.home),
    }
}

#[tauri::command]
pub fn open_full_disk_access_settings() -> CommandResult<()> {
    Ok(macos::open_full_disk_access_settings()?)
}
