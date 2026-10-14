//! `Verdict`: the provider-independent result of analysing one item.

use std::path::Path;

use crate::domain::file_entry::FileRecord;
use crate::domain::locale::Locale;
use crate::domain::risk::{
    AssessmentSource, Recommendation, RiskAssessment, RiskCategory, RiskLevel, SuggestedCommand,
};

const MAX_SUMMARY: usize = 300;
const MAX_EXPLANATION: usize = 900;
const MAX_STEPS: usize = 8;
const MAX_STEP: usize = 300;
const MAX_COMMANDS: usize = 6;
const MAX_COMMAND: usize = 400;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub level: RiskLevel,
    pub category: RiskCategory,
    pub summary: String,
    pub explanation: String,
    pub recommendation: Recommendation,
    pub steps: Vec<String>,
    pub commands: Vec<SuggestedCommand>,
}

impl Verdict {
    /// Trims and bounds every field, so neither a rule nor a model can flood the UI.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            summary: clip(&self.summary, MAX_SUMMARY),
            explanation: clip(&self.explanation, MAX_EXPLANATION),
            steps: self
                .steps
                .iter()
                .map(|s| clip(s, MAX_STEP))
                .filter(|s| !s.is_empty())
                .take(MAX_STEPS)
                .collect(),
            commands: self
                .commands
                .into_iter()
                .filter_map(|c| {
                    // Commands are shown as single copyable lines.
                    let command = clip(&c.command.replace(['\n', '\r'], " "), MAX_COMMAND);
                    (!command.is_empty()).then(|| SuggestedCommand {
                        command,
                        description: clip(&c.description, MAX_STEP),
                    })
                })
                .take(MAX_COMMANDS)
                .collect(),
            ..self
        }
    }

    pub fn into_assessment(
        self,
        record: &FileRecord,
        source: AssessmentSource,
        evaluator_version: String,
        created_at: i64,
    ) -> RiskAssessment {
        let v = self.sanitized();
        RiskAssessment {
            path: record.path.clone(),
            fingerprint: record.fingerprint(),
            level: v.level,
            category: v.category,
            summary: v.summary,
            explanation: v.explanation,
            recommendation: v.recommendation,
            steps: v.steps,
            commands: v.commands,
            source,
            evaluator_version,
            created_at,
        }
    }
}

/// Verdict for items the app must never touch, regardless of any classifier (FR-016).
pub fn protected_verdict(path: &Path, locale: Locale) -> Verdict {
    Verdict {
        level: RiskLevel::Dangerous,
        category: RiskCategory::System,
        summary: match locale {
            Locale::EnUs => format!("{} is a protected location.", path.display()),
            Locale::PtBr => format!("{} é um local protegido.", path.display()),
        },
        explanation: locale
            .pick(
                "This item is part of the system, an essential user folder, or belongs to macOS Cleaner itself. The app never removes it.",
                "Este item faz parte do sistema, é uma pasta essencial do usuário ou pertence ao próprio macOS Cleaner. O app nunca o remove.",
            )
            .to_owned(),
        recommendation: Recommendation::Keep,
        steps: vec![],
        commands: vec![],
    }
}

fn clip(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_owned();
    }
    let mut out: String = trimmed.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verdict() -> Verdict {
        Verdict {
            level: RiskLevel::Safe,
            category: RiskCategory::Cache,
            summary: "  cache  ".into(),
            explanation: "x".repeat(2_000),
            recommendation: Recommendation::DeleteSafely,
            steps: vec![" a ".into(), "  ".into()],
            commands: vec![
                SuggestedCommand {
                    command: "echo a\necho b".into(),
                    description: "d".into(),
                },
                SuggestedCommand {
                    command: "   ".into(),
                    description: "empty".into(),
                },
            ],
        }
    }

    #[test]
    fn sanitizing_trims_bounds_and_drops_empty_entries() {
        let v = verdict().sanitized();
        assert_eq!(v.summary, "cache");
        assert_eq!(v.explanation.chars().count(), MAX_EXPLANATION);
        assert!(v.explanation.ends_with('…'));
        assert_eq!(v.steps, ["a"]);
        assert_eq!(v.commands.len(), 1);
        assert_eq!(v.commands[0].command, "echo a echo b");
    }

    #[test]
    fn caps_number_of_steps_and_commands() {
        let mut v = verdict();
        v.steps = (0..20).map(|i| format!("step {i}")).collect();
        v.commands = (0..20)
            .map(|i| SuggestedCommand {
                command: format!("cmd {i}"),
                description: String::new(),
            })
            .collect();
        let v = v.sanitized();
        assert_eq!(v.steps.len(), MAX_STEPS);
        assert_eq!(v.commands.len(), MAX_COMMANDS);
    }

    #[test]
    fn protected_verdict_is_always_dangerous_and_keep() {
        let v = protected_verdict(Path::new("/System"), Locale::EnUs);
        assert_eq!(v.level, RiskLevel::Dangerous);
        assert_eq!(v.recommendation, Recommendation::Keep);
    }
}
