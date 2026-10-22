//! Terminal lifecycle and the event loop: draw, read keys, watch the data dir.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::DefaultTerminal;

use crate::app::App;
use crate::{input, ui};

/// Key polling granularity; also the redraw cadence when idle.
const POLL: Duration = Duration::from_millis(100);
/// One animation frame every 450ms: lively without being distracting.
const ANIM_STEP_MS: u128 = 450;
/// How often the data dir is fingerprinted for changes.
const WATCH_EVERY: Duration = Duration::from_millis(700);

pub fn run(root: PathBuf) -> Result<()> {
    let mut terminal = ratatui::init();
    let outcome = event_loop(&mut terminal, App::new(root));
    ratatui::restore();
    outcome
}

fn event_loop(terminal: &mut DefaultTerminal, mut app: App) -> Result<()> {
    let started = Instant::now();
    let mut last_watch = Instant::now();
    while !app.should_quit {
        app.map_cols = ui::overworld::map_columns(terminal.size()?.width);
        app.frame = (started.elapsed().as_millis() / ANIM_STEP_MS) as u64;
        terminal.draw(|frame| ui::draw(frame, &app))?;
        pump_input(&mut app)?;
        if last_watch.elapsed() >= WATCH_EVERY {
            app.refresh_if_changed();
            last_watch = Instant::now();
        }
    }
    Ok(())
}

fn pump_input(app: &mut App) -> Result<()> {
    if !event::poll(POLL)? {
        return Ok(());
    }
    if let Event::Key(key) = event::read()? {
        input::handle_key(app, key);
    }
    Ok(())
}
