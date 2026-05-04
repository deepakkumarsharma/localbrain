use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, SqlitePool};
use thiserror::Error;

use super::schema::{
    CREATE_EMBEDDINGS_TABLE, CREATE_FILES_TABLE, CREATE_INDEX_RUNS_TABLE,
    CREATE_SEARCH_DOCUMENTS_TABLE,
};
use super::types::{FileChangeStatus, FileMetadata, IndexRunSummary};

#[derive(Clone)]
pub struct MetadataStore {
    pool: SqlitePool,
}

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("failed to create metadata database directory: {0}")]
    CreateDirectory(#[from] std::io::Error),
    #[error("sqlite operation failed: {0}")]
    Sqlite(#[from] sqlx::Error),
    #[error("metadata row has invalid status: {0}")]
    InvalidStatus(String),
    #[error("failed to resolve metadata path: {0}")]
    InvalidPath(String),
    #[error("system clock is before unix epoch")]
    SystemClock,
}

impl MetadataStore {
    pub async fn open(root_dir: impl AsRef<Path>) -> Result<Self, MetadataError> {
        let root_dir = root_dir.as_ref().to_path_buf();
        fs::create_dir_all(&root_dir)?;
        let db_path = root_dir.join("metadata.db");
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        let store = Self { pool };
        store.init_schema().await?;

        Ok(store)
    }

    pub async fn open_default<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
    ) -> Result<Self, MetadataError> {
        use tauri::Manager;

        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|_| MetadataError::InvalidPath("app_data_dir".to_string()))?;

        Self::open(app_data_dir.join(".localbrain").join("metadata")).await
    }

    pub async fn init_schema(&self) -> Result<(), MetadataError> {
        sqlx::query(CREATE_FILES_TABLE).execute(&self.pool).await?;
        sqlx::query(CREATE_INDEX_RUNS_TABLE)
            .execute(&self.pool)
            .await?;
        sqlx::query(CREATE_SEARCH_DOCUMENTS_TABLE)
            .execute(&self.pool)
            .await?;
        sqlx::query(CREATE_EMBEDDINGS_TABLE)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub fn resolve_path(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        if path.is_absolute() {
            return path.to_path_buf();
        }

        let root = project_root();
        let project_candidate = root.join(path);
        if project_candidate.exists() {
            return project_candidate;
        }

        path.to_path_buf()
    }

    pub fn normalize_path(&self, path: impl AsRef<Path>) -> String {
        normalize_display_path(path.as_ref())
    }

    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn scan_file(&self, path: impl AsRef<Path>) -> Result<FileMetadata, MetadataError> {
        let requested_path = path.as_ref();
        let source_path = self.resolve_path(requested_path);
        let bytes = fs::read(&source_path)?;
        let metadata = fs::metadata(&source_path)?;
        let modified_at = metadata.modified().ok().map(timestamp_from_system_time);
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let content_hash = format!("{:x}", hasher.finalize());

        Ok(FileMetadata {
            path: normalize_display_path(requested_path),
            language: language_from_path(requested_path).map(str::to_string),
            size_bytes: i64::try_from(bytes.len()).unwrap_or(i64::MAX),
            modified_at,
            content_hash,
            last_indexed_at: None,
            status: FileChangeStatus::New,
        })
    }

    pub async fn upsert_file(&self, metadata: &FileMetadata) -> Result<(), MetadataError> {
        sqlx::query(
            "
            INSERT INTO files (
              path,
              language,
              size_bytes,
              modified_at,
              content_hash,
              last_indexed_at,
              status
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(path) DO UPDATE SET
              language = excluded.language,
              size_bytes = excluded.size_bytes,
              modified_at = excluded.modified_at,
              content_hash = excluded.content_hash,
              last_indexed_at = excluded.last_indexed_at,
              status = excluded.status
            ",
        )
        .bind(&metadata.path)
        .bind(&metadata.language)
        .bind(metadata.size_bytes)
        .bind(&metadata.modified_at)
        .bind(&metadata.content_hash)
        .bind(&metadata.last_indexed_at)
        .bind(metadata.status.as_str())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_file(&self, path: &str) -> Result<Option<FileMetadata>, MetadataError> {
        let path = self.normalize_path(path);
        let row = sqlx::query(
            "
            SELECT path, language, size_bytes, modified_at, content_hash, last_indexed_at, status
            FROM files
            WHERE path = ?
            ",
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_file_metadata).transpose()
    }

    pub async fn classify_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<FileChangeStatus, MetadataError> {
        let requested_path = path.as_ref();
        let normalized_path = normalize_display_path(requested_path);

        if !self.resolve_path(requested_path).exists() {
            return match self.get_file(&normalized_path).await? {
                Some(_) => Ok(FileChangeStatus::Deleted),
                None => Ok(FileChangeStatus::Error),
            };
        }

        let scanned = self.scan_file(requested_path).await?;

        match self.get_file(&scanned.path).await? {
            Some(existing) if existing.content_hash == scanned.content_hash => {
                Ok(FileChangeStatus::Unchanged)
            }
            Some(_) => Ok(FileChangeStatus::Changed),
            None => Ok(FileChangeStatus::New),
        }
    }

    pub async fn record_file_metadata(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<FileMetadata, MetadataError> {
        let path_ref = path.as_ref();
        let status = self.classify_file(path_ref).await?;
        let normalized_path = normalize_display_path(path_ref);

        if status == FileChangeStatus::Deleted {
            let existing = self.get_file(&normalized_path).await?;
            let metadata = FileMetadata {
                path: normalized_path,
                language: existing.as_ref().and_then(|e| e.language.clone()),
                size_bytes: existing.as_ref().map(|e| e.size_bytes).unwrap_or(0),
                modified_at: existing.as_ref().and_then(|e| e.modified_at.clone()),
                content_hash: existing
                    .as_ref()
                    .map(|e| e.content_hash.clone())
                    .unwrap_or_default(),
                last_indexed_at: existing.as_ref().and_then(|e| e.last_indexed_at.clone()),
                status,
            };
            self.upsert_file(&metadata).await?;
            return Ok(metadata);
        }

        let mut metadata = self.scan_file(path_ref).await?;
        if let Some(existing) = self.get_file(&metadata.path).await? {
            metadata.last_indexed_at = existing.last_indexed_at;
        }
        metadata.status = status;
        self.upsert_file(&metadata).await?;

        Ok(metadata)
    }

    pub async fn mark_file_indexed(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<FileMetadata, MetadataError> {
        let path_ref = path.as_ref();
        let status = self.classify_file(path_ref).await?;

        if status == FileChangeStatus::Deleted {
            let normalized_path = normalize_display_path(path_ref);
            let existing = self.get_file(&normalized_path).await?;
            let metadata = FileMetadata {
                path: normalized_path,
                language: existing.as_ref().and_then(|e| e.language.clone()),
                size_bytes: existing.as_ref().map(|e| e.size_bytes).unwrap_or(0),
                modified_at: existing.as_ref().and_then(|e| e.modified_at.clone()),
                content_hash: existing
                    .as_ref()
                    .map(|e| e.content_hash.clone())
                    .unwrap_or_default(),
                last_indexed_at: Some(current_timestamp()?),
                status: FileChangeStatus::Deleted,
            };
            self.upsert_file(&metadata).await?;
            return Ok(metadata);
        }

        let mut metadata = self.scan_file(path_ref).await?;
        metadata.status = FileChangeStatus::Unchanged;
        metadata.last_indexed_at = Some(current_timestamp()?);
        self.upsert_file(&metadata).await?;

        Ok(metadata)
    }

    pub async fn mark_file_deleted(&self, path: &str) -> Result<(), MetadataError> {
        let path = self.normalize_path(path);
        sqlx::query("UPDATE files SET status = ? WHERE path = ?")
            .bind(FileChangeStatus::Deleted.as_str())
            .bind(path)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_tracked_files(&self, prefix: &str) -> Result<Vec<String>, MetadataError> {
        let rows = sqlx::query(
            "
            SELECT path FROM files
            WHERE (path = ? OR path LIKE ?) AND status != ?
            ",
        )
        .bind(prefix)
        .bind(format!("{}/%", prefix))
        .bind(FileChangeStatus::Deleted.as_str())
        .fetch_all(&self.pool)
        .await?;

        let mut paths = Vec::new();
        for row in rows {
            paths.push(row.try_get("path")?);
        }
        Ok(paths)
    }

    pub async fn begin_index_run(&self) -> Result<i64, MetadataError> {
        let started_at = current_timestamp()?;
        let result = sqlx::query(
            "
            INSERT INTO index_runs (started_at, status)
            VALUES (?, ?)
            ",
        )
        .bind(started_at)
        .bind("running")
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn finish_index_run(
        &self,
        id: i64,
        files_seen: i64,
        files_changed: i64,
        status: &str,
    ) -> Result<(), MetadataError> {
        sqlx::query(
            "
            UPDATE index_runs
            SET finished_at = ?, files_seen = ?, files_changed = ?, status = ?
            WHERE id = ?
            ",
        )
        .bind(current_timestamp()?)
        .bind(files_seen)
        .bind(files_changed)
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn latest_index_run(&self) -> Result<Option<IndexRunSummary>, MetadataError> {
        let row = sqlx::query(
            "
            SELECT id, started_at, finished_at, files_seen, files_changed, status
            FROM index_runs
            ORDER BY id DESC
            LIMIT 1
            ",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_index_run))
    }
}

fn row_to_file_metadata(row: SqliteRow) -> Result<FileMetadata, MetadataError> {
    let status: String = row.try_get("status")?;
    let status =
        FileChangeStatus::from_str(&status).ok_or_else(|| MetadataError::InvalidStatus(status))?;

    Ok(FileMetadata {
        path: row.try_get("path")?,
        language: row.try_get("language")?,
        size_bytes: row.try_get("size_bytes")?,
        modified_at: row.try_get("modified_at")?,
        content_hash: row.try_get("content_hash")?,
        last_indexed_at: row.try_get("last_indexed_at")?,
        status,
    })
}

fn row_to_index_run(row: SqliteRow) -> IndexRunSummary {
    IndexRunSummary {
        id: row.get("id"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        files_seen: row.get("files_seen"),
        files_changed: row.get("files_changed"),
        status: row.get("status"),
    }
}

pub fn current_timestamp() -> Result<String, MetadataError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| MetadataError::SystemClock)?
        .as_millis()
        .to_string())
}

fn timestamp_from_system_time(time: SystemTime) -> String {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn language_from_path(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("js") => Some("javascript"),
        Some("jsx") => Some("jsx"),
        Some("ts") => Some("typescript"),
        Some("tsx") => Some("tsx"),
        _ => None,
    }
}

fn project_root() -> PathBuf {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    for ancestor in current_dir.ancestors() {
        if ancestor.join("package.json").exists() && ancestor.join("src-tauri").exists() {
            return ancestor.to_path_buf();
        }
    }

    current_dir
}

fn normalize_display_path(path: &Path) -> String {
    if path.is_absolute() {
        let root = project_root();
        if let Ok(relative) = path.strip_prefix(&root) {
            return normalize_relative_path(relative);
        } else {
            return path.to_string_lossy().to_string();
        }
    }

    normalize_relative_path(path)
}

fn normalize_relative_path(path: &Path) -> String {
    let mut parts = Vec::new();
    let mut leading_parents = 0;

    for component in path.components() {
        match component {
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
            Component::ParentDir => {
                if parts.pop().is_none() {
                    leading_parents += 1;
                }
            }
            Component::Normal(value) => {
                parts.push(value.to_string_lossy().to_string());
            }
        }
    }

    let mut result = Vec::with_capacity(leading_parents + parts.len());
    for _ in 0..leading_parents {
        result.push("..".to_string());
    }
    result.extend(parts);

    result.join("/")
}

#[cfg(test)]
mod tests {
    use super::{FileChangeStatus, MetadataStore};
    use std::fs;

    #[tokio::test]
    async fn initializes_schema_without_error() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let store = MetadataStore::open(temp_dir.path())
            .await
            .expect("metadata store should open");

        store.init_schema().await.expect("schema should initialize");
    }

    #[tokio::test]
    async fn hashes_same_content_consistently() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let path = temp_dir.path().join("App.tsx");
        fs::write(&path, "export function App() { return null; }")
            .expect("test file should be written");
        let store = MetadataStore::open(temp_dir.path())
            .await
            .expect("metadata store should open");

        let first = store.scan_file(&path).await.expect("file should scan");
        let second = store
            .scan_file(&path)
            .await
            .expect("file should scan again");

        assert_eq!(first.content_hash, second.content_hash);
    }

    #[tokio::test]
    async fn hash_changes_when_content_changes() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let path = temp_dir.path().join("App.tsx");
        let store = MetadataStore::open(temp_dir.path())
            .await
            .expect("metadata store should open");

        fs::write(&path, "export const value = 1;").expect("test file should be written");
        let first = store.scan_file(&path).await.expect("file should scan");
        fs::write(&path, "export const value = 2;").expect("test file should be updated");
        let second = store
            .scan_file(&path)
            .await
            .expect("file should scan again");

        assert_ne!(first.content_hash, second.content_hash);
    }

    #[tokio::test]
    async fn upsert_is_idempotent_and_classifies_changes() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let path = temp_dir.path().join("App.tsx");
        fs::write(&path, "export const value = 1;").expect("test file should be written");
        let store = MetadataStore::open(temp_dir.path())
            .await
            .expect("metadata store should open");

        let first = store
            .record_file_metadata(&path)
            .await
            .expect("metadata should record");
        let second = store
            .record_file_metadata(&path)
            .await
            .expect("metadata should record again");

        assert_eq!(first.status, FileChangeStatus::New);
        assert_eq!(second.status, FileChangeStatus::Unchanged);

        fs::write(&path, "export const value = 2;").expect("test file should be updated");
        let status = store
            .classify_file(&path)
            .await
            .expect("file should classify");

        assert_eq!(status, FileChangeStatus::Changed);
    }
}
