//! Domain types: projects (realms) hold demands (quests) made of stages (rooms).
//!
//! Every field an LLM writes is optional except names, and status/progress are
//! derived when absent, so a half-written file still renders something useful.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// Lifecycle shared by stages, demands and projects.
///
/// Aliases accept the words LLMs tend to reach for ("wip", "busy", ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    #[serde(alias = "pending", alias = "locked")]
    Todo,
    #[serde(alias = "busy", alias = "doing", alias = "wip", alias = "in_progress")]
    Active,
    Blocked,
    #[serde(alias = "complete", alias = "completed")]
    Done,
}

impl Status {
    /// Parses CLI input with the same names and aliases the JSON files accept.
    ///
    /// ```text
    /// Status::parse("WIP") == Some(Status::Active)
    /// ```
    pub fn parse(raw: &str) -> Option<Status> {
        let value = serde_json::Value::String(raw.trim().to_ascii_lowercase());
        serde_json::from_value(value).ok()
    }

    pub fn label(self) -> &'static str {
        match self {
            Status::Todo => "TODO",
            Status::Active => "ACTIVE",
            Status::Blocked => "BLOCKED",
            Status::Done => "DONE",
        }
    }

    /// Plain word for the overworld, where speed of reading beats flavor.
    pub fn headline(self) -> &'static str {
        match self {
            Status::Todo => "NOT STARTED",
            Status::Active => "ACTIVE",
            Status::Blocked => "BLOCKED",
            Status::Done => "DONE",
        }
    }

    /// Single-width glyphs only: wide emoji break column alignment.
    pub fn icon(self) -> &'static str {
        match self {
            Status::Todo => "◇",
            Status::Active => "⚔",
            Status::Blocked => "⊘",
            Status::Done => "✓",
        }
    }

    /// Pipeline order: cleared quests first, then the fight, the walls, the fog.
    pub fn pipeline_rank(self) -> u8 {
        match self {
            Status::Done => 0,
            Status::Active => 1,
            Status::Blocked => 2,
            Status::Todo => 3,
        }
    }
}

/// One step of a demand ("Implement auth", "Run tests").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stage {
    pub name: String,
    #[serde(default)]
    pub status: Status,
    /// 0-100, only meaningful while active or blocked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Stage {
    pub fn new(name: &str) -> Stage {
        Stage {
            name: name.to_string(),
            status: Status::Todo,
            progress: None,
            note: None,
        }
    }

    pub fn percent(&self) -> u8 {
        match self.status {
            Status::Done => 100,
            Status::Todo => 0,
            _ => self.progress.map(clamp_percent).unwrap_or(0),
        }
    }
}

/// Something stopping a demand, and who can remove it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Blocker {
    pub what: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub who: Option<String>,
}

/// A card / issue / user story: the unit of work inside a project.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Demand {
    /// Defaults to the file stem when the file omits it.
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    /// Explicit override; derived from stages and blockers when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    /// Explicit override; averaged from stages when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// One line about what is happening right now (the hero's speech bubble).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
    #[serde(default)]
    pub stages: Vec<Stage>,
    #[serde(default)]
    pub blockers: Vec<Blocker>,
    /// Who is waiting on this demand to finish.
    #[serde(default)]
    pub waiting: Vec<String>,
    /// File mtime, filled by the loader.
    #[serde(skip)]
    pub updated: Option<DateTime<Local>>,
}

impl Demand {
    pub fn new(id: &str) -> Demand {
        Demand {
            id: id.to_string(),
            ..Demand::default()
        }
    }

    pub fn display_title(&self) -> &str {
        if self.title.trim().is_empty() {
            return &self.id;
        }
        &self.title
    }

    /// ```text
    /// stages [done, active] -> Active; any blocker -> Blocked; all done -> Done
    /// ```
    pub fn status(&self) -> Status {
        if let Some(explicit) = self.status {
            return explicit;
        }
        if !self.blockers.is_empty() || self.has_stage(Status::Blocked) {
            return Status::Blocked;
        }
        if !self.stages.is_empty() && self.stages.iter().all(|s| s.status == Status::Done) {
            return Status::Done;
        }
        if self.has_stage(Status::Active) || self.has_stage(Status::Done) || self.percent_hint() {
            return Status::Active;
        }
        Status::Todo
    }

    pub fn percent(&self) -> u8 {
        if self.status() == Status::Done {
            return 100;
        }
        if let Some(raw) = self.progress {
            return clamp_percent(raw);
        }
        mean_percent(self.stages.iter().map(Stage::percent)).unwrap_or(0)
    }

    /// Index of the first stage not yet done: where the hero stands.
    pub fn current_stage(&self) -> Option<usize> {
        self.stages.iter().position(|s| s.status != Status::Done)
    }

    pub fn done_stages(&self) -> usize {
        self.stages
            .iter()
            .filter(|s| s.status == Status::Done)
            .count()
    }

    fn has_stage(&self, status: Status) -> bool {
        self.stages.iter().any(|s| s.status == status)
    }

    fn percent_hint(&self) -> bool {
        self.progress.is_some_and(|p| p > 0.0)
    }
}

/// Optional `project.json` next to the demand files.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<i64>,
}

/// A directory of demands.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub slug: String,
    pub name: String,
    pub order: i64,
    pub demands: Vec<Demand>,
}

impl Project {
    pub fn status(&self) -> Status {
        let all: Vec<Status> = self.demands.iter().map(Demand::status).collect();
        if all.is_empty() {
            return Status::Todo;
        }
        if all.iter().all(|s| *s == Status::Done) {
            return Status::Done;
        }
        if all.contains(&Status::Active) {
            return Status::Active;
        }
        if all.contains(&Status::Blocked) {
            return Status::Blocked;
        }
        if all.contains(&Status::Done) {
            return Status::Active;
        }
        Status::Todo
    }

    pub fn percent(&self) -> u8 {
        mean_percent(self.demands.iter().map(Demand::percent)).unwrap_or(0)
    }

    pub fn count(&self, status: Status) -> usize {
        self.demands.iter().filter(|d| d.status() == status).count()
    }

    pub fn updated(&self) -> Option<DateTime<Local>> {
        self.demands.iter().filter_map(|d| d.updated).max()
    }
}

/// Rounded mean, `None` for an empty input.
///
/// ```text
/// mean_percent([100, 50].into_iter()) == Some(75)
/// ```
pub fn mean_percent(values: impl Iterator<Item = u8>) -> Option<u8> {
    let (sum, count) = values.fold((0u32, 0u32), |(s, c), v| (s + v as u32, c + 1));
    if count == 0 {
        return None;
    }
    Some(((sum as f64) / (count as f64)).round() as u8)
}

fn clamp_percent(raw: f64) -> u8 {
    raw.clamp(0.0, 100.0).round() as u8
}

/// Score for the header: 100 per cleared stage, 500 per finished demand.
pub fn experience(projects: &[Project]) -> u64 {
    let demands = projects.iter().flat_map(|p| p.demands.iter());
    demands
        .map(|d| d.done_stages() as u64 * 100 + if d.status() == Status::Done { 500 } else { 0 })
        .sum()
}

pub fn level(experience: u64) -> u64 {
    1 + experience / 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage(status: Status, progress: Option<f64>) -> Stage {
        Stage {
            progress,
            status,
            ..Stage::new("s")
        }
    }

    fn demand_with(stages: Vec<Stage>) -> Demand {
        Demand {
            stages,
            ..Demand::new("d")
        }
    }

    #[test]
    fn status_parse_accepts_aliases_any_case() {
        assert_eq!(Status::parse("WIP"), Some(Status::Active));
        assert_eq!(Status::parse(" done "), Some(Status::Done));
        assert_eq!(Status::parse("locked"), Some(Status::Todo));
        assert_eq!(Status::parse("nope"), None);
    }

    #[test]
    fn status_labels_and_icons_are_distinct() {
        let all = [Status::Todo, Status::Active, Status::Blocked, Status::Done];
        let icons: std::collections::HashSet<&str> = all.iter().map(|s| s.icon()).collect();
        assert_eq!(icons.len(), 4);
        assert_eq!(Status::Blocked.label(), "BLOCKED");
        assert_eq!(Status::Todo.headline(), "NOT STARTED");
        assert!(Status::Done.pipeline_rank() < Status::Todo.pipeline_rank());
    }

    #[test]
    fn stage_percent_depends_on_status() {
        assert_eq!(stage(Status::Done, None).percent(), 100);
        assert_eq!(stage(Status::Todo, Some(80.0)).percent(), 0);
        assert_eq!(stage(Status::Active, Some(74.4)).percent(), 74);
        assert_eq!(stage(Status::Active, Some(400.0)).percent(), 100);
    }

    #[test]
    fn demand_status_is_derived_from_stages() {
        let fresh = demand_with(vec![stage(Status::Todo, None)]);
        assert_eq!(fresh.status(), Status::Todo);
        let mid = demand_with(vec![stage(Status::Done, None), stage(Status::Todo, None)]);
        assert_eq!(mid.status(), Status::Active);
        let all_done = demand_with(vec![stage(Status::Done, None)]);
        assert_eq!(all_done.status(), Status::Done);
        let stuck = demand_with(vec![stage(Status::Blocked, None)]);
        assert_eq!(stuck.status(), Status::Blocked);
    }

    #[test]
    fn demand_blockers_and_override_win() {
        let mut demand = demand_with(vec![stage(Status::Active, None)]);
        demand.blockers.push(Blocker {
            what: "PO".into(),
            who: None,
        });
        assert_eq!(demand.status(), Status::Blocked);
        demand.status = Some(Status::Done);
        assert_eq!(demand.status(), Status::Done);
        assert_eq!(demand.percent(), 100);
    }

    #[test]
    fn demand_percent_and_current_stage() {
        let demand = demand_with(vec![
            stage(Status::Done, None),
            stage(Status::Active, Some(50.0)),
        ]);
        assert_eq!(demand.percent(), 75);
        assert_eq!(demand.current_stage(), Some(1));
        assert_eq!(demand.done_stages(), 1);
        let explicit = Demand {
            progress: Some(30.0),
            ..Demand::new("x")
        };
        assert_eq!(explicit.percent(), 30);
        assert_eq!(explicit.status(), Status::Active);
    }

    #[test]
    fn display_title_falls_back_to_id() {
        assert_eq!(Demand::new("US-1").display_title(), "US-1");
        let titled = Demand {
            title: "Auth".into(),
            ..Demand::new("US-1")
        };
        assert_eq!(titled.display_title(), "Auth");
    }

    fn project(demands: Vec<Demand>) -> Project {
        Project {
            slug: "p".into(),
            name: "P".into(),
            order: 0,
            demands,
        }
    }

    #[test]
    fn project_status_prefers_active_over_blocked() {
        let done = Demand {
            status: Some(Status::Done),
            ..Demand::new("a")
        };
        let blocked = Demand {
            status: Some(Status::Blocked),
            ..Demand::new("b")
        };
        let active = Demand {
            status: Some(Status::Active),
            ..Demand::new("c")
        };
        assert_eq!(project(vec![]).status(), Status::Todo);
        assert_eq!(project(vec![done.clone()]).status(), Status::Done);
        assert_eq!(
            project(vec![blocked.clone(), active]).status(),
            Status::Active
        );
        assert_eq!(project(vec![blocked.clone()]).status(), Status::Blocked);
        assert_eq!(
            project(vec![done, Demand::new("t")]).status(),
            Status::Active
        );
        assert_eq!(project(vec![blocked]).count(Status::Blocked), 1);
    }

    #[test]
    fn project_percent_and_experience() {
        let done = Demand {
            status: Some(Status::Done),
            ..Demand::new("a")
        };
        let half = demand_with(vec![stage(Status::Done, None), stage(Status::Todo, None)]);
        let realm = project(vec![done, half]);
        assert_eq!(realm.percent(), 75);
        assert_eq!(experience(&[realm]), 600);
        assert_eq!(level(2500), 3);
        assert_eq!(mean_percent(std::iter::empty()), None);
    }
}
