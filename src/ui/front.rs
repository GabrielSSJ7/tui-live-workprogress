//! Panels about one demand, shared by the realm and dungeon views: the animated
//! battle ("the front") and the party sheet (focus, progress, blockers).

use chrono::{DateTime, Local};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::activity;
use crate::clock;
use crate::loader::World;
use crate::model::{Demand, Status};
use crate::ui::pixels::PixelScene;
use crate::ui::{parts, scene, theme};

pub fn draw_front(frame: &mut Frame, area: Rect, demand: Option<&Demand>, anim: u64) {
    let status = demand.map_or(Status::Todo, Demand::status);
    let title = demand.map_or("THE FRONT".to_string(), |d| format!("THE FRONT · {}", d.id));
    let panel = parts::panel(&title, theme::status_color(status));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let parts = scene::battle(status, anim, inner.width as i16, inner.height as i16 * 2);
    frame.render_widget(PixelScene::new(parts).with_ground(theme::TURF), inner);
}

pub fn draw_party(
    frame: &mut Frame,
    area: Rect,
    demand: Option<&Demand>,
    world: &World,
    project: &str,
) {
    let panel = parts::panel("PARTY", theme::FROST);
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let Some(demand) = demand else {
        frame.render_widget(
            Paragraph::new(Line::styled("No quest selected.", theme::fg(theme::FADED))),
            inner,
        );
        return;
    };
    let latest = activity::matching(&world.log, Some(project), Some(&demand.id))
        .first()
        .map(|e| e.msg.clone());
    let lines = party_lines(demand, latest.as_deref(), Local::now(), inner.width);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// What the hero says: the declared focus, else the last log line, else a stock line.
pub fn speech(demand: &Demand, latest: Option<&str>) -> String {
    if let Some(focus) = demand.focus.as_deref().filter(|f| !f.trim().is_empty()) {
        return focus.to_string();
    }
    if let Some(msg) = latest {
        return msg.to_string();
    }
    match demand.status() {
        Status::Done => "Victory! The quest is complete.",
        Status::Active => "Onward!",
        Status::Blocked => "The path is blocked…",
        Status::Todo => "Awaiting orders.",
    }
    .to_string()
}

fn hero_name(demand: &Demand) -> String {
    crate::loader::prettify_slug(demand.owner.as_deref().unwrap_or("hero"))
}

pub fn party_lines(
    demand: &Demand,
    latest: Option<&str>,
    now: DateTime<Local>,
    width: u16,
) -> Vec<Line<'static>> {
    let status = demand.status();
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{}: ", hero_name(demand)),
                theme::bold(theme::FROST),
            ),
            Span::styled(
                format!("\"{}\"", speech(demand, latest)),
                theme::fg(theme::PARCHMENT),
            ),
        ]),
        progress_line(demand.percent(), status, width),
        stage_strip(demand),
    ];
    lines.extend(demand.blockers.iter().map(|b| {
        let who = b
            .who
            .as_deref()
            .map(|w| format!(" → {w}"))
            .unwrap_or_default();
        Line::styled(format!("⊘ {}{who}", b.what), theme::bold(theme::BLOOD))
    }));
    if !demand.waiting.is_empty() {
        lines.push(Line::styled(
            format!("⇄ waiting on this: {}", demand.waiting.join(", ")),
            theme::fg(theme::ARCANE),
        ));
    }
    let updated = demand
        .updated
        .map_or("never".to_string(), |t| clock::ago_phrase(t, now));
    lines.push(Line::styled(
        format!("↻ updated {updated}"),
        theme::fg(theme::FADED),
    ));
    lines
}

/// `PROGRESS ██████░░░░ 45%`
fn progress_line(percent: u8, status: Status, width: u16) -> Line<'static> {
    let mut spans = vec![Span::styled("PROGRESS ", theme::fg(theme::FADED))];
    spans.extend(parts::hp_bar(
        percent,
        width.saturating_sub(15).max(4),
        theme::status_color(status),
    ));
    spans.push(Span::styled(
        format!(" {percent:>3}%"),
        theme::bold(theme::status_color(status)),
    ));
    Line::from(spans)
}

/// `STAGES   ✓✓⚔◇  2/4`: one colored glyph per stage, readable in a blink.
fn stage_strip(demand: &Demand) -> Line<'static> {
    let mut spans = vec![Span::styled("STAGES   ", theme::fg(theme::FADED))];
    spans.extend(demand.stages.iter().map(|s| parts::icon(s.status)));
    let tally = format!("  {}/{}", demand.done_stages(), demand.stages.len());
    spans.push(Span::styled(tally, theme::fg(theme::PARCHMENT)));
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Blocker, Stage};

    fn text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn speech_prefers_focus_then_log_then_stock() {
        let mut demand = Demand::new("d");
        assert_eq!(speech(&demand, None), "Awaiting orders.");
        assert_eq!(speech(&demand, Some("log")), "log");
        demand.focus = Some("focus".into());
        assert_eq!(speech(&demand, Some("log")), "focus");
    }

    #[test]
    fn party_lines_show_blockers_and_strip() {
        let demand = Demand {
            owner: Some("claude".into()),
            stages: vec![
                Stage {
                    status: Status::Done,
                    ..Stage::new("a")
                },
                Stage::new("b"),
            ],
            blockers: vec![Blocker {
                what: "PO".into(),
                who: Some("Ana".into()),
            }],
            waiting: vec!["web/W-1".into()],
            ..Demand::new("d")
        };
        let lines: Vec<String> = party_lines(&demand, None, Local::now(), 40)
            .iter()
            .map(text)
            .collect();
        assert!(lines[0].starts_with("Claude: \""), "{lines:?}");
        assert!(lines[1].ends_with(" 50%"));
        assert_eq!(lines[2], "STAGES   ✓◇  1/2");
        assert_eq!(lines[3], "⊘ PO → Ana");
        assert!(lines[4].contains("web/W-1"));
        assert_eq!(lines[5], "↻ updated never");
    }
}
