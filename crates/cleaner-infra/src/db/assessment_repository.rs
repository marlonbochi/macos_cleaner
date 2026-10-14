use std::path::PathBuf;
use std::sync::Arc;

use cleaner_core::domain::file_entry::FileFingerprint;
use cleaner_core::domain::risk::{
    AssessmentKey, AssessmentSource, Recommendation, RiskAssessment, RiskCategory, RiskLevel,
    SuggestedCommand,
};
use cleaner_core::error::RepoError;
use cleaner_core::ports::repository::AssessmentRepository;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::convert::{
    invalid_column, path_from_sql, path_to_sql, size_from_sql, size_to_sql, u64_from_sql,
    u64_to_sql,
};
use super::{Database, storage};

const COLUMNS: &str = "path, size_on_disk, modified_at, inode, level, category, explanation, \
                       source, evaluator_version, created_at, details";

/// Fields stored in the `details` JSON column (added in V002).
#[derive(Debug, Default, Serialize, Deserialize)]
struct Details {
    summary: String,
    recommendation: String,
    steps: Vec<String>,
    commands: Vec<SuggestedCommand>,
}

#[derive(Debug, Clone)]
pub struct SqliteAssessmentRepository {
    db: Arc<Database>,
}

impl SqliteAssessmentRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

impl AssessmentRepository for SqliteAssessmentRepository {
    fn find(&self, key: &AssessmentKey) -> Result<Option<RiskAssessment>, RepoError> {
        let fp = key.fingerprint;
        self.db
            .reader()?
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM risk_assessments
                     WHERE path = ?1 AND size_on_disk = ?2 AND modified_at IS ?3 AND inode = ?4
                       AND evaluator_version = ?5
                     ORDER BY created_at DESC, id DESC LIMIT 1"
                ),
                params![
                    path_to_sql(&key.path)?,
                    size_to_sql(fp.size_on_disk),
                    fp.modified_at,
                    u64_to_sql(fp.inode),
                    key.evaluator_version,
                ],
                map_row,
            )
            .optional()
            .map_err(storage)
    }

    fn save(&self, a: &RiskAssessment) -> Result<(), RepoError> {
        let source = to_json(&a.source)?;
        let details = to_json(&Details {
            summary: a.summary.clone(),
            recommendation: a.recommendation.as_str().to_owned(),
            steps: a.steps.clone(),
            commands: a.commands.clone(),
        })?;
        // The unique index on (path, fingerprint, evaluator) makes this an upsert.
        self.db
            .writer()?
            .execute(
                "INSERT OR REPLACE INTO risk_assessments (
                    path, size_on_disk, modified_at, inode, level, category, explanation,
                    source, evaluator_version, created_at, details)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    path_to_sql(&a.path)?,
                    size_to_sql(a.fingerprint.size_on_disk),
                    a.fingerprint.modified_at,
                    u64_to_sql(a.fingerprint.inode),
                    a.level.as_str(),
                    a.category.as_str(),
                    a.explanation,
                    source,
                    a.evaluator_version,
                    a.created_at,
                    details,
                ],
            )
            .map(|_| ())
            .map_err(storage)
    }

    fn latest_matching(
        &self,
        items: &[(PathBuf, FileFingerprint)],
    ) -> Result<Vec<RiskAssessment>, RepoError> {
        let conn = self.db.reader()?;
        let mut stmt = conn
            .prepare_cached(&format!(
                "SELECT {COLUMNS} FROM risk_assessments
                 WHERE path = ?1 AND size_on_disk = ?2 AND modified_at IS ?3 AND inode = ?4
                 ORDER BY created_at DESC, id DESC LIMIT 1"
            ))
            .map_err(storage)?;
        let mut out = Vec::new();
        for (path, fp) in items {
            let found = stmt
                .query_row(
                    params![
                        path_to_sql(path)?,
                        size_to_sql(fp.size_on_disk),
                        fp.modified_at,
                        u64_to_sql(fp.inode)
                    ],
                    map_row,
                )
                .optional()
                .map_err(storage)?;
            out.extend(found);
        }
        Ok(out)
    }
}

fn map_row(row: &Row<'_>) -> rusqlite::Result<RiskAssessment> {
    let level: String = row.get(4)?;
    let category: String = row.get(5)?;
    let source: String = row.get(7)?;
    let details: String = row.get(10)?;
    let details: Details = serde_json::from_str(&details).unwrap_or_default();
    Ok(RiskAssessment {
        path: path_from_sql(row.get(0)?),
        fingerprint: FileFingerprint {
            size_on_disk: size_from_sql(row.get(1)?),
            modified_at: row.get(2)?,
            inode: u64_from_sql(row.get(3)?),
        },
        level: RiskLevel::parse(&level).ok_or_else(|| invalid_column("level", &level))?,
        category: RiskCategory::parse(&category).unwrap_or(RiskCategory::Other),
        summary: details.summary,
        explanation: row.get(6)?,
        recommendation: Recommendation::parse(&details.recommendation)
            .unwrap_or(Recommendation::ReviewManually),
        steps: details.steps,
        commands: details.commands,
        source: serde_json::from_str::<AssessmentSource>(&source)
            .map_err(|_| invalid_column("source", &source))?,
        evaluator_version: row.get(8)?,
        created_at: row.get(9)?,
    })
}

fn to_json<T: Serialize>(value: &T) -> Result<String, RepoError> {
    serde_json::to_string(value).map_err(|e| RepoError::Storage(e.to_string()))
}
