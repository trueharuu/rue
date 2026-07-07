//! Known feature extractions.

use rue_core::{board::Board, header::WIDTH};

/// Returns the heights of each column in the board.
#[inline]
#[must_use]
pub fn heights<const N: usize>(board: &Board<N>) -> [usize; WIDTH as usize] {
    let mut heights = [0; WIDTH as usize];
    for x in 0..WIDTH {
        heights[x as usize] = board.col_height(x as usize) as usize;
    }
    heights
}

/// The total sum of `|h[i]-h[i+1]|` and `(h[i]-h[i+1])^2` for `bumpiness` and `bumpiness_sq` respectively, where `h[i]` is the height of column `i`.
/// Skips the column located at `well_col`, if given.
#[inline]
#[must_use]
pub fn bumpiness(heights: &[usize; WIDTH as usize], well_col: Option<usize>) -> (i32, i32) {
    let mut bump = 0i32;
    let mut bump_sq = 0i32;

    for i in 0..(WIDTH as usize - 1) {
        // skip transitions involving the well column
        if let Some(wc) = well_col {
            if i == wc || i + 1 == wc {
                continue;
            }
        }
        let diff = (heights[i] as i32) - (heights[i + 1] as i32);
        bump += diff.abs();
        bump_sq += diff * diff;
    }

    (bump, bump_sq)
}

/// Finds the deepest well column where both neighbors are taller.
/// Returns `(well_col, well_depth)`.
#[inline]
#[must_use]
pub fn find_well(heights: &[usize; WIDTH as usize]) -> (Option<usize>, i32) {
    let mut best_col = None;
    let mut best_depth = 0i32;

    for x in 0..WIDTH as usize {
        let h = heights[x] as i32;
        let left = if x == 0 { 40 } else { heights[x - 1] as i32 };
        let right = if x == WIDTH as usize - 1 {
            40
        } else {
            heights[x + 1] as i32
        };

        if left > h && right > h {
            let depth = left.min(right) - h;
            if depth > best_depth {
                best_depth = depth;
                best_col = Some(x);
            }
        }
    }

    (best_col, best_depth)
}

/// Returns the number of holes and the number of covered blocks in the board.
/// A hole is defined to be an empty cell located below the column's height.
/// A covered block is defined to be a filled cell above the topmost hole, and is capped at 6 per column.
#[inline]
#[must_use]
pub fn holes_and_covered<const N: usize>(board: &Board<N>, heights: &[usize; WIDTH as usize]) -> (i32, i32) {
    let mut holes = 0i32;
    let mut covered = 0i32;

    let cols = board.as_cols();

    for (x, &h) in heights.iter().enumerate() {
        if h == 0 {
            continue;
        }

        let below_mask = (1u64 << h) - 1;
        let filled_below = cols[x] & below_mask;
        let col_holes = h as i32 - filled_below.count_ones() as i32;
        holes += col_holes;

        if col_holes == 0 {
            continue;
        }

        let empty_below = !cols[x] & below_mask;
        let topmost_hole = 63usize - empty_below.leading_zeros() as usize;
        let at_or_below_hole = (1u64 << (topmost_hole + 1)) - 1;
        let cov = (filled_below & !at_or_below_hole).count_ones() as i32;
        covered += cov.min(6);
    }

    (holes, covered)
}