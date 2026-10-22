//! Instructions printed by `questlog guide`, meant for any LLM harness:
//! CLAUDE.md, AGENTS.md, a system prompt, a hook.
//!
//! The text lives in `docs/QUESTLOG-CLAUDE.md` so the same file can be imported
//! directly (e.g. `@QUESTLOG.md` in ~/.claude/CLAUDE.md) and printed here.

use std::path::Path;

const GUIDE: &str = include_str!("../docs/QUESTLOG-CLAUDE.md");
/// How the doc spells the data dir; replaced by the real root when printed.
const DEFAULT_ROOT_LABEL: &str = "~/.local/tui-live-workprogress";

/// The guide with the real data directory filled in.
///
/// ```text
/// guide_text(Path::new("/data")).contains("/data/log.jsonl")
/// ```
pub fn guide_text(root: &Path) -> String {
    GUIDE.replace(DEFAULT_ROOT_LABEL, &root.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_names_the_real_root() {
        let text = guide_text(Path::new("/data"));
        assert!(text.contains("/data/log.jsonl"));
        assert!(!text.contains(DEFAULT_ROOT_LABEL));
    }

    #[test]
    fn guide_example_demand_parses() {
        let start = GUIDE.find("{\n  \"title\"").unwrap();
        let end = start + GUIDE[start..].find("\n}\n").unwrap() + 2;
        let demand: crate::model::Demand = serde_json::from_str(&GUIDE[start..end]).unwrap();
        assert_eq!(demand.status(), crate::model::Status::Blocked);
    }

    #[test]
    fn guide_example_log_line_parses() {
        let start = GUIDE.find("{\"ts\"").unwrap();
        let end = start + GUIDE[start..].find('\n').unwrap();
        assert!(crate::activity::parse_line(&GUIDE[start..end]).is_ok());
    }
}
