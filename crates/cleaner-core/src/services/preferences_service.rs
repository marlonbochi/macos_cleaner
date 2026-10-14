//! Use case: user preferences that are not AI-specific (currently the language).

use std::sync::Arc;

use serde_json::json;

use crate::domain::locale::Locale;
use crate::error::CoreError;
use crate::ports::repository::SettingsRepository;

const LOCALE_KEY: &str = "ui.locale";

#[derive(Clone)]
pub struct PreferencesService {
    settings: Arc<dyn SettingsRepository>,
}

impl std::fmt::Debug for PreferencesService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreferencesService").finish_non_exhaustive()
    }
}

impl PreferencesService {
    pub fn new(settings: Arc<dyn SettingsRepository>) -> Self {
        Self { settings }
    }

    /// The language chosen by the user, or `None` when they never chose one (the UI then follows
    /// the system language).
    pub fn saved_locale(&self) -> Result<Option<Locale>, CoreError> {
        Ok(self
            .settings
            .get(LOCALE_KEY)?
            .and_then(|v| v.as_str().and_then(Locale::parse)))
    }

    /// Language used for analyses: the saved one, else English.
    pub fn locale(&self) -> Locale {
        self.saved_locale().ok().flatten().unwrap_or_default()
    }

    pub fn set_locale(&self, locale: Locale) -> Result<(), CoreError> {
        self.settings.set(LOCALE_KEY, &json!(locale.tag()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::InMemorySettingsRepository;

    #[test]
    fn locale_is_unset_until_chosen_and_then_persisted() {
        let service = PreferencesService::new(Arc::new(InMemorySettingsRepository::default()));
        assert_eq!(service.saved_locale().unwrap(), None);
        assert_eq!(service.locale(), Locale::EnUs);

        service.set_locale(Locale::PtBr).unwrap();

        assert_eq!(service.saved_locale().unwrap(), Some(Locale::PtBr));
        assert_eq!(service.locale(), Locale::PtBr);
    }

    #[test]
    fn unknown_stored_values_are_ignored() {
        let repo = Arc::new(InMemorySettingsRepository::default());
        repo.set(LOCALE_KEY, &json!("fr-FR")).unwrap();
        assert_eq!(PreferencesService::new(repo).saved_locale().unwrap(), None);
    }
}
