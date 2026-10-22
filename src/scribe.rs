//! Safe writes for the CLI: validated ids, atomic file replacement, and pure
//! edit helpers so a harness never has to hand-craft JSON.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::loader::{self, META_FILE};
use crate::model::{Demand, Stage, Status};

/// Ids become path components, so only a conservative charset is allowed.
///
/// ```text
/// validate_slug("US-1234") -> Ok; validate_slug("../etc") -> Err
/// ```
pub fn validate_slug(raw: &str) -> Result<&str> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
    if !raw.is_empty() && !raw.starts_with('.') && raw.chars().all(allowed) {
        return Ok(raw);
    }
    bail!("invalid id {raw:?}: expected letters, digits, '-', '_' or '.', not starting with '.'")
}

pub fn demand_path(root: &Path, project: &str, demand: &str) -> Result<PathBuf> {
    let file = format!("{}.json", validate_slug(demand)?);
    if file == META_FILE {
        bail!("invalid demand id {demand:?}: \"project\" is reserved for project.json");
    }
    Ok(root.join(validate_slug(project)?).join(file))
}

pub fn project_meta_path(root: &Path, project: &str) -> Result<PathBuf> {
    Ok(root.join(validate_slug(project)?).join(META_FILE))
}

/// Existing demand, or a fresh one with `id` when the file does not exist yet.
pub fn load_or_new(path: &Path, id: &str) -> Result<Demand> {
    if !path.exists() {
        return Ok(Demand::new(id));
    }
    loader::load_demand(path)
}

/// Write to a hidden temp file then rename, so the TUI never reads half a file.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("path {path:?} has no parent dir"))?;
    std::fs::create_dir_all(parent).with_context(|| format!("creating {parent:?}"))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = parent.join(format!(".{name}.tmp"));
    let body = serde_json::to_string_pretty(value)? + "\n";
    std::fs::write(&temp, body).with_context(|| format!("writing {temp:?}"))?;
    std::fs::rename(&temp, path).with_context(|| format!("renaming {temp:?} to {path:?}"))
}

/// Set a stage's status (and progress), appending the stage if it is new.
///
/// Clearing the demand-level override lets the new stage state show through.
pub fn upsert_stage(demand: &mut Demand, name: &str, status: Status, progress: Option<f64>) {
    demand.status = None;
    demand.progress = None;
    let index = match demand.stages.iter().position(|s| s.name == name) {
        Some(found) => found,
        None => {
            demand.stages.push(Stage::new(name));
            demand.stages.len() - 1
        }
    };
    let stage = &mut demand.stages[index];
    stage.status = status;
    stage.progress = progress;
}

/// Append the named stages that are not there yet, as todo, keeping order.
pub fn seed_stages(demand: &mut Demand, names: &[String]) {
    for name in names {
        if !demand.stages.iter().any(|s| &s.name == name) {
            demand.stages.push(Stage::new(name));
        }
    }
}

/// Finish everything: all stages done, blockers cleared.
pub fn complete(demand: &mut Demand) {
    demand
        .stages
        .iter_mut()
        .for_each(|s| s.status = Status::Done);
    demand.blockers.clear();
    demand.progress = None;
    demand.status = demand.stages.is_empty().then_some(Status::Done);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch;

    #[test]
    fn validate_slug_rejects_paths() {
        assert!(validate_slug("US-1234").is_ok());
        assert!(validate_slug("v1.2_x").is_ok());
        let err = validate_slug("../etc").unwrap_err().to_string();
        assert!(err.contains("\"../etc\""), "{err}");
        assert!(validate_slug("").is_err());
        assert!(validate_slug("a/b").is_err());
    }

    #[test]
    fn demand_path_reserves_project_json() {
        let root = Path::new("/r");
        assert_eq!(
            demand_path(root, "p", "d").unwrap(),
            PathBuf::from("/r/p/d.json")
        );
        assert!(demand_path(root, "p", "project").is_err());
        assert_eq!(
            project_meta_path(root, "p").unwrap(),
            PathBuf::from("/r/p/project.json")
        );
    }

    #[test]
    fn save_then_load_roundtrip_leaves_no_temp() {
        let dir = scratch::dir("scribe-roundtrip");
        let path = dir.join("p/US-1.json");
        assert_eq!(load_or_new(&path, "US-1").unwrap().id, "US-1");
        let mut demand = Demand::new("US-1");
        seed_stages(&mut demand, &["a".into(), "b".into()]);
        save_json(&path, &demand).unwrap();
        let back = load_or_new(&path, "ignored").unwrap();
        assert_eq!(back.stages.len(), 2);
        let leftovers = std::fs::read_dir(dir.join("p")).unwrap().count();
        assert_eq!(leftovers, 1);
    }

    #[test]
    fn upsert_stage_updates_or_appends_and_clears_override() {
        let mut demand = Demand {
            status: Some(Status::Todo),
            ..Demand::new("d")
        };
        seed_stages(&mut demand, &["a".into()]);
        upsert_stage(&mut demand, "a", Status::Active, Some(40.0));
        upsert_stage(&mut demand, "b", Status::Todo, None);
        assert_eq!(demand.stages.len(), 2);
        assert_eq!(demand.stages[0].percent(), 40);
        assert_eq!(demand.status, None);
        assert_eq!(demand.status(), Status::Active);
    }

    #[test]
    fn seed_stages_skips_existing() {
        let mut demand = Demand::new("d");
        seed_stages(&mut demand, &["a".into(), "b".into()]);
        seed_stages(&mut demand, &["b".into(), "c".into()]);
        let names: Vec<&str> = demand.stages.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c"]);
    }

    #[test]
    fn complete_marks_everything_done() {
        let mut staged = Demand::new("d");
        seed_stages(&mut staged, &["a".into()]);
        complete(&mut staged);
        assert_eq!((staged.status, staged.status()), (None, Status::Done));
        let mut bare = Demand::new("e");
        complete(&mut bare);
        assert_eq!(bare.status(), Status::Done);
    }
}
