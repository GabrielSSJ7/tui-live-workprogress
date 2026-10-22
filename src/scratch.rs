//! Per-test scratch directories under the system temp dir (no tempfile crate).
//!
//! Each test passes a unique tag so parallel tests never share a directory.

use std::path::PathBuf;

pub fn dir(tag: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("questlog-test-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("creating scratch dir");
    path
}
