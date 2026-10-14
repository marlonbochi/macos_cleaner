//! Catalog browsing commands (US1).

use std::path::PathBuf;

use cleaner_core::domain::scan::ScanId;
use cleaner_core::ports::repository::FileQuery;
use tauri::State;

use crate::dto::{FileEntryDto, FileFilterDto, FileSortDto, PageDto};
use crate::error::{AppError, CommandResult};
use crate::state::AppState;

#[tauri::command]
pub fn list_files(
    state: State<'_, AppState>,
    scan_id: i64,
    offset: u64,
    limit: u32,
    filter: Option<FileFilterDto>,
    sort: Option<FileSortDto>,
) -> CommandResult<PageDto<FileEntryDto>> {
    let query = FileQuery::new(
        filter.unwrap_or_default().into(),
        sort.unwrap_or_default().into(),
        offset,
        limit,
    );
    let page = state.catalog.list_files(ScanId(scan_id), &query)?;
    Ok(PageDto::from_page(page, FileEntryDto::from))
}

#[tauri::command]
pub fn list_folder_children(
    state: State<'_, AppState>,
    scan_id: i64,
    path: String,
    offset: u64,
    limit: u32,
) -> CommandResult<PageDto<FileEntryDto>> {
    let folder = PathBuf::from(path);
    if !folder.is_absolute() {
        return Err(AppError::invalid_input("folder path must be absolute"));
    }
    let page = state
        .catalog
        .list_children(ScanId(scan_id), folder, offset, limit)?;
    Ok(PageDto::from_page(page, FileEntryDto::from))
}
