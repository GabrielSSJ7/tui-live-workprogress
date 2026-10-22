//! Serpentine layout of realms on the overworld: row 0 runs left→right, row 1
//! right→left, and so on, so the road between consecutive realms stays short.

/// `(row, col)` of the `index`-th realm.
///
/// ```text
/// cell_of(3, 3) == (1, 2)   // second row starts on the right
/// ```
pub fn cell_of(index: usize, cols: usize) -> (usize, usize) {
    let cols = cols.max(1);
    let (row, offset) = (index / cols, index % cols);
    if row % 2 == 0 {
        return (row, offset);
    }
    (row, cols - 1 - offset)
}

pub fn index_of(row: usize, col: usize, cols: usize) -> usize {
    let cols = cols.max(1);
    let col = col.min(cols - 1);
    if row.is_multiple_of(2) {
        return row * cols + col;
    }
    row * cols + (cols - 1 - col)
}

/// Move visually by `(dx, dy)` cells; stays inside the `len` realms.
pub fn step(index: usize, len: usize, cols: usize, dx: i64, dy: i64) -> usize {
    if len == 0 {
        return 0;
    }
    let (row, col) = cell_of(index, cols);
    let last_row = (len - 1) / cols.max(1);
    let row = (row as i64 + dy).clamp(0, last_row as i64) as usize;
    let col = (col as i64 + dx).clamp(0, cols.max(1) as i64 - 1) as usize;
    let target = index_of(row, col, cols);
    if target >= len && dx != 0 {
        return index;
    }
    target.min(len - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serpentine_cells_roundtrip() {
        assert_eq!(cell_of(0, 3), (0, 0));
        assert_eq!(cell_of(3, 3), (1, 2));
        assert_eq!(cell_of(5, 3), (1, 0));
        for i in 0..9 {
            let (row, col) = cell_of(i, 3);
            assert_eq!(index_of(row, col, 3), i);
        }
        assert_eq!(cell_of(4, 0), (4, 0));
    }

    #[test]
    fn step_moves_visually_and_clamps() {
        assert_eq!(step(0, 5, 3, 1, 0), 1);
        assert_eq!(step(2, 5, 3, 0, 1), 3);
        assert_eq!(step(3, 5, 3, -1, 0), 4);
        assert_eq!(step(4, 5, 3, -1, 0), 4);
        assert_eq!(step(0, 5, 3, -1, -1), 0);
        assert_eq!(step(0, 2, 3, 0, 5), 0);
        assert_eq!(step(0, 0, 3, 1, 0), 0);
    }
}
