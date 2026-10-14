-- V001: initial catalog schema (see specs/001-disk-space-cleaner/data-model.md).
-- Never edit a migration after it is merged; add a new one instead.

CREATE TABLE scans (
    id                 INTEGER PRIMARY KEY,
    roots              TEXT    NOT NULL,             -- JSON array of absolute paths
    min_file_size      INTEGER NOT NULL,
    status             TEXT    NOT NULL CHECK (status IN ('Running','Completed','Cancelled','Failed')),
    started_at         INTEGER NOT NULL,
    finished_at        INTEGER,
    files_seen         INTEGER NOT NULL DEFAULT 0,
    bytes_seen         INTEGER NOT NULL DEFAULT 0,
    inaccessible_count INTEGER NOT NULL DEFAULT 0,
    error              TEXT
);

CREATE TABLE file_entries (
    id              INTEGER PRIMARY KEY,
    scan_id         INTEGER NOT NULL REFERENCES scans(id) ON DELETE CASCADE,
    path            TEXT    NOT NULL,
    parent_path     TEXT,
    name            TEXT    NOT NULL,
    extension       TEXT,
    kind            TEXT    NOT NULL CHECK (kind IN ('File','Directory','Package')),
    size_on_disk    INTEGER NOT NULL,
    apparent_size   INTEGER NOT NULL,
    file_count      INTEGER NOT NULL,
    created_at      INTEGER,
    modified_at     INTEGER,
    accessed_at     INTEGER,
    inode           INTEGER NOT NULL,
    device          INTEGER NOT NULL,
    is_hardlink_dup INTEGER NOT NULL DEFAULT 0,
    is_protected    INTEGER NOT NULL DEFAULT 0,
    UNIQUE (scan_id, path)
);
CREATE INDEX idx_file_entries_size   ON file_entries (scan_id, size_on_disk DESC);
CREATE INDEX idx_file_entries_parent ON file_entries (scan_id, parent_path);
CREATE INDEX idx_file_entries_ext    ON file_entries (scan_id, extension);

CREATE TABLE risk_assessments (
    id                INTEGER PRIMARY KEY,
    path              TEXT    NOT NULL,
    size_on_disk      INTEGER NOT NULL,
    modified_at       INTEGER,
    inode             INTEGER NOT NULL,
    level             TEXT    NOT NULL CHECK (level IN ('Safe','Unknown','Caution','Dangerous')),
    category          TEXT    NOT NULL,
    explanation       TEXT    NOT NULL,
    source            TEXT    NOT NULL,              -- JSON AssessmentSource
    evaluator_version TEXT    NOT NULL,
    created_at        INTEGER NOT NULL
);
-- modified_at is nullable; COALESCE keeps NULLs from bypassing uniqueness.
CREATE UNIQUE INDEX idx_risk_key
    ON risk_assessments (path, size_on_disk, COALESCE(modified_at, -1), inode, evaluator_version);
CREATE INDEX idx_risk_path ON risk_assessments (path);

CREATE TABLE action_records (
    id           INTEGER PRIMARY KEY,
    batch_id     TEXT    NOT NULL,
    action       TEXT    NOT NULL,
    path         TEXT    NOT NULL,
    size_on_disk INTEGER NOT NULL,
    risk_level   TEXT    NOT NULL,
    status       TEXT    NOT NULL CHECK (status IN ('Succeeded','Skipped','Failed')),
    reason       TEXT,
    executed_at  INTEGER NOT NULL
);
CREATE INDEX idx_action_records_time ON action_records (executed_at DESC);

-- The audit log is append-only (Constitution I).
CREATE TRIGGER action_records_no_update BEFORE UPDATE ON action_records
BEGIN SELECT RAISE(ABORT, 'action_records is append-only'); END;
CREATE TRIGGER action_records_no_delete BEFORE DELETE ON action_records
BEGIN SELECT RAISE(ABORT, 'action_records is append-only'); END;

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL                              -- JSON
);
