//! AI analysis and AI settings commands (US2).

use cleaner_core::domain::ai::AiProviderKind;
use cleaner_core::domain::file_entry::FileId;
use cleaner_core::services::ai_settings_service::ApiKeyChange;
use tauri::State;

use crate::dto::{
    AiModelDto, AiProviderIdDto, AiProviderInfoDto, AiSettingsDto, AnalysisDto, ConnectionTestDto,
    SaveAiSettingsInput,
};
use crate::error::{AppError, CommandResult, ErrorCode};
use crate::state::AppState;

/// Runs off the main thread: an AI call can take tens of seconds.
#[tauri::command]
pub async fn analyze_file(
    state: State<'_, AppState>,
    file_id: i64,
    force: bool,
) -> CommandResult<AnalysisDto> {
    let service = state.analysis.clone();
    tauri::async_runtime::spawn_blocking(move || service.analyze(FileId(file_id), force))
        .await
        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?
        .map(AnalysisDto::from)
        .map_err(Into::into)
}

/// Upper bound per call; the service still splits it into AI batches of `MAX_BATCH`.
const MAX_SELECTION: usize = 200;

/// Analyses the selected items with as few AI requests as possible.
#[tauri::command]
pub async fn analyze_files(
    state: State<'_, AppState>,
    file_ids: Vec<i64>,
    force: bool,
) -> CommandResult<Vec<AnalysisDto>> {
    if file_ids.len() > MAX_SELECTION {
        return Err(AppError::invalid_input(format!(
            "select at most {MAX_SELECTION} items per analysis"
        )));
    }
    let ids: Vec<FileId> = file_ids.into_iter().map(FileId).collect();
    let service = state.analysis.clone();
    tauri::async_runtime::spawn_blocking(move || service.analyze_many(&ids, force))
        .await
        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?
        .map(|outcomes| outcomes.into_iter().map(AnalysisDto::from).collect())
        .map_err(Into::into)
}

/// Stored analyses that still match the items' current state (risk badges in the table).
#[tauri::command]
pub fn get_analyses(
    state: State<'_, AppState>,
    file_ids: Vec<i64>,
) -> CommandResult<Vec<AnalysisDto>> {
    let ids: Vec<FileId> = file_ids.into_iter().map(FileId).collect();
    Ok(state
        .analysis
        .latest_for(&ids)?
        .into_iter()
        .map(|(id, a)| AnalysisDto::new(id.0, a, None))
        .collect())
}

#[tauri::command]
pub fn get_ai_settings(state: State<'_, AppState>) -> CommandResult<AiSettingsDto> {
    let settings = state.ai_settings.load()?;
    let providers = AiProviderKind::ALL
        .into_iter()
        .map(|kind| {
            Ok(AiProviderInfoDto {
                id: kind.into(),
                label: kind.label().into(),
                default_model: kind.default_model().into(),
                default_base_url: kind.default_base_url().into(),
                needs_api_key: kind.needs_api_key(),
                has_api_key: kind.needs_api_key() && state.ai_settings.has_api_key(kind)?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(AiSettingsDto {
        provider: settings.provider.map(Into::into),
        model: settings.model,
        base_url: settings.base_url,
        providers,
    })
}

#[tauri::command]
pub fn save_ai_settings(
    state: State<'_, AppState>,
    input: SaveAiSettingsInput,
) -> CommandResult<AiSettingsDto> {
    let key = match (&input.api_key, input.clear_api_key) {
        (_, true) => ApiKeyChange::Clear,
        (Some(k), false) if !k.trim().is_empty() => ApiKeyChange::Set(k.clone()),
        _ => ApiKeyChange::Keep,
    };
    state.ai_settings.save(&input.settings(), key)?;
    get_ai_settings(state)
}

#[tauri::command]
pub async fn test_ai_connection(state: State<'_, AppState>) -> CommandResult<ConnectionTestDto> {
    let service = state.ai_settings.clone();
    let (provider, model) = tauri::async_runtime::spawn_blocking(move || service.test_connection())
        .await
        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))??;
    Ok(ConnectionTestDto { provider, model })
}

/// Models available to the given (or stored) credentials, so the UI offers only valid ids.
/// `api_key` lets the user try a key before saving it; it is never stored by this command.
#[tauri::command]
pub async fn list_ai_models(
    state: State<'_, AppState>,
    provider: AiProviderIdDto,
    base_url: Option<String>,
    api_key: Option<String>,
) -> CommandResult<Vec<AiModelDto>> {
    let service = state.ai_settings.clone();
    let base_url = base_url
        .map(|u| u.trim().to_owned())
        .filter(|u| !u.is_empty());
    tauri::async_runtime::spawn_blocking(move || {
        service.list_models(provider.into(), base_url, api_key)
    })
    .await
    .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?
    .map(|models| models.into_iter().map(Into::into).collect())
    .map_err(Into::into)
}
