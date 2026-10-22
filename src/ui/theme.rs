//! Colors and styles. One accent per status so color alone tells the state.

use ratatui::style::{Color, Modifier, Style};

use crate::model::Status;

pub const BACKDROP: Color = Color::Rgb(14, 16, 28);
/// Slightly lifted backdrop for the selected row.
pub const SELECTION: Color = Color::Rgb(36, 40, 66);
pub const PARCHMENT: Color = Color::Rgb(226, 220, 200);
pub const FADED: Color = Color::Rgb(104, 114, 146);
pub const SHADOW: Color = Color::Rgb(52, 58, 84);
pub const GOLD: Color = Color::Rgb(254, 196, 72);
pub const BLOOD: Color = Color::Rgb(232, 72, 80);
pub const MOSS: Color = Color::Rgb(110, 210, 90);
pub const FROST: Color = Color::Rgb(115, 190, 230);
pub const ARCANE: Color = Color::Rgb(186, 130, 230);
/// Ground line under battle scenes.
pub const TURF: Color = Color::Rgb(38, 92, 66);

pub fn status_color(status: Status) -> Color {
    match status {
        Status::Done => MOSS,
        Status::Active => GOLD,
        Status::Blocked => BLOOD,
        Status::Todo => FADED,
    }
}

pub fn fg(color: Color) -> Style {
    Style::new().fg(color)
}

pub fn bold(color: Color) -> Style {
    Style::new().fg(color).add_modifier(Modifier::BOLD)
}

/// Color of a chronicle line by its kind.
pub fn kind_color(kind: &str) -> Color {
    match kind {
        "done" => MOSS,
        "blocked" | "fail" => BLOOD,
        "progress" | "start" => GOLD,
        "file" | "test" => FROST,
        "unblocked" => ARCANE,
        _ => PARCHMENT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_status_has_its_own_color() {
        let all = [Status::Todo, Status::Active, Status::Blocked, Status::Done];
        let colors: std::collections::HashSet<String> = all
            .iter()
            .map(|s| format!("{:?}", status_color(*s)))
            .collect();
        assert_eq!(colors.len(), 4);
    }

    #[test]
    fn kind_color_matches_status_meaning() {
        assert_eq!(kind_color("done"), status_color(Status::Done));
        assert_eq!(kind_color("blocked"), status_color(Status::Blocked));
        assert_eq!(kind_color("???"), PARCHMENT);
        assert_eq!(bold(GOLD).fg, Some(GOLD));
        assert_eq!(fg(GOLD).fg, Some(GOLD));
    }
}
