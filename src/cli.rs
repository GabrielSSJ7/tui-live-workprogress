//! Command router. With no arguments it opens the TUI; every other command is a
//! small, safe mutation meant to be called by an LLM harness, and each one also
//! appends a line to the chronicle so the activity log fills itself.

use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};

use crate::activity::{self, LogEntry};
use crate::model::{Blocker, Demand, ProjectMeta, Status};
use crate::{brief, demo, guide, loader, paths, scribe, tui};

pub const HELP: &str = "\
questlog — live D&D-style view of your project demands

USAGE
  questlog                                          open the TUI
  questlog quest   <project> <demand> <title> [stage...]  create/update a demand
  questlog stage   <project> <demand> <stage> <todo|active|blocked|done> [percent]
  questlog focus   <project> <demand> <text>        what is happening right now
  questlog block   <project> <demand> <what> [who]  add a blocker
  questlog unblock <project> <demand>               clear all blockers
  questlog done    <project> <demand>               finish every stage
  questlog log     <project> <demand|-> <kind> <message>
  questlog realm   <project> <display name> [order] name/sort a project
  questlog brief                                    open quests, compact (for LLM session start)
  questlog guide                                    instructions for LLM harnesses
  questlog where                                    print the data directory
  questlog demo                                     seed sample realms

Data dir: $QUESTLOG_DIR or ~/.local/tui-live-workprogress
";

pub fn run(args: Vec<String>) -> Result<()> {
    run_in(&paths::data_dir(), &args)
}

/// Same as [`run`] against an explicit data root (tests, scripts).
///
/// ```text
/// run_in(root, &["stage", "api", "US-1", "Tests", "done"])
/// ```
pub fn run_in(root: &Path, args: &[String]) -> Result<()> {
    let rest = args.get(1..).unwrap_or(&[]);
    match args.first().map(String::as_str).unwrap_or("tui") {
        "tui" => tui::run(root.to_path_buf()),
        "quest" => cmd_quest(root, rest),
        "stage" => cmd_stage(root, rest),
        "focus" => cmd_focus(root, rest),
        "block" => cmd_block(root, rest),
        "unblock" => cmd_unblock(root, rest),
        "done" => cmd_done(root, rest),
        "log" => cmd_log(root, rest),
        "realm" => cmd_realm(root, rest),
        "brief" => print_out(&brief::brief_text(&loader::load_world(root), root)),
        "guide" => print_out(&guide::guide_text(root)),
        "where" => print_out(&format!("{}\n", root.display())),
        "demo" => demo::seed(root),
        "-h" | "--help" | "help" => print_out(HELP),
        other => bail!("unknown command {other:?}; run `questlog --help`"),
    }
}

fn print_out(text: &str) -> Result<()> {
    print!("{text}");
    Ok(())
}

fn arg<'a>(rest: &'a [String], index: usize, usage: &str) -> Result<&'a str> {
    rest.get(index)
        .map(String::as_str)
        .ok_or_else(|| anyhow!("missing argument #{}; usage: {usage}", index + 1))
}

fn parse_status(raw: &str) -> Result<Status> {
    Status::parse(raw)
        .ok_or_else(|| anyhow!("invalid status {raw:?}: expected todo|active|blocked|done"))
}

fn parse_percent(raw: &str) -> Result<f64> {
    let value: f64 = raw
        .trim_end_matches('%')
        .parse()
        .with_context(|| format!("invalid percent {raw:?}: expected a number 0-100"))?;
    if !(0.0..=100.0).contains(&value) {
        bail!("invalid percent {raw:?}: expected a number 0-100");
    }
    Ok(value)
}

/// Load (or create), edit, save atomically.
fn edit_demand(
    root: &Path,
    project: &str,
    demand: &str,
    edit: impl FnOnce(&mut Demand),
) -> Result<()> {
    let path = scribe::demand_path(root, project, demand)?;
    let mut current = scribe::load_or_new(&path, demand)?;
    edit(&mut current);
    scribe::save_json(&path, &current)
}

fn chronicle(
    root: &Path,
    project: &str,
    demand: Option<&str>,
    kind: &str,
    msg: &str,
) -> Result<()> {
    activity::append(
        &paths::log_path(root),
        &LogEntry::now(project, demand, kind, msg),
    )
}

fn status_kind(status: Status) -> &'static str {
    match status {
        Status::Done => "done",
        Status::Blocked => "blocked",
        Status::Active => "progress",
        Status::Todo => "note",
    }
}

const USAGE_QUEST: &str = "questlog quest <project> <demand> <title> [stage...]";

fn cmd_quest(root: &Path, rest: &[String]) -> Result<()> {
    let (project, demand) = (arg(rest, 0, USAGE_QUEST)?, arg(rest, 1, USAGE_QUEST)?);
    let title = arg(rest, 2, USAGE_QUEST)?;
    let stages = rest.get(3..).unwrap_or(&[]);
    edit_demand(root, project, demand, |d| {
        d.title = title.to_string();
        scribe::seed_stages(d, stages);
    })?;
    chronicle(
        root,
        project,
        Some(demand),
        "start",
        &format!("Quest accepted: {title}"),
    )
}

const USAGE_STAGE: &str =
    "questlog stage <project> <demand> <stage> <todo|active|blocked|done> [percent]";

fn cmd_stage(root: &Path, rest: &[String]) -> Result<()> {
    let (project, demand) = (arg(rest, 0, USAGE_STAGE)?, arg(rest, 1, USAGE_STAGE)?);
    let name = arg(rest, 2, USAGE_STAGE)?;
    let status = parse_status(arg(rest, 3, USAGE_STAGE)?)?;
    let progress = rest.get(4).map(|p| parse_percent(p)).transpose()?;
    edit_demand(root, project, demand, |d| {
        scribe::upsert_stage(d, name, status, progress)
    })?;
    let pct = progress.map(|p| format!(" {p:.0}%")).unwrap_or_default();
    chronicle(
        root,
        project,
        Some(demand),
        status_kind(status),
        &format!("{name} → {}{pct}", status.label()),
    )
}

const USAGE_FOCUS: &str = "questlog focus <project> <demand> <text>";

fn cmd_focus(root: &Path, rest: &[String]) -> Result<()> {
    let (project, demand) = (arg(rest, 0, USAGE_FOCUS)?, arg(rest, 1, USAGE_FOCUS)?);
    let text = arg(rest, 2, USAGE_FOCUS)?;
    edit_demand(root, project, demand, |d| d.focus = Some(text.to_string()))?;
    chronicle(root, project, Some(demand), "note", text)
}

const USAGE_BLOCK: &str = "questlog block <project> <demand> <what> [who]";

fn cmd_block(root: &Path, rest: &[String]) -> Result<()> {
    let (project, demand) = (arg(rest, 0, USAGE_BLOCK)?, arg(rest, 1, USAGE_BLOCK)?);
    let what = arg(rest, 2, USAGE_BLOCK)?;
    let who = rest.get(3).cloned();
    let msg = match &who {
        Some(owner) => format!("Blocked: {what} (unblocks: {owner})"),
        None => format!("Blocked: {what}"),
    };
    edit_demand(root, project, demand, |d| {
        d.status = None;
        d.blockers.push(Blocker {
            what: what.to_string(),
            who,
        });
    })?;
    chronicle(root, project, Some(demand), "blocked", &msg)
}

const USAGE_PAIR: &str = "questlog <unblock|done> <project> <demand>";

fn cmd_unblock(root: &Path, rest: &[String]) -> Result<()> {
    let (project, demand) = (arg(rest, 0, USAGE_PAIR)?, arg(rest, 1, USAGE_PAIR)?);
    edit_demand(root, project, demand, |d| {
        d.status = None;
        d.blockers.clear();
    })?;
    chronicle(
        root,
        project,
        Some(demand),
        "unblocked",
        "Path cleared, blockers removed",
    )
}

fn cmd_done(root: &Path, rest: &[String]) -> Result<()> {
    let (project, demand) = (arg(rest, 0, USAGE_PAIR)?, arg(rest, 1, USAGE_PAIR)?);
    edit_demand(root, project, demand, scribe::complete)?;
    chronicle(root, project, Some(demand), "done", "Quest complete!")
}

const USAGE_LOG: &str = "questlog log <project> <demand|-> <kind> <message>";

fn cmd_log(root: &Path, rest: &[String]) -> Result<()> {
    let project = scribe::validate_slug(arg(rest, 0, USAGE_LOG)?)?;
    let demand = Some(arg(rest, 1, USAGE_LOG)?).filter(|d| *d != "-");
    let (kind, msg) = (arg(rest, 2, USAGE_LOG)?, arg(rest, 3, USAGE_LOG)?);
    chronicle(root, project, demand, kind, msg)
}

const USAGE_REALM: &str = "questlog realm <project> <display name> [order]";

fn cmd_realm(root: &Path, rest: &[String]) -> Result<()> {
    let project = arg(rest, 0, USAGE_REALM)?;
    let name = arg(rest, 1, USAGE_REALM)?;
    let order = rest
        .get(2)
        .map(|o| {
            o.parse::<i64>()
                .with_context(|| format!("invalid order {o:?}: expected an integer"))
        })
        .transpose()?;
    let meta = ProjectMeta {
        name: Some(name.to_string()),
        order,
    };
    scribe::save_json(&scribe::project_meta_path(root, project)?, &meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch;

    fn call(root: &Path, words: &[&str]) -> Result<()> {
        run_in(
            root,
            &words.iter().map(|w| w.to_string()).collect::<Vec<_>>(),
        )
    }

    #[test]
    fn quest_stage_block_done_flow() {
        let root = scratch::dir("cli-flow");
        call(&root, &["quest", "api", "US-1", "Auth", "Design", "Build"]).unwrap();
        call(&root, &["stage", "api", "US-1", "Design", "done"]).unwrap();
        call(&root, &["stage", "api", "US-1", "Build", "busy", "40%"]).unwrap();
        let world = loader::load_world(&root);
        let demand = &world.projects[0].demands[0];
        assert_eq!((demand.status(), demand.percent()), (Status::Active, 70));
        call(&root, &["block", "api", "US-1", "PO answer", "Ana"]).unwrap();
        assert_eq!(
            loader::load_world(&root).projects[0].demands[0].status(),
            Status::Blocked
        );
        call(&root, &["done", "api", "US-1"]).unwrap();
        let world = loader::load_world(&root);
        assert_eq!(world.projects[0].demands[0].status(), Status::Done);
        assert_eq!(world.log.len(), 5);
    }

    #[test]
    fn focus_unblock_log_and_realm() {
        let root = scratch::dir("cli-misc");
        call(&root, &["block", "web", "W-2", "API"]).unwrap();
        call(&root, &["unblock", "web", "W-2"]).unwrap();
        call(&root, &["focus", "web", "W-2", "Wiring the form"]).unwrap();
        call(&root, &["log", "web", "-", "file", "Spawned app.ts"]).unwrap();
        call(&root, &["realm", "web", "Frontend Realm", "2"]).unwrap();
        let world = loader::load_world(&root);
        let realm = &world.projects[0];
        assert_eq!((realm.name.as_str(), realm.order), ("Frontend Realm", 2));
        assert_eq!(realm.demands[0].focus.as_deref(), Some("Wiring the form"));
        assert!(realm.demands[0].blockers.is_empty());
        assert_eq!(world.log.last().unwrap().demand, None);
    }

    #[test]
    fn bad_input_is_explained() {
        let root = scratch::dir("cli-errors");
        let missing = call(&root, &["stage", "api"]).unwrap_err().to_string();
        assert!(missing.contains("usage"), "{missing}");
        let status = call(&root, &["stage", "api", "d", "s", "maybe"])
            .unwrap_err()
            .to_string();
        assert!(status.contains("\"maybe\""), "{status}");
        assert!(call(&root, &["stage", "api", "d", "s", "active", "150"]).is_err());
        assert!(call(&root, &["quest", "../x", "d", "t"]).is_err());
        assert!(call(&root, &["realm", "p", "n", "first"]).is_err());
        assert!(call(&root, &["fly"]).is_err());
    }

    #[test]
    fn status_kind_maps_each_status() {
        assert_eq!(status_kind(Status::Done), "done");
        assert_eq!(status_kind(Status::Todo), "note");
        assert_eq!(parse_percent("74%").unwrap(), 74.0);
    }
}
