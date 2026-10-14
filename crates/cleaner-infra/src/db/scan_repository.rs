use std::path::PathBuf;
use std::sync::Arc;

use cleaner_core::domain::scan::{NewScan, Scan, ScanId, ScanStats, ScanStatus};
use cleaner_core::error::RepoError;
use cleaner_core::ports::repository::ScanRepository;
use rusqlite::{OptionalExtension, Row, params};

use super::convert::{invalid_column, size_from_sql, size_to_sql, u64_from_sql, u64_to_sql};
use super::{Database, storage};

const COLUMNS: &str = "id, roots, min_file_size, status, started_at, finished_at, \
                       files_seen, bytes_seen, inaccessible_count, error";

#[derive(Debug, Clone)]
pub struct SqliteScanRepository {
    db: Arc<Database>,
}

impl SqliteScanRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

impl ScanRepository for SqliteScanRepository {
    fn create(&self, new: &NewScan) -> Result<Scan, RepoError> {
        let roots =
            serde_json::to_string(&new.roots).map_err(|e| RepoError::Storage(e.to_string()))?;
        let conn = self.db.writer()?;
        conn.execute(
            "INSERT INTO scans (roots, min_file_size, status, started_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                roots,
                size_to_sql(new.min_file_size),
                ScanStatus::Running.as_str(),
                new.started_at
            ],
        )
        .map_err(storage)?;
        Ok(Scan::start(ScanId(conn.last_insert_rowid()), new.clone()))
    }

    fn save(&self, scan: &Scan) -> Result<(), RepoError> {
        let changed = self
            .db
            .writer()?
            .execute(
                "UPDATE scans SET status = ?2, finished_at = ?3, files_seen = ?4, bytes_seen = ?5,
                        inaccessible_count = ?6, error = ?7
                 WHERE id = ?1",
                params![
                    scan.id.0,
                    scan.status.as_str(),
                    scan.finished_at,
                    u64_to_sql(scan.stats.files_seen),
                    size_to_sql(scan.stats.bytes_seen),
                    u64_to_sql(scan.stats.inaccessible_count),
                    scan.error,
                ],
            )
            .map_err(storage)?;
        if changed == 0 {
            return Err(RepoError::NotFound(format!("scan {}", scan.id.0)));
        }
        Ok(())
    }

    fn get(&self, id: ScanId) -> Result<Option<Scan>, RepoError> {
        self.db
            .reader()?
            .query_row(
                &format!("SELECT {COLUMNS} FROM scans WHERE id = ?1"),
                [id.0],
                map_scan,
            )
            .optional()
            .map_err(storage)
    }

    fn find_running(&self) -> Result<Vec<Scan>, RepoError> {
        let conn = self.db.reader()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM scans WHERE status = 'Running' ORDER BY id"
            ))
            .map_err(storage)?;
        stmt.query_map([], map_scan)
            .map_err(storage)?
            .collect::<Result<_, _>>()
            .map_err(storage)
    }

    fn latest_finished(&self) -> Result<Option<Scan>, RepoError> {
        self.db
            .reader()?
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM scans WHERE status IN ('Completed', 'Cancelled')
                     ORDER BY started_at DESC, id DESC LIMIT 1"
                ),
                [],
                map_scan,
            )
            .optional()
            .map_err(storage)
    }

    fn delete_all_except(&self, keep: ScanId) -> Result<(), RepoError> {
        // ON DELETE CASCADE removes the file entries of deleted scans.
        self.db
            .writer()?
            .execute("DELETE FROM scans WHERE id <> ?1", [keep.0])
            .map(|_| ())
            .map_err(storage)
    }
}

fn map_scan(row: &Row<'_>) -> rusqlite::Result<Scan> {
    let roots: String = row.get(1)?;
    let roots: Vec<PathBuf> =
        serde_json::from_str(&roots).map_err(|_| invalid_column("roots", &roots))?;
    let status: String = row.get(3)?;
    Ok(Scan {
        id: ScanId(row.get(0)?),
        roots,
        min_file_size: size_from_sql(row.get(2)?),
        status: ScanStatus::parse(&status).ok_or_else(|| invalid_column("status", &status))?,
        started_at: row.get(4)?,
        finished_at: row.get(5)?,
        stats: ScanStats {
            files_seen: u64_from_sql(row.get(6)?),
            bytes_seen: size_from_sql(row.get(7)?),
            inaccessible_count: u64_from_sql(row.get(8)?),
        },
        error: row.get(9)?,
    })
}
