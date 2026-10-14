//! UI / analysis language.

use serde::{Deserialize, Serialize};

/// Languages the app ships with. English is the fallback for any other system language.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Locale {
    #[default]
    #[serde(rename = "en-US")]
    EnUs,
    #[serde(rename = "pt-BR")]
    PtBr,
}

impl Locale {
    pub const ALL: [Self; 2] = [Self::EnUs, Self::PtBr];

    /// BCP 47 tag, as used by the frontend.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::PtBr => "pt-BR",
        }
    }

    pub fn parse(tag: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|l| l.tag().eq_ignore_ascii_case(tag))
    }

    /// Language name used in instructions to AI models.
    pub const fn language_name(self) -> &'static str {
        match self {
            Self::EnUs => "English",
            Self::PtBr => "Brazilian Portuguese",
        }
    }

    /// Picks the English or Portuguese variant of a text.
    pub fn pick<'a>(self, en: &'a str, pt: &'a str) -> &'a str {
        match self {
            Self::EnUs => en,
            Self::PtBr => pt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_round_trip_and_default_is_english() {
        for l in Locale::ALL {
            assert_eq!(Locale::parse(l.tag()), Some(l));
        }
        assert_eq!(Locale::parse("PT-br"), Some(Locale::PtBr));
        assert_eq!(Locale::parse("fr-FR"), None);
        assert_eq!(Locale::default(), Locale::EnUs);
        assert_eq!(serde_json::to_string(&Locale::PtBr).unwrap(), "\"pt-BR\"");
    }

    #[test]
    fn picks_the_matching_text() {
        assert_eq!(Locale::EnUs.pick("Folder", "Pasta"), "Folder");
        assert_eq!(Locale::PtBr.pick("Folder", "Pasta"), "Pasta");
    }
}
