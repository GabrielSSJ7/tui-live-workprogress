use std::path::{Path, PathBuf};

/// Data root: `$QUESTLOG_DIR` if set, else `~/.local/tui-live-workprogress`.
///
/// The caller creates it when writing; reading a missing root yields an empty world.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("QUESTLOG_DIR") {
        return PathBuf::from(dir);
    }
    let home = std::env::var_os("HOME").expect("HOME env var not set");
    PathBuf::from(home).join(".local/tui-live-workprogress")
}

/// Append-only activity log shared by every project.
pub fn log_path(root: &Path) -> PathBuf {
    root.join("log.jsonl")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_lives_at_root() {
        assert_eq!(log_path(Path::new("/r")), PathBuf::from("/r/log.jsonl"));
    }
}
