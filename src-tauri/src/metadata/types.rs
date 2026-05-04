use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileChangeStatus {
    New,
    Changed,
    Unchanged,
    Deleted,
    Error,
}

impl FileChangeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Changed => "changed",
            Self::Unchanged => "unchanged",
            Self::Deleted => "deleted",
            Self::Error => "error",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "new" => Some(Self::New),
            "changed" => Some(Self::Changed),
            "unchanged" => Some(Self::Unchanged),
            "deleted" => Some(Self::Deleted),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub path: String,
    pub language: Option<String>,
    pub size_bytes: i64,
    pub modified_at: Option<String>,
    pub content_hash: String,
    pub last_indexed_at: Option<String>,
    pub status: FileChangeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexRunSummary {
    pub id: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub files_seen: i64,
    pub files_changed: i64,
    pub status: String,
}
