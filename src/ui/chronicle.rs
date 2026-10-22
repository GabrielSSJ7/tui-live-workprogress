//! The activity log panel, optionally narrowed to one realm or one quest.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::activity::{self, LogEntry};
use crate::loader::World;
use crate::ui::{parts, theme};

/// Narrowing for the panel: `None` means "all".
#[derive(Clone, Copy)]
pub struct Scope<'a> {
    pub project: Option<&'a str>,
    pub demand: Option<&'a str>,
}

pub fn draw(frame: &mut Frame, area: Rect, world: &World, scope: Scope, title: &str) {
    let panel = parts::panel(title, theme::ARCANE);
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let mut lines = problem_lines(world, scope);
    let entries = activity::matching(&world.log, scope.project, scope.demand);
    lines.extend(
        entries
            .iter()
            .map(|e| entry_line(e, scope, inner.width as usize)),
    );
    if lines.is_empty() {
        lines.push(Line::styled(
            "  The chronicle is empty… for now.",
            theme::fg(theme::FADED),
        ));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Broken files are shown only on the unscoped log, where they cannot hide.
fn problem_lines(world: &World, scope: Scope) -> Vec<Line<'static>> {
    if scope.project.is_some() {
        return Vec::new();
    }
    world
        .problems
        .iter()
        .map(|p| Line::styled(format!("⚠ {p}"), theme::fg(theme::BLOOD)))
        .collect()
}

/// `[10:55:12] ⚔ frontend-realm · Claude entered...`; the label shows the next
/// level down from the scope (realm on the map, quest inside a realm).
pub fn entry_line(entry: &LogEntry, scope: Scope, width: usize) -> Line<'static> {
    let label = match (scope.project, scope.demand) {
        (None, _) => entry.project.clone(),
        (Some(_), None) => entry.demand.clone(),
        _ => None,
    };
    let color = theme::kind_color(&entry.kind);
    let mut spans = vec![
        Span::styled(
            format!("[{}] ", entry.ts.format("%H:%M:%S")),
            theme::fg(theme::FADED),
        ),
        Span::styled(
            format!("{} ", activity::kind_icon(&entry.kind)),
            theme::bold(color),
        ),
    ];
    let mut used = 13;
    if let Some(label) = label {
        used += label.chars().count() + 3;
        spans.push(Span::styled(format!("{label} · "), theme::fg(theme::FADED)));
    }
    spans.push(Span::styled(
        parts::truncate(&entry.msg, width.saturating_sub(used)),
        theme::fg(color),
    ));
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn label_depends_on_scope() {
        let entry = LogEntry::now("web", Some("W-7"), "done", "Shipped");
        let all = Scope {
            project: None,
            demand: None,
        };
        let realm = Scope {
            project: Some("web"),
            demand: None,
        };
        let quest = Scope {
            project: Some("web"),
            demand: Some("W-7"),
        };
        assert!(text(&entry_line(&entry, all, 80)).ends_with("✓ web · Shipped"));
        assert!(text(&entry_line(&entry, realm, 80)).ends_with("✓ W-7 · Shipped"));
        assert!(text(&entry_line(&entry, quest, 80)).ends_with("✓ Shipped"));
        assert!(text(&entry_line(&entry, all, 20)).ends_with("…"));
    }

    #[test]
    fn problems_only_on_unscoped_log() {
        let world = World {
            problems: vec!["bad.json".into()],
            ..World::default()
        };
        assert_eq!(
            problem_lines(
                &world,
                Scope {
                    project: None,
                    demand: None
                }
            )
            .len(),
            1
        );
        assert!(problem_lines(
            &world,
            Scope {
                project: Some("p"),
                demand: None
            }
        )
        .is_empty());
    }
}
