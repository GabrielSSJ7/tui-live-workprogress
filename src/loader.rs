//! Reads the data root into a [`World`]. Never fails as a whole: a broken file
//! becomes a line in `problems` so the rest of the map keeps rendering.
//!
//! Layout:
//! ```text
//! <root>/log.jsonl                 activity log (append-only)
//! <root>/<project>/project.json    optional {"name", "order"}
//! <root>/<project>/<demand>.json   one demand per file
//! ```

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use chrono::{DateTime, Local};

use crate::activity::{self, LogEntry};
use crate::model::{Demand, Project, ProjectMeta};
use crate::paths;

pub const META_FILE: &str = "project.json";
const LOG_TAIL: usize = 500;

#[derive(Debug, Default)]
pub struct World {
    pub projects: Vec<Project>,
    pub log: Vec<LogEntry>,
    /// Human-readable load errors, each naming the file and the expected shape.
    pub problems: Vec<String>,
}

impl World {
    /// Newest demand-file write or log line: drives the "updated Xs ago" pulse.
    pub fn last_activity(&self) -> Option<DateTime<Local>> {
        let files = self.projects.iter().filter_map(Project::updated);
        files.chain(self.log.iter().map(|e| e.ts)).max()
    }
}

pub fn load_world(root: &Path) -> World {
    let mut problems = Vec::new();
    let mut projects: Vec<Project> = project_dirs(root)
        .iter()
        .map(|dir| load_project(dir, &mut problems))
        .collect();
    projects.sort_by(|a, b| (a.order, &a.name).cmp(&(b.order, &b.name)));
    let tail = activity::read_tail(&paths::log_path(root), LOG_TAIL);
    if tail.bad_lines > 0 {
        problems.push(format!(
            "log.jsonl: {} unreadable line(s), expected {{\"ts\",\"msg\",...}} per line",
            tail.bad_lines
        ));
    }
    World {
        projects,
        log: tail.entries,
        problems,
    }
}

/// Cheap change detector: hashes names, sizes and mtimes two levels deep.
pub fn fingerprint(root: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    stamp(&paths::log_path(root), &mut hasher);
    for dir in project_dirs(root) {
        stamp(&dir, &mut hasher);
        visible_entries(&dir)
            .iter()
            .for_each(|f| stamp(f, &mut hasher));
    }
    hasher.finish()
}

/// `backend-citadel` -> `Backend Citadel`.
pub fn prettify_slug(slug: &str) -> String {
    slug.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn stamp(path: &Path, hasher: &mut DefaultHasher) {
    path.hash(hasher);
    if let Ok(meta) = std::fs::metadata(path) {
        meta.len().hash(hasher);
        meta.modified().ok().hash(hasher);
    }
}

/// Sorted entries, skipping dotfiles (atomic-write temp files start with '.').
fn visible_entries(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = read
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| !file_name(p).starts_with('.'))
        .collect();
    paths.sort();
    paths
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn project_dirs(root: &Path) -> Vec<PathBuf> {
    visible_entries(root)
        .into_iter()
        .filter(|p| p.is_dir())
        .collect()
}

fn demand_files(dir: &Path) -> Vec<PathBuf> {
    visible_entries(dir)
        .into_iter()
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "json"))
        .filter(|p| file_name(p) != META_FILE)
        .collect()
}

fn load_project(dir: &Path, problems: &mut Vec<String>) -> Project {
    let slug = file_name(dir);
    let meta = read_meta(dir, problems);
    let mut demands = Vec::new();
    for path in demand_files(dir) {
        match load_demand(&path) {
            Ok(demand) => demands.push(demand),
            Err(err) => problems.push(format!("{slug}/{}: {err:#}", file_name(&path))),
        }
    }
    demands.sort_by(|a, b| {
        (a.status().pipeline_rank(), &a.id).cmp(&(b.status().pipeline_rank(), &b.id))
    });
    Project {
        name: meta.name.unwrap_or_else(|| prettify_slug(&slug)),
        order: meta.order.unwrap_or(i64::MAX),
        slug,
        demands,
    }
}

fn read_meta(dir: &Path, problems: &mut Vec<String>) -> ProjectMeta {
    let path = dir.join(META_FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return ProjectMeta::default();
    };
    serde_json::from_str(&text).unwrap_or_else(|err| {
        problems.push(format!(
            "{}/{META_FILE}: {err}, expected {{\"name\",\"order\"}}",
            file_name(dir)
        ));
        ProjectMeta::default()
    })
}

/// Parses one demand file; the id defaults to the file stem.
pub fn load_demand(path: &Path) -> Result<Demand> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {path:?}"))?;
    let mut demand: Demand = serde_json::from_str(&text)
        .context("expected {\"title\", \"stages\":[{\"name\",\"status\"}], ...}")?;
    if demand.id.trim().is_empty() {
        demand.id = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
    }
    demand.updated = modified_at(path);
    Ok(demand)
}

fn modified_at(path: &Path) -> Option<DateTime<Local>> {
    let modified: SystemTime = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(DateTime::<Local>::from(modified))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Status;
    use crate::scratch;

    fn write(path: PathBuf, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn loads_projects_sorted_with_meta_and_problems() {
        let root = scratch::dir("loader-world");
        write(
            root.join("zeta/project.json"),
            r#"{"name":"Zeta Keep","order":1}"#,
        );
        write(
            root.join("zeta/US-1.json"),
            r#"{"title":"Auth","stages":[{"name":"a","status":"done"}]}"#,
        );
        write(root.join("alpha-realm/broken.json"), "{oops");
        write(root.join("alpha-realm/.tmp.json"), "{oops");
        write(root.join("alpha-realm/notes.md"), "ignored");
        let world = load_world(&root);
        let names: Vec<&str> = world.projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Zeta Keep", "Alpha Realm"]);
        assert_eq!(world.projects[0].demands[0].id, "US-1");
        assert_eq!(world.projects[0].demands[0].status(), Status::Done);
        assert!(world.projects[0].updated().is_some());
        assert_eq!(world.last_activity(), world.projects[0].updated());
        assert_eq!(world.problems.len(), 1);
        assert!(
            world.problems[0].starts_with("alpha-realm/broken.json"),
            "{:?}",
            world.problems
        );
    }

    #[test]
    fn demands_follow_pipeline_order() {
        let root = scratch::dir("loader-order");
        write(root.join("p/a.json"), r#"{"status":"todo"}"#);
        write(root.join("p/b.json"), r#"{"status":"done"}"#);
        write(root.join("p/c.json"), r#"{"status":"busy"}"#);
        let ids: Vec<String> = load_world(&root).projects[0]
            .demands
            .iter()
            .map(|d| d.id.clone())
            .collect();
        assert_eq!(ids, ["b", "c", "a"]);
    }

    #[test]
    fn bad_meta_and_bad_log_are_reported() {
        let root = scratch::dir("loader-bad-meta");
        write(root.join("p/project.json"), r#"{"order":"first"}"#);
        write(root.join("log.jsonl"), "nope\n");
        let world = load_world(&root);
        assert_eq!(world.projects[0].name, "P");
        assert_eq!(world.problems.len(), 2, "{:?}", world.problems);
    }

    #[test]
    fn fingerprint_changes_when_a_file_changes() {
        let root = scratch::dir("loader-fingerprint");
        write(root.join("p/a.json"), "{}");
        let before = fingerprint(&root);
        assert_eq!(before, fingerprint(&root));
        write(root.join("p/a.json"), r#"{"title":"longer"}"#);
        assert_ne!(before, fingerprint(&root));
    }

    #[test]
    fn missing_root_is_an_empty_world() {
        let world = load_world(Path::new("/nonexistent/questlog-root"));
        assert!(world.projects.is_empty() && world.problems.is_empty());
    }

    #[test]
    fn prettify_slug_title_cases_words() {
        assert_eq!(prettify_slug("backend-citadel"), "Backend Citadel");
        assert_eq!(prettify_slug("db__dungeon"), "Db Dungeon");
    }
}
