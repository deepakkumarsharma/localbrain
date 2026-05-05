use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::metadata::{current_timestamp, MetadataError};

const REDACTION_MARKER: &str = "[redacted]";

pub fn init_local_logging(log_root: impl AsRef<Path>) -> Result<(), MetadataError> {
    let log_root = log_root.as_ref();
    fs::create_dir_all(log_root)?;
    append_log_line(log_root, "localbrain started")
}

pub fn append_log_line(log_root: impl AsRef<Path>, message: &str) -> Result<(), MetadataError> {
    let log_path = log_root.as_ref().join("localbrain.log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let line = format!("{} {}\n", current_timestamp()?, redact_secrets(message));
    file.write_all(line.as_bytes())?;
    Ok(())
}

pub fn redact_secrets(value: &str) -> String {
    value
        .split_whitespace()
        .map(redact_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_token(token: &str) -> String {
    let lower = token.to_ascii_lowercase();
    if token.starts_with("sk-")
        || lower.starts_with("api_key=")
        || lower.starts_with("apikey=")
        || lower.starts_with("token=")
    {
        return REDACTION_MARKER.to_string();
    }

    token.to_string()
}

#[cfg(test)]
mod tests {
    use super::{append_log_line, redact_secrets};
    use std::fs;

    #[test]
    fn redacts_common_secret_shapes() {
        let redacted = redact_secrets("token=abc sk-test api_key=value safe");

        assert_eq!(redacted, "[redacted] [redacted] [redacted] safe");
    }

    #[test]
    fn writes_local_log_without_secret() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        append_log_line(temp_dir.path(), "started token=abc").expect("log should write");
        let log = fs::read_to_string(temp_dir.path().join("localbrain.log"))
            .expect("log should be readable");

        assert!(log.contains("[redacted]"));
        assert!(!log.contains("token=abc"));
    }
}
