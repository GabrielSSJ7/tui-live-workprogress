//! `questlog brief`: plain-text snapshot of open quests for an LLM at session
//! start (wired as a SessionStart hook), so it reuses ids instead of inventing new ones.

use std::path::Path;

use crate::loader::World;
use crate::model::{Demand, Project, Status};

/// More rows than this and the hook output starts crowding the context.
const MAX_ROWS: usize = 25;

/// ```text
/// QUESTLOG ACTIVE — report progress with `questlog` ...
/// Open quests (2):
///   api / US-1234  ACTIVE 40%  stage 2/4 Implement  focus: "..."
/// ```
pub fn brief_text(world: &World, root: &Path) -> String {
    let open = open_quests(world);
    let mut out = format!(
        "QUESTLOG ACTIVE — report progress with `questlog` (rules: ~/.claude/QUESTLOG.md, or `questlog guide`).\nData: {}\n",
        root.display()
    );
    out.push_str(&format!("Open quests ({}):\n", open.len()));
    if open.is_empty() {
        out.push_str("  (none — create one with `questlog quest <project> <demand> \"<title>\" <Stage...>`)\n");
    }
    open.iter()
        .take(MAX_ROWS)
        .for_each(|(p, d)| out.push_str(&format!("  {}\n", quest_row(p, d))));
    if open.len() > MAX_ROWS {
        out.push_str(&format!("  … and {} more\n", open.len() - MAX_ROWS));
    }
    world
        .problems
        .iter()
        .for_each(|p| out.push_str(&format!("⚠ broken: {p}\n")));
    out
}

/// Every demand not done, active first, then blocked, then todo.
pub fn open_quests(world: &World) -> Vec<(&Project, &Demand)> {
    let mut open: Vec<(&Project, &Demand)> = world
        .projects
        .iter()
        .flat_map(|p| p.demands.iter().map(move |d| (p, d)))
        .filter(|(_, d)| d.status() != Status::Done)
        .collect();
    open.sort_by_key(|(_, d)| d.status().pipeline_rank());
    open
}

/// `api / US-1234  ACTIVE 40%  "JWT auth"  stage 2/4 Implement  focus: "..."`
pub fn quest_row(project: &Project, demand: &Demand) -> String {
    let mut row = format!(
        "{} / {}  {} {}%  \"{}\"",
        project.slug,
        demand.id,
        demand.status().label(),
        demand.percent(),
        demand.display_title()
    );
    if let Some(index) = demand.current_stage() {
        let stage = &demand.stages[index];
        row.push_str(&format!(
            "  stage {}/{} {}",
            index + 1,
            demand.stages.len(),
            stage.name
        ));
    }
    for blocker in &demand.blockers {
        let who = blocker
            .who
            .as_deref()
            .map(|w| format!(" → {w}"))
            .unwrap_or_default();
        row.push_str(&format!("  blocked: {}{who}", blocker.what));
    }
    if let Some(focus) = &demand.focus {
        row.push_str(&format!("  focus: \"{focus}\""));
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch;

    #[test]
    fn brief_lists_open_quests_active_first() {
        let root = scratch::dir("brief-demo");
        crate::demo::seed(&root).unwrap();
        let world = crate::loader::load_world(&root);
        let text = brief_text(&world, &root);
        assert!(text.starts_with("QUESTLOG ACTIVE"));
        assert!(text.contains("Open quests (6):"), "{text}");
        let first = text.lines().nth(3).unwrap();
        assert!(first.contains("ACTIVE"), "{first}");
        assert!(text.contains("blocked: Pricing rules from PO → PO"));
        assert!(!text.contains("CORE-1"));
    }

    #[test]
    fn brief_of_empty_world_explains_how_to_start() {
        let text = brief_text(&World::default(), Path::new("/r"));
        assert!(text.contains("Open quests (0):") && text.contains("questlog quest"));
    }

    #[test]
    fn quest_row_shows_stage_and_focus() {
        let demand = Demand {
            title: "Auth".into(),
            focus: Some("wiring".into()),
            stages: vec![crate::model::Stage::new("Build")],
            ..Demand::new("US-1")
        };
        let project = Project {
            slug: "api".into(),
            name: "Api".into(),
            order: 0,
            demands: vec![],
        };
        assert_eq!(
            quest_row(&project, &demand),
            "api / US-1  TODO 0%  \"Auth\"  stage 1/1 Build  focus: \"wiring\""
        );
    }
}
