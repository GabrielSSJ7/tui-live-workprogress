//! Rendering. `draw` paints the backdrop, header and footer, then the open view.

pub mod banner;
pub mod chronicle;
pub mod dungeon;
pub mod front;
pub mod overworld;
pub mod parts;
pub mod pixels;
pub mod realm;
pub mod scene;
pub mod sprites;
pub mod theme;

use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::Frame;

use crate::app::{App, View};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame
        .buffer_mut()
        .set_style(area, Style::new().bg(theme::BACKDROP).fg(theme::PARCHMENT));
    let [head, body, foot] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    banner::header(frame, head, app);
    match app.view {
        View::Overworld => overworld::draw(frame, body, app),
        View::Realm => realm::draw(frame, body, app),
        View::Dungeon => dungeon::draw(frame, body, app),
    }
    banner::footer(frame, foot, app.view);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn screen(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        buffer.content().iter().map(|c| c.symbol()).collect()
    }

    fn demo_app(tag: &str) -> App {
        let root = scratch::dir(tag);
        crate::demo::seed(&root).unwrap();
        let mut app = App::new(root);
        app.map_cols = overworld::map_columns(160);
        app
    }

    #[test]
    fn every_view_renders_its_landmarks() {
        let mut app = demo_app("ui-views");
        let map = screen(&app, 160, 48);
        assert!(
            map.contains("OVERWORLD MAP")
                && map.contains("QUEST BOARD")
                && map.contains("BACKEND CITADEL")
        );
        app.project_idx = 2;
        app.enter();
        let realm = screen(&app, 160, 48);
        assert!(
            realm.contains("QUEST PIPELINE")
                && realm.contains("WEB-8")
                && realm.contains("Pricing rules")
        );
        app.enter();
        let dungeon = screen(&app, 160, 48);
        assert!(
            dungeon.contains("DUNGEON · WEB-7") && dungeon.contains("HERO HERE"),
            "{dungeon}"
        );
    }

    #[test]
    fn tiny_and_empty_terminals_do_not_panic() {
        let mut app = demo_app("ui-tiny");
        for view in [View::Overworld, View::Realm, View::Dungeon] {
            app.view = view;
            for (w, h) in [(1, 1), (20, 6), (60, 20)] {
                screen(&app, w, h);
            }
        }
        let empty = App::new(scratch::dir("ui-empty"));
        assert!(screen(&empty, 100, 30).contains("questlog demo"));
    }
}
