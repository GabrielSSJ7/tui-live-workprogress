//! Small shared building blocks: panels, HP-style bars, status tags, text fitting.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType};

use crate::model::Status;
use crate::ui::theme;

/// Rounded panel with a centered, bold title in the accent color.
pub fn panel(title: &str, accent: ratatui::style::Color) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::fg(accent))
        .title(Line::from(Span::styled(format!(" {title} "), theme::bold(accent))).centered())
}

/// `██████░░░░` sized to `width` cells.
///
/// ```text
/// hp_bar(50, 4, GOLD) -> "██" (gold) + "░░" (shadow)
/// ```
pub fn hp_bar(percent: u8, width: u16, color: ratatui::style::Color) -> Vec<Span<'static>> {
    let width = width as usize;
    let filled = (width * percent.min(100) as usize + 50) / 100;
    vec![
        Span::styled("█".repeat(filled), theme::fg(color)),
        Span::styled("░".repeat(width - filled), theme::fg(theme::SHADOW)),
    ]
}

/// `[DONE   ]`, fixed 9 cells wide so rows line up.
pub fn status_tag(status: Status) -> Span<'static> {
    let style = theme::bold(theme::status_color(status));
    Span::styled(format!("[{:<7}]", status.label()), style)
}

/// Pad a line with spaces to `width` so a row background spans the panel.
pub fn pad_to(mut line: Line<'static>, width: u16) -> Line<'static> {
    let used = line.width();
    if used < width as usize {
        line.spans
            .push(Span::raw(" ".repeat(width as usize - used)));
    }
    line
}

pub fn icon(status: Status) -> Span<'static> {
    Span::styled(status.icon(), theme::bold(theme::status_color(status)))
}

/// Cut to `max` chars with an ellipsis; `max == 0` yields an empty string.
pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let kept: String = text.chars().take(max - 1).collect();
    format!("{kept}…")
}

/// Left-pad / right-pad to exactly `width` chars (truncating when longer).
pub fn fit(text: &str, width: usize) -> String {
    format!("{:<width$}", truncate(text, width))
}

/// Rect of `width` x `height` centered inside `outer` (clamped to fit).
pub fn centered(outer: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(outer.width);
    let height = height.min(outer.height);
    Rect {
        x: outer.x + (outer.width - width) / 2,
        y: outer.y + (outer.height - height) / 2,
        width,
        height,
    }
}

/// Style for a list row, lifted when selected.
pub fn row_style(selected: bool) -> Style {
    if selected {
        return Style::new()
            .bg(theme::SELECTION)
            .add_modifier(Modifier::BOLD);
    }
    Style::new()
}

/// `▶ ` for the selected row, two spaces otherwise.
pub fn cursor(selected: bool) -> Span<'static> {
    if selected {
        return Span::styled("▶ ", theme::bold(theme::GOLD));
    }
    Span::raw("  ")
}

/// First index of a window of `visible` rows that keeps `selected` on screen.
pub fn scroll_start(selected: usize, visible: usize) -> usize {
    if visible == 0 {
        return selected;
    }
    selected.saturating_sub(visible - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(spans: &[Span]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn hp_bar_fills_proportionally() {
        assert_eq!(text_of(&hp_bar(50, 4, theme::GOLD)), "██░░");
        assert_eq!(text_of(&hp_bar(100, 3, theme::GOLD)), "███");
        assert_eq!(text_of(&hp_bar(0, 3, theme::GOLD)), "░░░");
        assert_eq!(text_of(&hp_bar(250, 2, theme::GOLD)), "██");
    }

    #[test]
    fn status_tag_is_fixed_width() {
        assert_eq!(status_tag(Status::Done).content, "[DONE   ]");
        assert_eq!(status_tag(Status::Blocked).content, "[BLOCKED]");
        assert_eq!(pad_to(Line::raw("ab"), 5).width(), 5);
        assert_eq!(pad_to(Line::raw("abcdef"), 5).width(), 6);
        assert_eq!(icon(Status::Done).content, "✓");
    }

    #[test]
    fn truncate_and_fit() {
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("abc", 4), "abc");
        assert_eq!(truncate("abc", 0), "");
        assert_eq!(fit("ab", 4), "ab  ");
        assert_eq!(fit("abcdef", 4), "abc…");
    }

    #[test]
    fn centered_clamps() {
        let outer = Rect::new(10, 10, 20, 10);
        assert_eq!(centered(outer, 10, 4), Rect::new(15, 13, 10, 4));
        assert_eq!(centered(outer, 99, 99), outer);
    }

    #[test]
    fn scroll_keeps_selection_visible() {
        assert_eq!(scroll_start(0, 5), 0);
        assert_eq!(scroll_start(7, 5), 3);
        assert_eq!(scroll_start(3, 0), 3);
        assert_eq!(cursor(true).content, "▶ ");
        assert!(row_style(true).bg.is_some() && row_style(false).bg.is_none());
        assert!(panel("x", theme::GOLD).inner(Rect::new(0, 0, 4, 4)).width == 2);
    }
}
