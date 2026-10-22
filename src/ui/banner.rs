//! Header (breadcrumb + global stats) and footer (key hints) shared by all views.

use chrono::{DateTime, Local};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::app::{App, View};
use crate::clock;
use crate::loader::World;
use crate::model::{self, Status};
use crate::ui::theme;

pub fn header(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::bordered()
        .border_type(BorderType::Double)
        .border_style(theme::fg(theme::GOLD));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(breadcrumb(app)), inner);
    frame.render_widget(
        Paragraph::new(stats_line(&app.world, Local::now())).right_aligned(),
        inner,
    );
}

/// `⚔ QUESTLOG  OVERWORLD › Frontend Realm › WEB-7`: where am I, at a glance.
pub fn breadcrumb(app: &App) -> Line<'static> {
    let mut spans = vec![
        Span::styled(" ⚔ QUESTLOG ", theme::bold(theme::GOLD)),
        Span::styled(" OVERWORLD", crumb_style(app.view == View::Overworld)),
    ];
    if let (View::Realm | View::Dungeon, Some(project)) = (app.view, app.project()) {
        spans.push(Span::styled(" › ", theme::fg(theme::FADED)));
        spans.push(Span::styled(
            project.name.to_uppercase(),
            crumb_style(app.view == View::Realm),
        ));
    }
    if let (View::Dungeon, Some(demand)) = (app.view, app.demand()) {
        spans.push(Span::styled(" › ", theme::fg(theme::FADED)));
        spans.push(Span::styled(demand.id.clone(), crumb_style(true)));
    }
    Line::from(spans)
}

fn crumb_style(current: bool) -> ratatui::style::Style {
    if current {
        return theme::bold(theme::PARCHMENT);
    }
    theme::fg(theme::FADED)
}

/// XP, active realms, blocked quests, freshness pulse, clock, broken files.
pub fn stats_line(world: &World, now: DateTime<Local>) -> Line<'static> {
    let xp = model::experience(&world.projects);
    let active = world
        .projects
        .iter()
        .filter(|p| p.status() == Status::Active)
        .count();
    let blocked: usize = world
        .projects
        .iter()
        .map(|p| p.count(Status::Blocked))
        .sum();
    let pulse = world
        .last_activity()
        .map_or("-".to_string(), |t| clock::ago(t, now));
    let mut spans = vec![
        Span::styled(
            format!(
                "★ XP {} · LV {}",
                clock::group_thousands(xp),
                model::level(xp)
            ),
            theme::fg(theme::ARCANE),
        ),
        divider(),
        Span::styled(
            format!("⚑ {active}/{} realms active", world.projects.len()),
            theme::fg(theme::GOLD),
        ),
        divider(),
        Span::styled(
            format!("⊘ {blocked} blocked"),
            theme::fg(blocked_color(blocked)),
        ),
        divider(),
        Span::styled(format!("↻ {pulse}"), theme::fg(theme::FROST)),
        divider(),
        Span::styled(
            format!("{} ", now.format("%H:%M:%S")),
            theme::fg(theme::PARCHMENT),
        ),
    ];
    if !world.problems.is_empty() {
        spans.insert(
            0,
            Span::styled(
                format!("⚠ {} broken file(s)  ", world.problems.len()),
                theme::bold(theme::BLOOD),
            ),
        );
    }
    Line::from(spans)
}

fn blocked_color(blocked: usize) -> ratatui::style::Color {
    if blocked == 0 {
        return theme::FADED;
    }
    theme::BLOOD
}

fn divider() -> Span<'static> {
    Span::styled(" │ ", theme::fg(theme::SHADOW))
}

pub fn footer(frame: &mut Frame, area: Rect, view: View) {
    let mut spans = Vec::new();
    for (key, action) in hints(view) {
        spans.push(Span::styled(format!(" {key} "), theme::bold(theme::GOLD)));
        spans.push(Span::styled(format!("{action}  "), theme::fg(theme::FADED)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

pub fn hints(view: View) -> &'static [(&'static str, &'static str)] {
    match view {
        View::Overworld => &[
            ("←↑↓→", "travel"),
            ("⏎", "enter realm"),
            ("r", "reload"),
            ("q", "quit"),
        ],
        View::Realm => &[
            ("↑↓", "quest"),
            ("←→", "other realm"),
            ("⏎", "enter dungeon"),
            ("esc", "map"),
            ("q", "quit"),
        ],
        View::Dungeon => &[("←→", "stage"), ("esc", "back to realm"), ("q", "quit")],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch;

    fn text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn breadcrumb_follows_zoom() {
        let root = scratch::dir("banner-crumb");
        crate::demo::seed(&root).unwrap();
        let mut app = App::new(root);
        assert!(text(&breadcrumb(&app)).ends_with("OVERWORLD"));
        app.enter();
        app.enter();
        assert!(text(&breadcrumb(&app)).ends_with("BACKEND CITADEL › CORE-1"));
    }

    #[test]
    fn stats_count_realms_and_blocks() {
        let root = scratch::dir("banner-stats");
        crate::demo::seed(&root).unwrap();
        let world = crate::loader::load_world(&root);
        let line = text(&stats_line(&world, Local::now()));
        assert!(line.contains("2/5 realms active"), "{line}");
        assert!(line.contains("⊘ 2 blocked"), "{line}");
        let broken = World {
            problems: vec!["x".into()],
            ..World::default()
        };
        assert!(text(&stats_line(&broken, Local::now())).starts_with("⚠ 1 broken"));
    }

    #[test]
    fn every_view_has_hints() {
        for view in [View::Overworld, View::Realm, View::Dungeon] {
            assert!(hints(view).iter().any(|(k, _)| *k == "q"));
        }
    }
}
