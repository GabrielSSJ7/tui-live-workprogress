//! Pixel art in the terminal: each cell is two vertical pixels drawn with the
//! upper half block `▀` (fg = top pixel, bg = bottom pixel).
//!
//! Sprites are ASCII grids where each char is a palette code and `.` is
//! transparent, so art stays editable in plain text.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::ui::theme;

pub struct Sprite {
    pub rows: &'static [&'static str],
}

impl Sprite {
    pub fn width(&self) -> i16 {
        self.rows
            .iter()
            .map(|r| r.chars().count())
            .max()
            .unwrap_or(0) as i16
    }

    pub fn height(&self) -> i16 {
        self.rows.len() as i16
    }
}

/// A sprite placed at pixel coordinates (y counts half-cells).
#[derive(Clone, Copy)]
pub struct Placed {
    pub sprite: &'static Sprite,
    pub x: i16,
    pub y: i16,
}

pub fn at(sprite: &'static Sprite, x: i16, y: i16) -> Placed {
    Placed { sprite, x, y }
}

/// Palette shared by every sprite (Endesga-like, readable on a dark backdrop).
pub fn pixel_color(code: char) -> Option<Color> {
    let rgb = match code {
        'k' => (24, 20, 37),
        'g' => (139, 155, 180),
        'G' => (90, 105, 136),
        'w' => (244, 244, 244),
        'r' => (228, 59, 68),
        'R' => (158, 40, 53),
        'y' => (254, 231, 97),
        'o' => (247, 118, 34),
        'e' => (99, 199, 77),
        'E' => (38, 92, 66),
        'b' => (184, 111, 80),
        'B' => (116, 63, 57),
        'c' => (192, 203, 220),
        'u' => (18, 78, 137),
        'U' => (0, 153, 219),
        'P' => (181, 80, 136),
        'f' => (58, 68, 102),
        _ => return None,
    };
    Some(Color::Rgb(rgb.0, rgb.1, rgb.2))
}

/// Grid of optional colors, `width` x `height` pixels.
pub struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<Option<Color>>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Canvas {
        Canvas {
            width,
            height,
            pixels: vec![None; width * height],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.pixels[y * self.width + x]
    }

    fn set(&mut self, x: i16, y: i16, color: Color) {
        let inside = x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height;
        if inside {
            self.pixels[y as usize * self.width + x as usize] = Some(color);
        }
    }

    /// Opaque pixels overwrite, `.` leaves what is below: later parts sit on top.
    pub fn paint(&mut self, placed: &Placed) {
        for (dy, row) in placed.sprite.rows.iter().enumerate() {
            for (dx, code) in row.chars().enumerate() {
                if let Some(color) = pixel_color(code) {
                    self.set(placed.x + dx as i16, placed.y + dy as i16, color);
                }
            }
        }
    }

    pub fn fill_row(&mut self, y: usize, color: Color) {
        (0..self.width).for_each(|x| self.set(x as i16, y as i16, color));
    }
}

/// Widget drawing placed sprites (and an optional ground line) into an area.
pub struct PixelScene {
    pub parts: Vec<Placed>,
    pub ground: Option<Color>,
}

impl PixelScene {
    pub fn new(parts: Vec<Placed>) -> PixelScene {
        PixelScene {
            parts,
            ground: None,
        }
    }

    pub fn with_ground(mut self, color: Color) -> PixelScene {
        self.ground = Some(color);
        self
    }

    fn canvas_for(&self, area: Rect) -> Canvas {
        let mut canvas = Canvas::new(area.width as usize, area.height as usize * 2);
        if let (Some(color), true) = (self.ground, area.height > 0) {
            canvas.fill_row(area.height as usize * 2 - 1, color);
        }
        self.parts.iter().for_each(|p| canvas.paint(p));
        canvas
    }
}

impl Widget for PixelScene {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let canvas = self.canvas_for(area);
        for cy in 0..area.height {
            for cx in 0..area.width {
                let top = canvas.get(cx as usize, cy as usize * 2);
                let bottom = canvas.get(cx as usize, cy as usize * 2 + 1);
                write_cell(buf, area.x + cx, area.y + cy, top, bottom);
            }
        }
    }
}

/// Fully transparent cells are left alone so text underneath survives.
fn write_cell(buf: &mut Buffer, x: u16, y: u16, top: Option<Color>, bottom: Option<Color>) {
    if top.is_none() && bottom.is_none() {
        return;
    }
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol("▀")
            .set_fg(top.unwrap_or(theme::BACKDROP))
            .set_bg(bottom.unwrap_or(theme::BACKDROP));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static DOT: Sprite = Sprite {
        rows: &["r.", ".e"],
    };

    #[test]
    fn sprite_dimensions() {
        assert_eq!((DOT.width(), DOT.height()), (2, 2));
    }

    #[test]
    fn canvas_paints_and_clips() {
        let mut canvas = Canvas::new(2, 2);
        canvas.paint(&at(&DOT, 1, 0));
        assert_eq!(canvas.get(1, 0), pixel_color('r'));
        assert_eq!(canvas.get(0, 0), None);
        assert_eq!(canvas.get(5, 5), None);
        canvas.fill_row(1, Color::Red);
        assert_eq!(canvas.get(0, 1), Some(Color::Red));
    }

    #[test]
    fn scene_renders_half_blocks() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        PixelScene::new(vec![at(&DOT, 0, 0)]).render(buf.area, &mut buf);
        let left = &buf[(0, 0)];
        assert_eq!(
            (left.symbol(), left.fg, left.bg),
            ("▀", pixel_color('r').unwrap(), theme::BACKDROP)
        );
        assert_eq!(buf[(1, 0)].bg, pixel_color('e').unwrap());
    }

    #[test]
    fn transparent_cells_keep_existing_text_and_ground_fills() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        buf[(0, 0)].set_symbol("x");
        PixelScene::new(vec![at(&DOT, 0, -1)]).render(buf.area, &mut buf);
        assert_eq!(buf[(0, 0)].symbol(), "x");
        assert_eq!(buf[(1, 0)].symbol(), "▀");
        let mut ground = Buffer::empty(Rect::new(0, 0, 3, 1));
        PixelScene::new(vec![])
            .with_ground(Color::Green)
            .render(ground.area, &mut ground);
        assert_eq!(ground[(2, 0)].bg, Color::Green);
        assert_eq!(pixel_color('?'), None);
    }
}
