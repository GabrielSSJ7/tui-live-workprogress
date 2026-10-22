//! The chronicle: an append-only JSONL log of what happened, newest last on disk.
//!
//! One line per event keeps writes trivial for any harness (`echo ... >> log.jsonl`)
//! and lets a torn or hand-mangled line cost only that line.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::clock;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LogEntry {
    pub ts: DateTime<Local>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demand: Option<String>,
    pub kind: String,
    pub msg: String,
}

/// On-disk shape; `ts` stays a string so naive timestamps still parse.
#[derive(Deserialize)]
struct RawLogEntry {
    ts: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    demand: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    msg: String,
}

impl LogEntry {
    pub fn now(project: &str, demand: Option<&str>, kind: &str, msg: &str) -> LogEntry {
        LogEntry {
            ts: Local::now(),
            project: Some(project.to_string()),
            demand: demand.map(str::to_string),
            kind: kind.to_string(),
            msg: msg.to_string(),
        }
    }
}

/// ```text
/// parse_line(r#"{"ts":"2026-10-22 10:55:12","msg":"hi"}"#) -> kind "note"
/// ```
pub fn parse_line(line: &str) -> Result<LogEntry> {
    let raw: RawLogEntry = serde_json::from_str(line)
        .with_context(|| format!("log line {line:?}: expected {{\"ts\",\"msg\",...}}"))?;
    let ts = clock::parse_loose(&raw.ts).ok_or_else(|| {
        anyhow!(
            "log ts {:?}: expected RFC 3339 or YYYY-MM-DD HH:MM:SS",
            raw.ts
        )
    })?;
    Ok(LogEntry {
        ts,
        project: raw.project,
        demand: raw.demand,
        kind: raw.kind.unwrap_or_else(|| "note".to_string()),
        msg: raw.msg,
    })
}

/// Last `max` readable entries in file order, plus how many lines were unreadable.
#[derive(Debug, Default)]
pub struct LogTail {
    pub entries: Vec<LogEntry>,
    pub bad_lines: usize,
}

/// A missing file is an empty log, not an error: fresh installs start that way.
pub fn read_tail(path: &Path, max: usize) -> LogTail {
    let Ok(text) = std::fs::read_to_string(path) else {
        return LogTail::default();
    };
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let start = lines.len().saturating_sub(max);
    let mut tail = LogTail::default();
    for line in &lines[start..] {
        match parse_line(line) {
            Ok(entry) => tail.entries.push(entry),
            Err(_) => tail.bad_lines += 1,
        }
    }
    tail
}

pub fn append(path: &Path, entry: &LogEntry) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating {parent:?}"))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening log {path:?}"))?;
    let line = serde_json::to_string(entry)?;
    writeln!(file, "{line}").with_context(|| format!("appending to {path:?}"))
}

/// Newest first, optionally narrowed to a project and a demand inside it.
pub fn matching<'a>(
    entries: &'a [LogEntry],
    project: Option<&str>,
    demand: Option<&str>,
) -> Vec<&'a LogEntry> {
    let wanted =
        |want: Option<&str>, got: &Option<String>| want.is_none() || got.as_deref() == want;
    entries
        .iter()
        .rev()
        .filter(|e| wanted(project, &e.project) && wanted(demand, &e.demand))
        .collect()
}

/// Glyph for a log kind; unknown kinds still get a bullet.
pub fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "start" => "»",
        "progress" => "⚔",
        "done" => "✓",
        "blocked" => "⊘",
        "unblocked" => "◆",
        "file" => "✚",
        "test" => "⚗",
        "fail" => "✗",
        "note" => "•",
        _ => "·",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch;

    #[test]
    fn parse_line_defaults_kind_and_accepts_naive_ts() {
        let entry = parse_line(r#"{"ts":"2026-10-22 10:55:12","msg":"hi"}"#).unwrap();
        assert_eq!(entry.kind, "note");
        assert_eq!(entry.project, None);
    }

    #[test]
    fn parse_line_errors_name_the_bad_value() {
        let err = parse_line(r#"{"ts":"soon","msg":"x"}"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("\"soon\""), "{err}");
        assert!(parse_line("not json").is_err());
    }

    #[test]
    fn append_then_read_tail_roundtrip() {
        let dir = scratch::dir("activity-roundtrip");
        let path = dir.join("log.jsonl");
        for i in 0..5 {
            append(
                &path,
                &LogEntry::now("p", Some("d"), "note", &format!("m{i}")),
            )
            .unwrap();
        }
        std::fs::write(&path, std::fs::read_to_string(&path).unwrap() + "garbage\n").unwrap();
        let tail = read_tail(&path, 3);
        assert_eq!(tail.bad_lines, 1);
        assert_eq!(
            tail.entries
                .iter()
                .map(|e| e.msg.as_str())
                .collect::<Vec<_>>(),
            ["m3", "m4"]
        );
    }

    #[test]
    fn read_tail_of_missing_file_is_empty() {
        let tail = read_tail(Path::new("/nonexistent/questlog/log.jsonl"), 10);
        assert!(tail.entries.is_empty());
        assert_eq!(tail.bad_lines, 0);
    }

    #[test]
    fn matching_filters_newest_first() {
        let entries = vec![
            LogEntry::now("a", Some("x"), "note", "1"),
            LogEntry::now("b", None, "note", "2"),
            LogEntry::now("a", Some("y"), "note", "3"),
        ];
        let msgs = |v: Vec<&LogEntry>| v.iter().map(|e| e.msg.clone()).collect::<Vec<_>>();
        assert_eq!(msgs(matching(&entries, None, None)), ["3", "2", "1"]);
        assert_eq!(msgs(matching(&entries, Some("a"), None)), ["3", "1"]);
        assert_eq!(msgs(matching(&entries, Some("a"), Some("x"))), ["1"]);
    }

    #[test]
    fn kind_icon_has_fallback() {
        assert_eq!(kind_icon("done"), "✓");
        assert_eq!(kind_icon("whatever"), "·");
    }
}
