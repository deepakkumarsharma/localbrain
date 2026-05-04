pub const CREATE_FILES_TABLE: &str = "
CREATE TABLE IF NOT EXISTS files (
  path TEXT PRIMARY KEY,
  language TEXT,
  size_bytes INTEGER NOT NULL,
  modified_at TEXT,
  content_hash TEXT NOT NULL,
  last_indexed_at TEXT,
  status TEXT NOT NULL
)";

pub const CREATE_INDEX_RUNS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS index_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  files_seen INTEGER NOT NULL DEFAULT 0,
  files_changed INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL
)";
