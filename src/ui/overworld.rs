//! Macro view: every realm as an animated tile on a serpentine road, a quest
//! board with one bar per realm, and the global activity log.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::map_grid;
use crate::model::{Project, Status};
use crate::ui::chronicle::{self, Scope};
use crate::ui::pixels::PixelScene;
use crate::ui::scene::{self, EMBLEM_ROWS, EMBLEM_W};
use crate::ui::{parts, theme};

pub const TILE_W: u16 = 26;
/// Border 2 + art 6 + status line + bar line.
pub const TILE_H: u16 = 10;
const GAP_X: u16 = 4;
const GAP_Y: u16 = 1;

/// Columns of tiles that fit a terminal `width` (map panel spans it fully).
pub fn map_columns(width: u16) -> usize {
    columns_for(width.saturating_sub(2))
}

fn columns_for(inner_width: u16) -> usize {
    ((inner_width + GAP_X) / (TILE_W + GAP_X)).max(1) as usize
}

/// Panel height that shows every realm (plus a blank row above and below);
/// whatever is left goes to the board and the log.
fn map_height(width: u16, len: usize) -> u16 {
    let rows = len.div_ceil(map_columns(width)).max(1) as u16;
    rows * (TILE_H + GAP_Y) - GAP_Y + 4
}

fn rows_for(inner_height: u16) -> usize {
    ((inner_height + GAP_Y) / (TILE_H + GAP_Y)).max(1) as usize
}

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    if app.world.projects.is_empty() {
        return draw_welcome(frame, area, app);
    }
    let map_h = map_height(area.width, app.world.projects.len()).min(area.height.saturating_sub(7));
    let [map, bottom] =
        Layout::vertical([Constraint::Length(map_h), Constraint::Min(5)]).areas(area);
    let [board, log] = Layout::horizontal([Constraint::Percentage(50); 2]).areas(bottom);
    draw_map(frame, map, app);
    draw_board(frame, board, app);
    chronicle::draw(
        frame,
        log,
        &app.world,
        Scope {
            project: None,
            demand: None,
        },
        "ACTIVITY LOG",
    );
}

/// Which slice of realms is on screen, and how it is laid out.
struct MapPage {
    first: usize,
    last: usize,
    cols: usize,
    size: usize,
    origin: (u16, u16),
}

impl MapPage {
    fn new(inner: Rect, selected: usize, len: usize) -> MapPage {
        let cols = columns_for(inner.width);
        let size = cols * rows_for(inner.height);
        let first = selected / size * size;
        let last = (first + size).min(len);
        let used_cols = cols.min(last - first) as u16;
        let used_rows = (last - first).div_ceil(cols) as u16;
        let used_w = used_cols * TILE_W + used_cols.saturating_sub(1) * GAP_X;
        let used_h = used_rows * TILE_H + used_rows.saturating_sub(1) * GAP_Y;
        let origin = (
            inner.x + inner.width.saturating_sub(used_w) / 2,
            inner.y + inner.height.saturating_sub(used_h) / 2,
        );
        MapPage {
            first,
            last,
            cols,
            size,
            origin,
        }
    }

    /// Absolute serpentine cell, shifted so the page's first row is row 0.
    fn tile_rect(&self, index: usize) -> Rect {
        let (row, col) = map_grid::cell_of(index, self.cols);
        let row = (row - self.first / self.cols) as u16;
        let x = self.origin.0 + col as u16 * (TILE_W + GAP_X);
        let y = self.origin.1 + row * (TILE_H + GAP_Y);
        Rect::new(x, y, TILE_W, TILE_H)
    }

    fn label(&self, len: usize) -> String {
        let pages = len.div_ceil(self.size);
        if pages <= 1 {
            return "OVERWORLD MAP".to_string();
        }
        format!(
            "OVERWORLD MAP · page {}/{pages}",
            self.first / self.size + 1
        )
    }
}

fn draw_map(frame: &mut Frame, area: Rect, app: &App) {
    let len = app.world.projects.len();
    let probe = Block::bordered().inner(area);
    let page = MapPage::new(probe, app.project_idx, len);
    let panel = parts::panel(&page.label(len), theme::FROST);
    frame.render_widget(panel, area);
    for index in page.first..page.last.saturating_sub(1) {
        let lit = app.world.projects[index].status() == Status::Done;
        draw_road(frame, page.tile_rect(index), page.tile_rect(index + 1), lit);
    }
    for index in page.first..page.last {
        let rect = page.tile_rect(index).intersection(probe);
        draw_tile(
            frame,
            rect,
            &app.world.projects[index],
            index == app.project_idx,
            app.frame,
        );
    }
}

/// Dotted path between consecutive realms; gold once the earlier realm is done.
fn draw_road(frame: &mut Frame, from: Rect, to: Rect, lit: bool) {
    let style = theme::fg(if lit { theme::GOLD } else { theme::SHADOW });
    let area = frame.area();
    if from.y == to.y {
        let (left, right) = if from.x < to.x {
            (from, to)
        } else {
            (to, from)
        };
        let road = Rect::new(left.right(), left.y + TILE_H / 2, right.x - left.right(), 1)
            .intersection(area);
        frame.render_widget(
            Paragraph::new(Span::styled("·".repeat(road.width as usize), style)),
            road,
        );
        return;
    }
    let road = Rect::new(
        from.x + TILE_W / 2,
        from.bottom(),
        1,
        to.y.saturating_sub(from.bottom()),
    )
    .intersection(area);
    let lines: Vec<Line> = (0..road.height).map(|_| Line::styled("┊", style)).collect();
    frame.render_widget(Paragraph::new(lines), road);
}

fn draw_tile(frame: &mut Frame, rect: Rect, project: &Project, selected: bool, anim: u64) {
    let status = project.status();
    let accent = if selected {
        theme::GOLD
    } else {
        theme::status_color(status)
    };
    let border = if selected {
        BorderType::Thick
    } else {
        BorderType::Rounded
    };
    let name = parts::truncate(&project.name.to_uppercase(), TILE_W as usize - 4);
    let block = Block::bordered()
        .border_type(border)
        .border_style(theme::fg(accent))
        .title(Line::from(Span::styled(format!(" {name} "), theme::bold(accent))).centered());
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    let [art, state, bar] = Layout::vertical([
        Constraint::Length(EMBLEM_ROWS),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);
    frame.render_widget(
        PixelScene::new(scene::emblem(status, anim)),
        parts::centered(art, EMBLEM_W, EMBLEM_ROWS),
    );
    frame.render_widget(Paragraph::new(state_line(project)).centered(), state);
    let bar_spans = parts::hp_bar(
        project.percent(),
        bar.width.saturating_sub(2),
        theme::status_color(status),
    );
    frame.render_widget(Paragraph::new(Line::from(bar_spans)).centered(), bar);
}

/// `⚔ ACTIVE 45% ⊘1`: status word, percent, and hidden blocks inside the realm.
pub fn state_line(project: &Project) -> Line<'static> {
    let status = project.status();
    let color = theme::status_color(status);
    let mut text = format!("{} {}", status.icon(), status.headline());
    if status != Status::Todo {
        text.push_str(&format!(" {}%", project.percent()));
    }
    let mut spans = vec![Span::styled(text, theme::bold(color))];
    let blocked = project.count(Status::Blocked);
    if blocked > 0 && status != Status::Blocked {
        spans.push(Span::styled(
            format!(" ⊘{blocked}"),
            theme::bold(theme::BLOOD),
        ));
    }
    Line::from(spans)
}

fn draw_board(frame: &mut Frame, area: Rect, app: &App) {
    let panel = parts::panel("QUEST BOARD", theme::GOLD);
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let start = parts::scroll_start(app.project_idx, inner.height as usize);
    let lines: Vec<Line> = app
        .world
        .projects
        .iter()
        .enumerate()
        .skip(start)
        .take(inner.height as usize)
        .map(|(i, p)| board_line(p, i == app.project_idx, inner.width))
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

const BOARD_NAME_W: usize = 18;
/// Cursor 2, icon 1, " name" , " ", " 100% " 6, four " ✓9" counters 12.
const BOARD_FIXED_W: u16 = 2 + 1 + 1 + BOARD_NAME_W as u16 + 1 + 6 + 12;

/// `▶ ⚔ Frontend Realm   ████░░░░  45%  ✓0 ⚔1 ⊘1 ◇1`
pub fn board_line(project: &Project, selected: bool, width: u16) -> Line<'static> {
    let status = project.status();
    let mut spans = vec![
        parts::cursor(selected),
        parts::icon(status),
        Span::styled(
            format!(" {}", parts::fit(&project.name, BOARD_NAME_W)),
            theme::fg(theme::PARCHMENT),
        ),
        Span::raw(" "),
    ];
    spans.extend(parts::hp_bar(
        project.percent(),
        width.saturating_sub(BOARD_FIXED_W).max(4),
        theme::status_color(status),
    ));
    spans.push(Span::styled(
        format!(" {:>3}% ", project.percent()),
        theme::bold(theme::status_color(status)),
    ));
    spans.extend(counters(project));
    Line::from(spans).style(parts::row_style(selected))
}

/// `✓2 ⚔1 ⊘0 ◇3`, zero counts dimmed so the eye lands on what exists.
fn counters(project: &Project) -> Vec<Span<'static>> {
    [Status::Done, Status::Active, Status::Blocked, Status::Todo]
        .into_iter()
        .map(|s| {
            let n = project.count(s);
            let color = if n == 0 {
                theme::SHADOW
            } else {
                theme::status_color(s)
            };
            Span::styled(format!(" {}{n}", s.icon()), theme::fg(color))
        })
        .collect()
}

fn draw_welcome(frame: &mut Frame, area: Rect, app: &App) {
    let panel = parts::panel("WELCOME, ADVENTURER", theme::GOLD);
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let [art, text] =
        Layout::vertical([Constraint::Length(EMBLEM_ROWS + 1), Constraint::Min(0)]).areas(inner);
    frame.render_widget(
        PixelScene::new(scene::emblem(Status::Todo, 0)),
        parts::centered(art, EMBLEM_W, EMBLEM_ROWS),
    );
    let root = app.root.display().to_string();
    let lines = vec![
        Line::styled(
            format!("No realms yet in {root}"),
            theme::bold(theme::PARCHMENT),
        ),
        Line::raw(""),
        Line::styled(
            "questlog demo           seed sample realms",
            theme::fg(theme::GOLD),
        ),
        Line::styled(
            "questlog guide          instructions for your LLM harness",
            theme::fg(theme::GOLD),
        ),
        Line::styled(
            "questlog quest <project> <demand> <title> [stages...]",
            theme::fg(theme::FROST),
        ),
    ];
    frame.render_widget(Paragraph::new(lines).centered(), text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Demand;

    fn text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn realm(statuses: &[Status]) -> Project {
        let demands = statuses
            .iter()
            .enumerate()
            .map(|(i, s)| Demand {
                status: Some(*s),
                ..Demand::new(&format!("d{i}"))
            })
            .collect();
        Project {
            slug: "r".into(),
            name: "Frontend Realm".into(),
            order: 0,
            demands,
        }
    }

    #[test]
    fn columns_and_rows_fit_tiles() {
        assert_eq!(map_columns(10), 1);
        assert_eq!(map_columns(2 + TILE_W * 2 + GAP_X), 2);
        assert_eq!(rows_for(TILE_H * 2 + GAP_Y), 2);
        assert_eq!(rows_for(3), 1);
        assert_eq!(map_height(10, 3), 3 * (TILE_H + GAP_Y) - GAP_Y + 4);
        assert_eq!(map_height(500, 3), TILE_H + 4);
    }

    #[test]
    fn state_line_flags_hidden_blocks() {
        let line = text(&state_line(&realm(&[Status::Active, Status::Blocked])));
        assert_eq!(line, "⚔ ACTIVE 0% ⊘1");
        assert_eq!(text(&state_line(&realm(&[]))), "◇ NOT STARTED");
        assert_eq!(
            text(&state_line(&realm(&[Status::Blocked]))),
            "⊘ BLOCKED 0%"
        );
    }

    #[test]
    fn board_line_fills_width() {
        let line = board_line(&realm(&[Status::Done, Status::Todo]), true, 80);
        let rendered = text(&line);
        assert!(rendered.starts_with("▶ ⚔ Frontend Realm"), "{rendered}");
        assert_eq!(rendered.chars().count(), 80);
        assert!(rendered.contains(" ✓1 ⚔0 ⊘0 ◇1"));
    }

    #[test]
    fn page_places_serpentine_tiles() {
        let inner = Rect::new(0, 0, TILE_W * 2 + GAP_X, TILE_H * 2 + GAP_Y);
        let page = MapPage::new(inner, 0, 3);
        assert_eq!((page.first, page.last, page.cols), (0, 3, 2));
        assert_eq!(page.tile_rect(2).x, page.tile_rect(1).x);
        assert_eq!(page.tile_rect(2).y, TILE_H + GAP_Y);
        let second = MapPage::new(inner, 5, 6);
        assert_eq!((second.first, second.last), (4, 6));
        // Page 2 holds one row (indices 4, 5), drawn as the page's first row.
        assert_eq!(second.tile_rect(4).y, second.origin.1);
        assert_eq!(second.tile_rect(5).y, second.origin.1);
        assert_eq!(second.label(6), "OVERWORLD MAP · page 2/2");
        assert_eq!(page.label(3), "OVERWORLD MAP");
    }
}
