use std::sync::Arc;

use cleaner_core::domain::file_entry::{FileEntry, FileId, FileKind, FileRecord};
use cleaner_core::domain::scan::ScanId;
use cleaner_core::error::RepoError;
use cleaner_core::ports::repository::{FileEntryRepository, FileQuery, FileSort, Page};
use rusqlite::types::Value;
use rusqlite::{Row, params, params_from_iter};

use super::convert::{
    escape_like, invalid_column, path_from_sql, path_to_sql, size_from_sql, size_to_sql,
    u64_from_sql, u64_to_sql,
};
use super::{Database, storage};

/// SQLite's default limit on bound parameters is far above this; chunking keeps statements small.
const ID_CHUNK: usize = 500;

const COLUMNS: &str = "id, scan_id, path, kind, size_on_disk, apparent_size, file_count, \
                       created_at, modified_at, accessed_at, inode, device, is_hardlink_dup, \
                       is_protected";

#[derive(Debug, Clone)]
pub struct SqliteFileEntryRepository {
    db: Arc<Database>,
}

impl SqliteFileEntryRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

impl FileEntryRepository for SqliteFileEntryRepository {
    fn insert_batch(&self, scan: ScanId, records: &[FileRecord]) -> Result<(), RepoError> {
        let mut conn = self.db.writer()?;
        let tx = conn.transaction().map_err(storage)?;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR IGNORE INTO file_entries (
                        scan_id, path, parent_path, name, extension, kind, size_on_disk,
                        apparent_size, file_count, created_at, modified_at, accessed_at, inode,
                        device, is_hardlink_dup, is_protected)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                )
                .map_err(storage)?;
            for r in records {
                let parent = r.parent().map(path_to_sql).transpose()?;
                stmt.execute(params![
                    scan.0,
                    path_to_sql(&r.path)?,
                    parent,
                    r.name(),
                    r.extension(),
                    r.kind.as_str(),
                    size_to_sql(r.size_on_disk),
                    size_to_sql(r.apparent_size),
                    u64_to_sql(r.file_count),
                    r.created_at,
                    r.modified_at,
                    r.accessed_at,
                    u64_to_sql(r.inode),
                    u64_to_sql(r.device),
                    r.is_hardlink_dup,
                    r.is_protected,
                ])
                .map_err(storage)?;
            }
        }
        tx.commit().map_err(storage)
    }

    fn page(&self, scan: ScanId, query: &FileQuery) -> Result<Page<FileEntry>, RepoError> {
        let (where_sql, mut values) = build_where(scan, query)?;
        let order = match query.sort {
            FileSort::SizeDesc => "size_on_disk DESC, id",
            FileSort::SizeAsc => "size_on_disk ASC, id",
            FileSort::ModifiedAsc => "modified_at ASC, id",
        };
        let conn = self.db.reader()?;
        let total: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM file_entries WHERE {where_sql}"),
                params_from_iter(values.iter()),
                |row| row.get(0),
            )
            .map_err(storage)?;
        values.push(Value::Integer(i64::from(query.limit)));
        values.push(Value::Integer(u64_to_sql(query.offset)));
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM file_entries WHERE {where_sql} ORDER BY {order} LIMIT ? OFFSET ?"
            ))
            .map_err(storage)?;
        let items = stmt
            .query_map(params_from_iter(values.iter()), map_entry)
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        Ok(Page {
            items,
            total: u64_from_sql(total),
            offset: query.offset,
            limit: query.limit,
        })
    }

    fn get_many(&self, ids: &[FileId]) -> Result<Vec<FileEntry>, RepoError> {
        let conn = self.db.reader()?;
        let mut out = Vec::with_capacity(ids.len());
        for chunk in ids.chunks(ID_CHUNK) {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT {COLUMNS} FROM file_entries WHERE id IN ({})",
                    placeholders(chunk.len())
                ))
                .map_err(storage)?;
            let rows = stmt
                .query_map(params_from_iter(chunk.iter().map(|id| id.0)), map_entry)
                .map_err(storage)?;
            for row in rows {
                out.push(row.map_err(storage)?);
            }
        }
        Ok(out)
    }

    fn remove(&self, ids: &[FileId]) -> Result<(), RepoError> {
        let mut conn = self.db.writer()?;
        let tx = conn.transaction().map_err(storage)?;
        for chunk in ids.chunks(ID_CHUNK) {
            tx.execute(
                &format!(
                    "DELETE FROM file_entries WHERE id IN ({})",
                    placeholders(chunk.len())
                ),
                params_from_iter(chunk.iter().map(|id| id.0)),
            )
            .map_err(storage)?;
        }
        tx.commit().map_err(storage)
    }
}

fn build_where(scan: ScanId, query: &FileQuery) -> Result<(String, Vec<Value>), RepoError> {
    let f = &query.filter;
    let mut clauses = vec!["scan_id = ?".to_owned()];
    let mut values = vec![Value::Integer(scan.0)];
    if let Some(min) = f.min_size {
        clauses.push("size_on_disk >= ?".into());
        values.push(Value::Integer(size_to_sql(min)));
    }
    if !f.extensions.is_empty() {
        clauses.push(format!(
            "extension IN ({})",
            placeholders(f.extensions.len())
        ));
        values.extend(f.extensions.iter().map(|e| Value::Text(e.to_lowercase())));
    }
    if let Some(parent) = &f.parent {
        clauses.push("parent_path = ?".into());
        values.push(Value::Text(path_to_sql(parent)?.to_owned()));
    }
    if let Some(prefix) = &f.path_prefix {
        let prefix = path_to_sql(prefix)?.trim_end_matches('/');
        clauses.push("path LIKE ? ESCAPE '\\'".into());
        values.push(Value::Text(format!("{}/%", escape_like(prefix))));
    }
    if !f.kinds.is_empty() {
        clauses.push(format!("kind IN ({})", placeholders(f.kinds.len())));
        values.extend(f.kinds.iter().map(|k| Value::Text(k.as_str().to_owned())));
    }
    Ok((clauses.join(" AND "), values))
}

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

fn map_entry(row: &Row<'_>) -> rusqlite::Result<FileEntry> {
    let kind: String = row.get(3)?;
    Ok(FileEntry {
        id: FileId(row.get(0)?),
        scan_id: ScanId(row.get(1)?),
        record: FileRecord {
            path: path_from_sql(row.get(2)?),
            kind: FileKind::parse(&kind).ok_or_else(|| invalid_column("kind", &kind))?,
            size_on_disk: size_from_sql(row.get(4)?),
            apparent_size: size_from_sql(row.get(5)?),
            file_count: u64_from_sql(row.get(6)?),
            created_at: row.get(7)?,
            modified_at: row.get(8)?,
            accessed_at: row.get(9)?,
            inode: u64_from_sql(row.get(10)?),
            device: u64_from_sql(row.get(11)?),
            is_hardlink_dup: row.get(12)?,
            is_protected: row.get(13)?,
        },
    })
}
