//! User preferences (language).

use cleaner_core::domain::locale::Locale;
use tauri::State;

use crate::dto::PreferencesDto;
use crate::error::{AppError, CommandResult};
use crate::state::AppState;

#[tauri::command]
pub fn get_preferences(state: State<'_, AppState>) -> CommandResult<PreferencesDto> {
    let locale = state.preferences.saved_locale()?;
    Ok(PreferencesDto {
        locale: locale.map(|l| l.tag().to_owned()),
    })
}

/// Saves the UI language; analyses produced afterwards use it too.
#[tauri::command]
pub fn set_locale(state: State<'_, AppState>, locale: String) -> CommandResult<()> {
    let locale = Locale::parse(&locale)
        .ok_or_else(|| AppError::invalid_input(format!("unsupported language: {locale}")))?;
    Ok(state.preferences.set_locale(locale)?)
}
