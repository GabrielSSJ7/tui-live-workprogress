//! Key bindings. Vim keys and arrows both work; nothing here needs a modifier.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::App;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Enter | KeyCode::Char(' ') => app.enter(),
        KeyCode::Esc | KeyCode::Backspace => app.back(),
        KeyCode::Char('r') => app.reload(),
        code => move_for(app, code),
    }
}

fn move_for(app: &mut App, code: KeyCode) {
    let (dx, dy) = match code {
        KeyCode::Left | KeyCode::Char('h') => (-1, 0),
        KeyCode::Right | KeyCode::Char('l') => (1, 0),
        KeyCode::Up | KeyCode::Char('k') => (0, -1),
        KeyCode::Down | KeyCode::Char('j') => (0, 1),
        _ => return,
    };
    app.move_by(dx, dy);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::View;
    use crate::scratch;

    fn press(app: &mut App, code: KeyCode) {
        handle_key(app, KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn keys_drive_navigation_and_quit() {
        let root = scratch::dir("input-keys");
        crate::demo::seed(&root).unwrap();
        let mut app = App::new(root);
        app.map_cols = 5;
        press(&mut app, KeyCode::Char('l'));
        assert_eq!(app.project_idx, 1);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.view, View::Realm);
        press(&mut app, KeyCode::Down);
        assert_eq!(app.demand_idx, 1);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.view, View::Overworld);
        press(&mut app, KeyCode::Char('x'));
        assert!(!app.should_quit);
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert!(app.should_quit);
    }
}
