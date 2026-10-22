//! Sprite sheet. Palette codes live in `pixels::pixel_color`; `.` is transparent.
//! Every sprite is 12 pixels tall (6 terminal rows) unless noted.

use crate::ui::pixels::Sprite;

/// Knight, sword raised.
pub static HERO_GUARD: Sprite = Sprite {
    rows: &[
        "......w....",
        "......w....",
        "..cc..w....",
        ".cccc.w....",
        ".ckkc.w....",
        ".cccc.y....",
        "uuuuuyy....",
        "ruuuuu.....",
        "ruuuuu.....",
        ".cc.cc.....",
        ".cc.cc.....",
        "kk..kk.....",
    ],
};

/// Knight mid-slash: the second animation frame.
pub static HERO_STRIKE: Sprite = Sprite {
    rows: &[
        "...........",
        "...........",
        "..cc.......",
        ".cccc......",
        ".ckkc......",
        ".cccc......",
        "uuuuuywwwww",
        "ruuuuu.....",
        "ruuuuu.....",
        ".cc.cc.....",
        ".cc.cc.....",
        "kk..kk.....",
    ],
};

pub static GOBLIN: Sprite = Sprite {
    rows: &[
        "..........",
        "..........",
        "e..eeee..e",
        "eeeeeeeeee",
        ".eereeree.",
        ".eewwwwee.",
        "..eeeeee..",
        "bbEEEEEEb.",
        "b.EEEEEE.b",
        "..bbbbbb..",
        "..ee..ee..",
        ".EE...EE..",
    ],
};

/// Faces left, toward the hero. 16 wide.
pub static DRAGON: Sprite = Sprite {
    rows: &[
        "..rr..........R.",
        ".rrrr.......RRR.",
        "rwkrrr.....RRrR.",
        "rrrrrrr...RRrrR.",
        "yy.rrrrr.RRrrrR.",
        "....rrrrrrrrrRR.",
        ".....rrrrrrrrr..",
        ".....ryyyyyrrr..",
        "......ryyyyrr.rr",
        "......rr..rr...r",
        ".....RR...RR....",
        "................",
    ],
};

/// 6x5 breath puff placed left of the dragon's mouth.
pub static FIRE: Sprite = Sprite {
    rows: &["..oy..", ".oyyo.", "oyywyo", ".oyyo.", "..o..."],
};

/// Conquered keep with banners. 16 wide.
pub static CASTLE: Sprite = Sprite {
    rows: &[
        "...krr......krr.",
        "...kr.......kr..",
        "...k........k...",
        ".g.g.g....g.g.g.",
        ".ggggg....ggggg.",
        ".gGgggg.g.ggGgg.",
        ".gggggggggggggg.",
        ".gkgggggggggkgg.",
        ".gggggBBBBggggg.",
        ".gggggBbbBggggg.",
        "eeeeeeBbbBeeeeee",
        "EEEEEEEEEEEEEEEE",
    ],
};

/// Unexplored peak in the fog. 16 wide.
pub static MOUNTAIN: Sprite = Sprite {
    rows: &[
        ".......w........",
        "......wwg.......",
        ".....wggGG......",
        "....gggGGGG..w..",
        "...gggGGGGGGwwg.",
        "..gggGGGGGGgggG.",
        ".gggGGGGGGgggGGG",
        "gggGGGGGGgggGGGG",
        "ggGGGGGGgggGGGGG",
        "gGGGGGGgggGGGGGG",
        "GGGGGGgggGGGGGGG",
        "ffffffffffffffff",
    ],
};

/// 8x7 padlock overlaid on the mountain.
pub static PADLOCK: Sprite = Sprite {
    rows: &[
        "..kkkk..", ".k....k.", ".k....k.", "yyyyyyyy", "yyykkyyy", "yyykkyyy", "yyyyyyyy",
    ],
};

/// 8x6 loot chest for finished quests.
pub static CHEST: Sprite = Sprite {
    rows: &[
        ".bbbbbb.", "bBBBBBBb", "byyyyyyb", "bbbyybbb", "bBbbbbBb", "BBBBBBBB",
    ],
};

/// 3x3 hit flash between hero and foe.
pub static SPARK: Sprite = Sprite {
    rows: &["w.w", ".y.", "w.w"],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::pixels::pixel_color;

    const SHEET: [&Sprite; 10] = [
        &HERO_GUARD,
        &HERO_STRIKE,
        &GOBLIN,
        &DRAGON,
        &FIRE,
        &CASTLE,
        &MOUNTAIN,
        &PADLOCK,
        &CHEST,
        &SPARK,
    ];

    #[test]
    fn rows_are_rectangular() {
        for sprite in SHEET {
            let width = sprite.width() as usize;
            assert!(
                sprite.rows.iter().all(|r| r.chars().count() == width),
                "{:?}",
                sprite.rows
            );
        }
    }

    #[test]
    fn every_code_is_in_the_palette() {
        for sprite in SHEET {
            let unknown: Vec<char> = sprite
                .rows
                .concat()
                .chars()
                .filter(|c| *c != '.' && pixel_color(*c).is_none())
                .collect();
            assert!(unknown.is_empty(), "unknown codes {unknown:?}");
        }
    }
}
