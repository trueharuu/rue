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
        if let Some(wc) = well_col
            && (i == wc || i + 1 == wc)
        {
            continue;
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
pub fn holes_and_covered<const N: usize>(
    board: &Board<N>,
    heights: &[usize; WIDTH as usize],
) -> (i32, i32) {
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

// oracle sample implementation from `Mochbot/fusion`
// #[inline]
// fn row_transitions(board: &Board, max_height: usize) -> i32 {
//     const LANE_LSB: u64 = 0x0001_0001_0001_0001;
//     const LANE_LOW9: u64 = 0x01FF_01FF_01FF_01FF;
//     const LANE_SHIFT_GUARD: u64 = 0x7FFF_7FFF_7FFF_7FFF;

//     let mut total = 0u32;
//     let mut y = 0usize;
//     while y < max_height {
//         let v = (board.rows[y] as u64)
//             | (board.rows[y + 1] as u64) << 16
//             | (board.rows[y + 2] as u64) << 32
//             | (board.rows[y + 3] as u64) << 48;
//         let nz = ((v + LANE_SHIFT_GUARD) >> 15) & LANE_LSB;
//         let xor = v ^ ((v >> 1) & LANE_SHIFT_GUARD);
//         total += (xor & LANE_LOW9).count_ones();
//         total += ((!v) & LANE_LSB & nz).count_ones();
//         total += ((!(v >> 9)) & LANE_LSB & nz).count_ones();
//         y += 4;
//     }
//     total as i32
// }
/// Counts row transitions: horizontal transitions between adjacent cells plus walls within each row up to `max_height`.
#[inline]
#[must_use]
pub fn row_transitions<const N: usize>(board: &Board<N>, max_height: usize) -> i32 {
    let mut total = 0;
    for y in 0..max_height {
        let mut row = 0u16;
        for x in 0..WIDTH as usize {
            if board.get(x as i32, y as i32) {
                row |= 1u16 << x;
            }
        }
        if row == 0 {
            continue;
        }
        let xor = row ^ (row >> 1);
        total += (xor & 0x1FF).count_ones() as i32;
        if row & 1 == 0 {
            total += 1;
        }
        if (row >> 9) & 1 == 0 {
            total += 1;
        }
    }
    total
}