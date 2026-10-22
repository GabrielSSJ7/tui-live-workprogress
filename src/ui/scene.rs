//! Which sprites stand for which status. The picture *is* the state:
//! castle = done, hero vs goblin = active, dragon = blocked, locked peak = todo.

use crate::model::Status;
use crate::ui::pixels::{at, Placed, Sprite};
use crate::ui::sprites::{
    CASTLE, CHEST, DRAGON, FIRE, GOBLIN, HERO_GUARD, HERO_STRIKE, MOUNTAIN, PADLOCK, SPARK,
};

/// Overworld emblem size in cells (pixels are 2 per row).
pub const EMBLEM_W: u16 = 22;
pub const EMBLEM_ROWS: u16 = 6;

fn hero(alt: bool) -> &'static Sprite {
    if alt {
        return &HERO_STRIKE;
    }
    &HERO_GUARD
}

/// Foes bob one pixel on alternate frames so active fights read as "live".
fn bob(alt: bool) -> i16 {
    i16::from(alt)
}

fn with_fire(mut parts: Vec<Placed>, alt: bool, x: i16, y: i16) -> Vec<Placed> {
    if alt {
        parts.push(at(&FIRE, x, y));
    }
    parts
}

/// 22x12-pixel art for an overworld tile; `frame` drives the animation.
///
/// ```text
/// emblem(Status::Done, 0) -> [castle]
/// ```
pub fn emblem(status: Status, frame: u64) -> Vec<Placed> {
    let alt = frame % 2 == 1;
    match status {
        Status::Done => vec![at(&CASTLE, 3, 0)],
        Status::Active => vec![at(hero(alt), 0, 0), at(&GOBLIN, 12, bob(alt))],
        Status::Blocked => with_fire(vec![at(&DRAGON, 6, 0)], alt, 0, 3),
        Status::Todo => vec![at(&MOUNTAIN, 3, 0), at(&PADLOCK, 7, 5)],
    }
}

/// Wider scene centered in `width_px` x `height_px`, standing on the bottom row.
pub fn battle(status: Status, frame: u64, width_px: i16, height_px: i16) -> Vec<Placed> {
    let alt = frame % 2 == 1;
    let (cx, y) = (width_px / 2, (height_px - 13).max(0));
    match status {
        Status::Active => {
            let mut parts = vec![at(hero(alt), cx - 12, y), at(&GOBLIN, cx + 2, y + bob(alt))];
            if alt {
                parts.push(at(&SPARK, cx - 1, y + 5));
            }
            parts
        }
        Status::Blocked => with_fire(
            vec![at(&HERO_GUARD, cx - 18, y), at(&DRAGON, cx - 2, y)],
            alt,
            cx - 8,
            y + 3,
        ),
        Status::Done => vec![
            at(&CASTLE, cx - 18, y),
            at(&HERO_GUARD, cx, y),
            at(&CHEST, cx + 12, y + 6),
        ],
        Status::Todo => vec![at(&MOUNTAIN, cx - 8, y), at(&PADLOCK, cx - 4, y + 5)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn widest(parts: &[Placed]) -> i16 {
        parts
            .iter()
            .map(|p| p.x + p.sprite.width())
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn emblems_fit_the_tile() {
        for status in [Status::Todo, Status::Active, Status::Blocked, Status::Done] {
            for frame in 0..2 {
                let parts = emblem(status, frame);
                assert!(
                    widest(&parts) <= EMBLEM_W as i16,
                    "{status:?} frame {frame}"
                );
                assert!(parts
                    .iter()
                    .all(|p| p.x >= 0 && p.y + p.sprite.height() <= 13));
            }
        }
    }

    #[test]
    fn animation_alternates_frames() {
        let still = emblem(Status::Active, 0);
        let moving = emblem(Status::Active, 1);
        assert!(!std::ptr::eq(still[0].sprite, moving[0].sprite));
        assert_eq!(emblem(Status::Blocked, 1).len(), 2);
        assert_eq!(battle(Status::Active, 1, 60, 16).len(), 3);
        assert_eq!(battle(Status::Blocked, 0, 60, 16).len(), 2);
    }

    #[test]
    fn battle_scenes_fit_wide_canvas() {
        for status in [Status::Todo, Status::Active, Status::Blocked, Status::Done] {
            let parts = battle(status, 1, 60, 16);
            assert!(widest(&parts) <= 60 && parts.iter().all(|p| p.x >= 0));
        }
    }
}
