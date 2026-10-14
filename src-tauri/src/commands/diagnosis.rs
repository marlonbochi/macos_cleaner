//! AI diagnosis of the largest items of a scan (feature 008).

use cleaner_core::domain::scan::ScanId;
use tauri::State;

use crate::dto::DiagnosisItemDto;
use crate::error::{AppError, CommandResult, ErrorCode};
use crate::state::AppState;

/// The diagnosed items with their stored analyses; never calls the AI.
#[tauri::command]
pub fn get_ai_diagnosis(
    state: State<'_, AppState>,
    scan_id: i64,
) -> CommandResult<Vec<DiagnosisItemDto>> {
    Ok(state
        .diagnosis
        .latest(ScanId(scan_id))?
        .into_iter()
        .map(Into::into)
        .collect())
}

/// Sends the largest items to the configured AI (in batches; unchanged items come from the
/// cache). Runs off the main thread: it can take a minute.
#[tauri::command]
pub async fn run_ai_diagnosis(
    state: State<'_, AppState>,
    scan_id: i64,
    force: bool,
) -> CommandResult<Vec<DiagnosisItemDto>> {
    let service = state.diagnosis.clone();
    tauri::async_runtime::spawn_blocking(move || service.run(ScanId(scan_id), force))
        .await
        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
