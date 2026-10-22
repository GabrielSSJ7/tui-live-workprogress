//! Demand (micro) view: stages as rooms along a dungeon corridor, the stage
//! list with notes, the battle and party sheet, and this quest's log.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::model::{Demand, Stage, Status};
use crate::ui::chronicle::{self, Scope};
use crate::ui::{front, parts, theme};

const ROOM_W: u16 = 16;
const ROOM_H: u16 = 5;
const LINK_W: u16 = 3;
/// Marker row + rooms + overflow-hint row.
const CORRIDOR_INNER_H: u16 = ROOM_H + 2;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let (Some(project), Some(demand)) = (app.project(), app.demand()) else {
        return;
    };
    let rows = [
        Constraint::Length(CORRIDOR_INNER_H + 2),
        Constraint::Min(8),
        Constraint::Length(8),
    ];
    let [corridor, middle, log] = Layout::vertical(rows).areas(area);
    draw_corridor(frame, corridor, demand, app.stage_idx);
    let [stages, side] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(middle);
    draw_stage_list(frame, stages, demand, app.stage_idx);
    let [battle, party] = Layout::vertical([Constraint::Length(9), Constraint::Min(5)]).areas(side);
    front::draw_front(frame, battle, Some(demand), app.frame);
    front::draw_party(frame, party, Some(demand), &app.world, &project.slug);
    let scope = Scope {
        project: Some(&project.slug),
        demand: Some(&demand.id),
    };
    chronicle::draw(frame, log, &app.world, scope, "QUEST LOG");
}

/// Rooms that fit, as a window `[start, end)` that keeps `selected` visible.
pub fn room_window(width: u16, selected: usize, len: usize) -> (usize, usize) {
    let fit = ((width + LINK_W) / (ROOM_W + LINK_W)).max(1) as usize;
    let start = parts::scroll_start(selected, fit);
    (start, (start + fit).min(len))
}

fn draw_corridor(frame: &mut Frame, area: Rect, demand: &Demand, selected: usize) {
    let title = format!("DUNGEON · {} {}", demand.id, demand.display_title());
    let panel = parts::panel(&title, theme::status_color(demand.status()));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    if demand.stages.is_empty() {
        let hint = "No stages yet: questlog stage <project> <demand> <stage> active";
        frame.render_widget(
            Paragraph::new(Line::styled(hint, theme::fg(theme::FADED))).centered(),
            inner,
        );
        return;
    }
    let (start, end) = room_window(inner.width, selected, demand.stages.len());
    let count = (end - start) as u16;
    let used = count * ROOM_W + count.saturating_sub(1) * LINK_W;
    let x0 = inner.x + inner.width.saturating_sub(used) / 2;
    let current = demand.current_stage();
    for (slot, index) in (start..end).enumerate() {
        let x = x0 + slot as u16 * (ROOM_W + LINK_W);
        let room = Rect::new(x, inner.y + 1, ROOM_W, ROOM_H).intersection(inner);
        draw_room(frame, room, &demand.stages[index], index == selected);
        if current == Some(index) {
            let marker = Rect::new(x, inner.y, ROOM_W, 1).intersection(inner);
            frame.render_widget(
                Paragraph::new(Line::styled("▼ HERO HERE", theme::bold(theme::GOLD))).centered(),
                marker,
            );
        }
        if index + 1 < end {
            draw_link(
                frame,
                Rect::new(x + ROOM_W, inner.y + 1 + ROOM_H / 2, LINK_W, 1).intersection(inner),
                &demand.stages[index],
            );
        }
    }
    draw_overflow(
        frame,
        Rect::new(inner.x, inner.y + ROOM_H + 1, inner.width, 1).intersection(inner),
        start,
        demand.stages.len() - end,
    );
}

/// Corridor segment: solid gold once the room before it is cleared.
fn draw_link(frame: &mut Frame, area: Rect, before: &Stage) {
    let (glyph, color) = match before.status {
        Status::Done => ("━", theme::GOLD),
        _ => ("┄", theme::SHADOW),
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            glyph.repeat(area.width as usize),
            theme::fg(color),
        )),
        area,
    );
}

fn draw_overflow(frame: &mut Frame, area: Rect, hidden_left: usize, hidden_right: usize) {
    let style = theme::fg(theme::FADED);
    if hidden_left > 0 {
        frame.render_widget(
            Paragraph::new(Span::styled(format!("‹ {hidden_left} more"), style)),
            area,
        );
    }
    if hidden_right > 0 {
        frame.render_widget(
            Paragraph::new(Span::styled(format!("{hidden_right} more ›"), style)).right_aligned(),
            area,
        );
    }
}

fn draw_room(frame: &mut Frame, area: Rect, stage: &Stage, selected: bool) {
    let color = theme::status_color(stage.status);
    let (border, accent) = if selected {
        (BorderType::Thick, theme::GOLD)
    } else {
        (BorderType::Rounded, color)
    };
    let block = Block::bordered()
        .border_type(border)
        .border_style(theme::fg(accent));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let name = parts::truncate(&stage.name, inner.width as usize);
    let lines = vec![
        Line::styled(room_label(stage), theme::bold(color)),
        Line::styled(name, theme::bold(theme::PARCHMENT)),
        Line::from(parts::hp_bar(
            stage.percent(),
            inner.width.saturating_sub(2),
            color,
        )),
    ];
    frame.render_widget(Paragraph::new(lines).centered(), inner);
}

/// `✓ CLEARED`, `⚔ 45%`, `⊘ BLOCKED`, `◇ DARK`.
pub fn room_label(stage: &Stage) -> String {
    match stage.status {
        Status::Done => "✓ CLEARED".to_string(),
        Status::Active => format!("⚔ {}%", stage.percent()),
        Status::Blocked => "⊘ BLOCKED".to_string(),
        Status::Todo => "◇ DARK".to_string(),
    }
}

fn draw_stage_list(frame: &mut Frame, area: Rect, demand: &Demand, selected: usize) {
    let title = format!("STAGES {}/{}", demand.done_stages(), demand.stages.len());
    let panel = parts::panel(&title, theme::GOLD);
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let lines: Vec<Line> = demand
        .stages
        .iter()
        .enumerate()
        .flat_map(|(i, s)| stage_lines(s, i == selected, inner.width))
        .collect();
    let start = parts::scroll_start(line_of_stage(demand, selected), inner.height as usize);
    frame.render_widget(Paragraph::new(lines).scroll((start as u16, 0)), inner);
}

/// First rendered line of stage `index` (stages with a note take two lines).
fn line_of_stage(demand: &Demand, index: usize) -> usize {
    demand
        .stages
        .iter()
        .take(index)
        .map(|s| 1 + usize::from(s.note.is_some()))
        .sum()
}

const STAGE_NAME_W: usize = 20;

/// `▶ ⚔ Build                ██████░░░░  45%` plus an indented note line.
pub fn stage_lines(stage: &Stage, selected: bool, width: u16) -> Vec<Line<'static>> {
    let color = theme::status_color(stage.status);
    let mut head = vec![
        parts::cursor(selected),
        parts::icon(stage.status),
        Span::styled(
            format!(" {} ", parts::fit(&stage.name, STAGE_NAME_W)),
            theme::fg(theme::PARCHMENT),
        ),
    ];
    let fixed = 2 + 1 + STAGE_NAME_W as u16 + 2 + 5;
    head.extend(parts::hp_bar(
        stage.percent(),
        width.saturating_sub(fixed).max(4),
        color,
    ));
    head.push(Span::styled(
        format!(" {:>3}%", stage.percent()),
        theme::bold(color),
    ));
    let mut lines = vec![Line::from(head).style(parts::row_style(selected))];
    if let Some(note) = &stage.note {
        lines.push(Line::styled(
            format!("     ↳ {note}"),
            theme::fg(theme::FADED),
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn room_window_scrolls_with_selection() {
        let wide = ROOM_W * 3 + LINK_W * 2;
        assert_eq!(room_window(wide, 0, 5), (0, 3));
        assert_eq!(room_window(wide, 4, 5), (2, 5));
        assert_eq!(room_window(1, 2, 5), (2, 3));
    }

    #[test]
    fn room_labels_say_state_plainly() {
        let active = Stage {
            status: Status::Active,
            progress: Some(45.0),
            ..Stage::new("b")
        };
        assert_eq!(room_label(&active), "⚔ 45%");
        assert_eq!(room_label(&Stage::new("x")), "◇ DARK");
    }

    #[test]
    fn stage_lines_include_note_and_fill_width() {
        let stage = Stage {
            note: Some("left: middleware".into()),
            ..Stage::new("Build")
        };
        let lines = stage_lines(&stage, false, 60);
        assert_eq!(lines.len(), 2);
        assert_eq!(text(&lines[0]).chars().count(), 60);
        assert!(text(&lines[1]).ends_with("↳ left: middleware"));
    }

    #[test]
    fn line_of_stage_counts_note_lines() {
        let demand = Demand {
            stages: vec![
                Stage {
                    note: Some("n".into()),
                    ..Stage::new("a")
                },
                Stage::new("b"),
                Stage::new("c"),
            ],
            ..Demand::new("d")
        };
        assert_eq!(line_of_stage(&demand, 2), 3);
    }
}
