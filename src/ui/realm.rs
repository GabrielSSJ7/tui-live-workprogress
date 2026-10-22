//! Project view: the realm's quest pipeline on the left; on the right the
//! selected quest's battle, party sheet, and the realm's combat log.

use chrono::{DateTime, Local};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::clock;
use crate::model::{Demand, Project};
use crate::ui::chronicle::{self, Scope};
use crate::ui::{front, parts, theme};

/// Three content lines plus a spacer per quest.
const ITEM_H: usize = 4;
/// Width of `▶ [ACTIVE ] ⚔ ` so bars and meta align under the title.
const INDENT: usize = 14;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let Some(project) = app.project() else {
        return;
    };
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(area);
    draw_pipeline(frame, left, project, app.demand_idx);
    let [battle, party, log] = Layout::vertical([
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Min(3),
    ])
    .areas(right);
    front::draw_front(frame, battle, app.demand(), app.frame);
    front::draw_party(frame, party, app.demand(), &app.world, &project.slug);
    let scope = Scope {
        project: Some(&project.slug),
        demand: None,
    };
    chronicle::draw(frame, log, &app.world, scope, "COMBAT LOG");
}

fn draw_pipeline(frame: &mut Frame, area: Rect, project: &Project, selected: usize) {
    let title = format!(
        "QUEST PIPELINE · {} {}%",
        project.name.to_uppercase(),
        project.percent()
    );
    let panel = parts::panel(&title, theme::status_color(project.status()));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    if project.demands.is_empty() {
        let hint = "No quests yet: questlog quest <project> <demand> <title> [stages...]";
        frame.render_widget(
            Paragraph::new(Line::styled(hint, theme::fg(theme::FADED))),
            inner,
        );
        return;
    }
    let visible = (inner.height as usize / ITEM_H).max(1);
    let start = parts::scroll_start(selected, visible);
    let now = Local::now();
    let lines: Vec<Line> = project
        .demands
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .flat_map(|(i, d)| pipeline_item(d, i == selected, inner.width, now))
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

/// One quest: tag + title, progress bar, and where it stands (or what blocks it).
pub fn pipeline_item(
    demand: &Demand,
    selected: bool,
    width: u16,
    now: DateTime<Local>,
) -> Vec<Line<'static>> {
    let status = demand.status();
    let color = theme::status_color(status);
    let title_room = (width as usize).saturating_sub(INDENT + demand.id.chars().count() + 2);
    let head = Line::from(vec![
        parts::cursor(selected),
        parts::status_tag(status),
        Span::raw(" "),
        parts::icon(status),
        Span::raw(" "),
        Span::styled(demand.id.clone(), theme::bold(color)),
        Span::styled(
            format!("  {}", parts::truncate(demand.display_title(), title_room)),
            theme::bold(theme::PARCHMENT),
        ),
    ]);
    let mut bar = vec![Span::raw(" ".repeat(INDENT))];
    bar.extend(parts::hp_bar(
        demand.percent(),
        width.saturating_sub(INDENT as u16 + 5).max(4),
        color,
    ));
    bar.push(Span::styled(
        format!(" {:>3}%", demand.percent()),
        theme::bold(color),
    ));
    let style = parts::row_style(selected);
    let meta = parts::pad_to(meta_line(demand, now), width);
    vec![
        parts::pad_to(head, width).style(style),
        Line::from(bar).style(style),
        meta.style(style),
        Line::raw(""),
    ]
}

/// `stage 2/4 · Build · 6m ago`, or the first blocker in red.
pub fn meta_line(demand: &Demand, now: DateTime<Local>) -> Line<'static> {
    let pad = Span::raw(" ".repeat(INDENT));
    if let Some(blocker) = demand.blockers.first() {
        let who = blocker
            .who
            .as_deref()
            .map(|w| format!(" → {w}"))
            .unwrap_or_default();
        return Line::from(vec![
            pad,
            Span::styled(format!("⊘ {}{who}", blocker.what), theme::fg(theme::BLOOD)),
        ]);
    }
    let mut parts_text = Vec::new();
    if let Some(index) = demand.current_stage() {
        parts_text.push(format!(
            "stage {}/{} · {}",
            index + 1,
            demand.stages.len(),
            demand.stages[index].name
        ));
    } else if !demand.stages.is_empty() {
        parts_text.push(format!("all {} stages cleared", demand.stages.len()));
    }
    if let Some(updated) = demand.updated {
        parts_text.push(clock::ago_phrase(updated, now));
    }
    Line::from(vec![
        pad,
        Span::styled(parts_text.join(" · "), theme::fg(theme::FADED)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Blocker, Stage, Status};

    fn text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn staged() -> Demand {
        Demand {
            title: "Login screen".into(),
            stages: vec![
                Stage {
                    status: Status::Done,
                    ..Stage::new("Design")
                },
                Stage::new("Build"),
            ],
            ..Demand::new("WEB-7")
        }
    }

    #[test]
    fn pipeline_item_has_tag_bar_meta_spacer() {
        let lines: Vec<String> = pipeline_item(&staged(), true, 60, Local::now())
            .iter()
            .map(text)
            .collect();
        assert_eq!(lines.len(), ITEM_H);
        assert_eq!(lines[0].trim_end(), "▶ [ACTIVE ] ⚔ WEB-7  Login screen");
        assert_eq!(lines[0].chars().count(), 60);
        assert!(lines[1].ends_with("  50%"));
        assert_eq!(lines[1].chars().count(), 60);
        assert_eq!(lines[2].trim(), "stage 2/2 · Build");
    }

    #[test]
    fn meta_line_prefers_blocker_then_progress() {
        let mut demand = staged();
        demand.blockers.push(Blocker {
            what: "PO".into(),
            who: Some("Ana".into()),
        });
        assert_eq!(text(&meta_line(&demand, Local::now())).trim(), "⊘ PO → Ana");
        let done = Demand {
            stages: vec![Stage {
                status: Status::Done,
                ..Stage::new("a")
            }],
            ..Demand::new("x")
        };
        assert_eq!(
            text(&meta_line(&done, Local::now())).trim(),
            "all 1 stages cleared"
        );
        let mut fresh = Demand::new("y");
        fresh.updated = Some(Local::now());
        assert_eq!(text(&meta_line(&fresh, Local::now())).trim(), "just now");
    }
}
