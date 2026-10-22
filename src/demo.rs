//! `questlog demo`: seeds an empty data dir with realms covering every state,
//! so the map can be seen before any harness has written anything.

use std::path::Path;

use anyhow::{bail, Result};
use chrono::{Duration, Local};

use crate::activity::{self, LogEntry};
use crate::loader;
use crate::model::{Blocker, Demand, ProjectMeta, Stage, Status};
use crate::{paths, scribe};

struct DemoStage(&'static str, Status, Option<f64>);

struct DemoQuest {
    project: &'static str,
    id: &'static str,
    title: &'static str,
    focus: Option<&'static str>,
    stages: &'static [DemoStage],
    blocker: Option<(&'static str, &'static str)>,
}

const REALMS: [(&str, &str); 5] = [
    ("backend-citadel", "Backend Citadel"),
    ("database-dungeon", "Database Dungeon"),
    ("frontend-realm", "Frontend Realm"),
    ("arcane-archive", "Arcane Archive"),
    ("deployment-summit", "Deployment Summit"),
];

use Status::{Active, Blocked, Done, Todo};

const QUESTS: &[DemoQuest] = &[
    DemoQuest {
        project: "backend-citadel",
        id: "CORE-1",
        title: "Core domain models",
        focus: None,
        stages: &[
            DemoStage("Analyze", Done, None),
            DemoStage("Model", Done, None),
            DemoStage("Tests", Done, None),
        ],
        blocker: None,
    },
    DemoQuest {
        project: "backend-citadel",
        id: "CORE-2",
        title: "Auth service",
        focus: None,
        stages: &[
            DemoStage("Design", Done, None),
            DemoStage("Implement", Done, None),
        ],
        blocker: None,
    },
    DemoQuest {
        project: "database-dungeon",
        id: "DB-10",
        title: "Schema migration v2",
        focus: Some("Backfilling 2M rows in batches of 10k"),
        stages: &[
            DemoStage("Analyze", Done, None),
            DemoStage("Migrations", Done, None),
            DemoStage("Backfill", Active, Some(60.0)),
            DemoStage("Verify", Todo, None),
        ],
        blocker: None,
    },
    DemoQuest {
        project: "database-dungeon",
        id: "DB-11",
        title: "Query tuning",
        focus: None,
        stages: &[
            DemoStage("Profile", Done, None),
            DemoStage("Indexes", Done, None),
        ],
        blocker: None,
    },
    DemoQuest {
        project: "frontend-realm",
        id: "WEB-7",
        title: "Login screen",
        focus: Some("Wiring the auth form to /token"),
        stages: &[
            DemoStage("Design", Done, None),
            DemoStage("Build", Active, Some(45.0)),
            DemoStage("Tests", Todo, None),
            DemoStage("Review", Todo, None),
        ],
        blocker: None,
    },
    DemoQuest {
        project: "frontend-realm",
        id: "WEB-8",
        title: "Billing page",
        focus: None,
        stages: &[
            DemoStage("Design", Active, Some(20.0)),
            DemoStage("Build", Todo, None),
        ],
        blocker: Some(("Pricing rules from PO", "PO")),
    },
    DemoQuest {
        project: "frontend-realm",
        id: "WEB-9",
        title: "Dark mode",
        focus: None,
        stages: &[
            DemoStage("Tokens", Todo, None),
            DemoStage("Apply", Todo, None),
        ],
        blocker: None,
    },
    DemoQuest {
        project: "arcane-archive",
        id: "DOC-1",
        title: "API reference",
        focus: None,
        stages: &[
            DemoStage("Outline", Done, None),
            DemoStage("Write", Blocked, Some(30.0)),
        ],
        blocker: Some(("Final schema (database-dungeon/DB-10)", "Claude")),
    },
    DemoQuest {
        project: "deployment-summit",
        id: "OPS-1",
        title: "CI pipeline",
        focus: None,
        stages: &[
            DemoStage("Runners", Todo, None),
            DemoStage("Cache", Todo, None),
        ],
        blocker: None,
    },
];

const EVENTS: &[(i64, &str, &str, &str, &str)] = &[
    (
        190,
        "backend-citadel",
        "CORE-1",
        "done",
        "Citadel walls raised: core models complete",
    ),
    (
        150,
        "backend-citadel",
        "CORE-2",
        "done",
        "Auth service sealed and tested",
    ),
    (
        95,
        "database-dungeon",
        "DB-11",
        "done",
        "Slow queries slain: p95 400ms → 40ms",
    ),
    (
        70,
        "database-dungeon",
        "DB-10",
        "start",
        "Entered the dungeon: schema migration v2",
    ),
    (
        52,
        "arcane-archive",
        "DOC-1",
        "blocked",
        "Blocked: waiting final schema from DB-10",
    ),
    (
        40,
        "frontend-realm",
        "WEB-8",
        "blocked",
        "Blocked: pricing rules from PO",
    ),
    (
        25,
        "frontend-realm",
        "WEB-7",
        "start",
        "Claude entered Frontend Realm (Login screen)",
    ),
    (
        12,
        "frontend-realm",
        "WEB-7",
        "file",
        "Spawned src/auth/LoginForm.tsx",
    ),
    (
        6,
        "database-dungeon",
        "DB-10",
        "progress",
        "Backfill → ACTIVE 60%",
    ),
    (
        2,
        "frontend-realm",
        "WEB-7",
        "test",
        "Guarding LoginForm with 6 tests",
    ),
];

/// Seeds `root` when it holds no projects yet; refuses to touch real data.
pub fn seed(root: &Path) -> Result<()> {
    if !loader::load_world(root).projects.is_empty() {
        bail!("{root:?} already has projects; demo only seeds an empty dir (try QUESTLOG_DIR=/tmp/questlog-demo questlog demo)");
    }
    seed_realms(root)?;
    QUESTS.iter().try_for_each(|q| {
        scribe::save_json(&scribe::demand_path(root, q.project, q.id)?, &build(q))
    })?;
    seed_events(root)?;
    println!("Seeded {} realms in {}", REALMS.len(), root.display());
    Ok(())
}

fn seed_realms(root: &Path) -> Result<()> {
    for (index, (slug, name)) in REALMS.iter().enumerate() {
        let meta = ProjectMeta {
            name: Some(name.to_string()),
            order: Some(index as i64 + 1),
        };
        scribe::save_json(&scribe::project_meta_path(root, slug)?, &meta)?;
    }
    Ok(())
}

fn seed_events(root: &Path) -> Result<()> {
    let now = Local::now();
    for (minutes_ago, project, demand, kind, msg) in EVENTS {
        let mut entry = LogEntry::now(project, Some(demand), kind, msg);
        entry.ts = now - Duration::minutes(*minutes_ago);
        activity::append(&paths::log_path(root), &entry)?;
    }
    Ok(())
}

fn build(quest: &DemoQuest) -> Demand {
    let stages = quest.stages.iter().map(|s| Stage {
        status: s.1,
        progress: s.2,
        ..Stage::new(s.0)
    });
    let blockers = quest.blocker.iter().map(|(what, who)| Blocker {
        what: what.to_string(),
        who: Some(who.to_string()),
    });
    Demand {
        title: quest.title.to_string(),
        owner: Some("claude".to_string()),
        focus: quest.focus.map(str::to_string),
        stages: stages.collect(),
        blockers: blockers.collect(),
        ..Demand::new(quest.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch;

    #[test]
    fn seed_covers_every_realm_state_and_refuses_twice() {
        let root = scratch::dir("demo-seed");
        seed(&root).unwrap();
        let world = loader::load_world(&root);
        assert!(world.problems.is_empty(), "{:?}", world.problems);
        let states: Vec<Status> = world.projects.iter().map(|p| p.status()).collect();
        assert_eq!(states, [Done, Active, Active, Blocked, Todo]);
        assert_eq!(world.log.len(), EVENTS.len());
        assert!(seed(&root).is_err());
    }
}
