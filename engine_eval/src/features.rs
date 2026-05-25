use engine_core::board::{Board, COL_NB};

#[inline]
#[must_use]
pub fn column_heights(board: &Board) -> [usize; COL_NB] {
    let mut heights = [0usize; COL_NB];

    for (x, h) in heights.iter_mut().enumerate() {
        let col = board.cols[x];
        *h = if col == 0 {
            0
        } else {
            (64 - col.leading_zeros()) as usize
        };
    }

    heights
}

#[inline]
#[must_use]
pub fn covered_cells(board: &Board, heights: &[usize; COL_NB]) -> (i32, i32) {
    let mut covered = 0;
    let mut covered_sq = 0;

    for (x, &h) in heights.iter().enumerate().take(10) {
        if h <= 2 {
            continue;
        }
        for y in (0..h - 2).rev() {
            if !board.occupied(x as i32, y as i32) {
                let cells = 6.min(h - y - 1) as i32;
                covered += cells;
                covered_sq += cells * cells;
            }
        }
    }

    (covered, covered_sq)
}

#[inline]
#[must_use]
pub fn bumpiness(heights: &[usize; COL_NB], well_col: Option<usize>) -> (i32, i32) {
    let mut bump = 0i32;
    let mut bump_sq = 0i32;

    for i in 0..(COL_NB - 1) {
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

#[inline]
#[must_use]
pub fn find_well(heights: &[usize; COL_NB]) -> (Option<usize>, i32) {
    let mut best_col = None;
    let mut best_depth = 0i32;

    for x in 0..COL_NB {
        let h = heights[x] as i32;
        let left = if x == 0 { 40 } else { heights[x - 1] as i32 };
        let right = if x == COL_NB - 1 {
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

#[inline]
#[must_use]
pub fn count_tsd_overhangs(board: &Board, heights: &[usize; COL_NB]) -> i32 {
    let mut count = 0i32;

    for c in 0..COL_NB {
        let h = heights[c];
        if h < 2 {
            continue;
        }

        // Overhang: filled at top, empty directly below
        let has_overhang =
            board.occupied(c as i32, h as i32 - 1) && !board.occupied(c as i32, h as i32 - 2);

        if !has_overhang {
            continue;
        }

        let wall_left = c > 0
            && heights[c - 1] >= h
            && board.occupied(c as i32 - 1, h as i32 - 1)
            && board.occupied(c as i32 - 1, h as i32 - 2);

        let wall_right = c < COL_NB - 1
            && heights[c + 1] >= h
            && board.occupied(c as i32 + 1, h as i32 - 1)
            && board.occupied(c as i32 + 1, h as i32 - 2);

        if wall_left {
            let open_right = c < COL_NB - 1 && !board.occupied(c as i32 + 1, h as i32 - 2);
            let open_right = open_right || c == COL_NB - 1;
            if open_right {
                count += 1;
            }
        }
        if wall_right {
            let open_left = c > 0 && !board.occupied(c as i32 - 1, h as i32 - 2);
            let open_left = open_left || c == 0;
            if open_left {
                count += 1;
            }
        }
    }

    count.min(2)
}

#[inline]
#[must_use]
pub fn row_transitions(board: &Board, max_height: usize) -> i32 {
    let mut total = 0i32;
    for y in 0..max_height {
        let row = board.row(y);
        if row == 0 {
            continue;
        }

        let shifted = row >> 1;
        let xor = row ^ shifted;

        total += (xor & 0x1FF).count_ones() as i32;

        if row & 1 == 0 {
            total += 1;
        }

        if row & (1 << 9) == 0 {
            total += 1;
        }
    }
    total
}

#[inline]
#[must_use] 
pub fn holes_and_covered(board: &Board, heights: &[usize; COL_NB]) -> (i32, i32) {
    let mut holes = 0i32;
    let mut covered = 0i32;

    for (x, &h) in heights.iter().enumerate() {
        if h == 0 {
            continue;
        }

        let mut topmost_hole = None;
        for y in (0..h).rev() {
            if !board.occupied(x as i32, y as i32) {
                holes += 1;
                if topmost_hole.is_none() {
                    topmost_hole = Some(y);
                }
            }
        }

        if let Some(hole_y) = topmost_hole {
            let mut cov = 0;
            for y in (hole_y + 1)..h {
                if board.occupied(x as i32, y as i32) {
                    cov += 1;
                }
            }

            covered += cov.min(6);
        }
    }

    (holes, covered)
}
