//! Dev aid: render every view of the demo world to an HTML file, cell by cell,
//! so the pixel art can be screenshotted without a real terminal.
//!
//! cargo run --example snapshot -- <out-dir> [width] [height]

use std::fmt::Write as _;
use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;
use ratatui::Terminal;
use tui_live_workprogress::app::{App, View};
use tui_live_workprogress::{demo, ui};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = PathBuf::from(args.first().map(String::as_str).unwrap_or("."));
    let width: u16 = args.get(1).map_or(Ok(160), |w| w.parse())?;
    let height: u16 = args.get(2).map_or(Ok(48), |h| h.parse())?;
    let root = std::env::temp_dir().join(format!("questlog-snapshot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    demo::seed(&root)?;
    let mut app = App::new(root);
    app.map_cols = ui::overworld::map_columns(width);
    app.project_idx = 2;
    app.frame = 1;
    for (view, name) in [
        (View::Overworld, "overworld"),
        (View::Realm, "realm"),
        (View::Dungeon, "dungeon"),
    ] {
        if view != View::Overworld {
            app.enter();
        }
        assert_eq!(app.view, view);
        let mut terminal = Terminal::new(TestBackend::new(width, height))?;
        terminal.draw(|f| ui::draw(f, &app))?;
        std::fs::write(
            out.join(format!("{name}.html")),
            to_html(terminal.backend().buffer()),
        )?;
    }
    Ok(())
}

fn css(color: Color, fallback: &str) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        _ => fallback.to_string(),
    }
}

fn to_html(buffer: &Buffer) -> String {
    let mut html = String::from("<!doctype html><meta charset=utf-8><style>body{margin:0;background:#0e101c}pre{margin:0;font:16px/1 'DejaVu Sans Mono',monospace}span{display:inline-block;width:.6em;height:1em;text-align:center;overflow:hidden;vertical-align:top}</style><pre>");
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            let symbol = match cell.symbol() {
                "<" => "&lt;",
                ">" => "&gt;",
                "&" => "&amp;",
                s => s,
            };
            let _ = write!(
                html,
                "<span style=\"color:{};background:{}\">{}</span>",
                css(cell.fg, "#e2dcc8"),
                css(cell.bg, "#0e101c"),
                symbol
            );
        }
        html.push('\n');
    }
    html + "</pre>"
}
