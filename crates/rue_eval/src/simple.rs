use rue_core::game::attack::Attack;
use rue_core::game::search::SearchGame;
use rue_core::placement::Move;
use rue_core::rule::Rule;

use crate::model::Model;

pub struct Simple {
    pub holes: f32,
    pub cell_coveredness: f32,
    pub height: f32,
    pub height_upper_half: f32,
    pub height_upper_quarter: f32,
    pub bumpiness: f32,
    pub bumpiness_sq: f32,
    pub row_transitions: f32,
    pub well_depth: f32,

    // extrinsic features
    pub combo: f32,
    pub b2b: f32,
    pub incoming: f32,
    
    // attack
    pub clear: [[f32; 5]; 3],
    pub base_attack: f32,
    pub attack: f32,
    pub pc: f32,

    // playstyle
    pub well_distance: f32,
}

impl Default for Simple {
    fn default() -> Self {
        Self {
            holes: -4.0,
            cell_coveredness: -0.5,
            height: -0.2,
            height_upper_half: -1.0,
            height_upper_quarter: -5.0,
            bumpiness: -0.3,
            bumpiness_sq: -0.1,
            row_transitions: -0.3,
            well_depth: 0.2,
            attack: 0.5,
            base_attack: 2.5,
            combo: 0.3,
            b2b: 2.0,
            pc: 6.0,
            incoming: -0.5,
            clear: [
                [0.0, -10.0, -10.0, -10.0, 1.0],
                [0.0, 0.5, 0.25, 0.25, 0.0],
                [0.0, 0.75, 10.0, 1.0, 0.0],
            ],
            // reward spins that are far from the well
            well_distance: 1.0,
        }
    }
}

impl Model for Simple {
    fn name(&self) -> &'static str {
        "simple"
    }

    fn evaluate<const N: usize, const RULE: Rule>(
        &self,
        game: &SearchGame<N>,
        placement: Move,
        ctx: Attack,
    ) -> f32 {
        let board = &game.board;
        let max_height = board.height() as usize;
        let max_height_f = max_height as f32;

        let mut score = 0.0;

        score += self.height * max_height_f;
        if max_height_f > 10.0 {
            score += self.height_upper_half * (max_height_f - 10.0);
        }

        if max_height_f > 15.0 {
            score += self.height_upper_quarter * (max_height_f - 15.0);
        }

        let (cols, heights) = feature::cols_and_heights(board, max_height);

        let (holes, covered) = feature::holes_and_covered(cols, &heights);

        score += self.holes * holes as f32;
        score += self.cell_coveredness * covered as f32;

        let (well_col, well_depth) = feature::find_well(&heights);
        let (bump, bump_sq) = feature::bumpiness(&heights, None);
        let r_transitions = feature::row_transitions::<N>(board, max_height);

        score += self.bumpiness * bump as f32;
        score += self.bumpiness_sq * bump_sq as f32;
        score += self.row_transitions * r_transitions as f32;

        score += self.well_depth * well_depth as f32;

        // Extrinsic terms from the placement and game context.
        score += self.combo * game.combo.map_or(0, |c| c as i32) as f32;
        score += self.b2b * game.b2b.map_or(0, |b| b as i32) as f32;
        score += self.incoming * game.incoming as f32;
        
        // Attack-specific terms.
        score += self.clear[ctx.spin_type as usize][ctx.line_clears as usize];
        if ctx.is_perfect_clear {
            score += self.pc;
        }
        score += self.attack * ctx.total as f32;
        score += self.base_attack * ctx.base_attack as f32;

        if ctx.is_special_clear() && let Some(col) = well_col {
            let centered_at = placement.x();
            score += self.well_distance * (col as i32 - centered_at).abs() as f32;
        }
        
        score
    }
}

mod feature {
    use rue_core::board::Board;
    use rue_core::header::TLINES;
    use rue_core::header::WIDTH;

    const ROW_MASK: u64 = (1u64 << WIDTH) - 1;

    /// Returns the occupancy bits of row `y` as a 10-bit mask.
    #[inline]
    fn row_bits<const N: usize>(board: &Board<N>, y: usize) -> u64 {
        let band = y / TLINES as usize;
        let row = (y % TLINES as usize) as u32;
        (board.0[band] >> (row * WIDTH as u32)) & ROW_MASK
    }

    pub fn cols_and_heights<const N: usize>(
        board: &Board<N>,
        max_height: usize,
    ) -> ([u64; WIDTH as usize], [usize; WIDTH as usize]) {
        let mut cols = [0u64; WIDTH as usize];

        let mut y = 0;
        while y < max_height {
            let r = row_bits(board, y);
            let mut x = 0usize;
            while x < WIDTH as usize {
                cols[x] |= ((r >> x) & 1) << y;
                x += 1;
            }
            y += 1;
        }

        let mut heights = [0usize; WIDTH as usize];
        let mut x = 0usize;
        while x < WIDTH as usize {
            heights[x] = if cols[x] == 0 {
                0
            } else {
                64 - cols[x].leading_zeros() as usize
            };
            x += 1;
        }

        (cols, heights)
    }

    #[inline]
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

    /// Returns the occupancy bits of row `y`, or zeros for rows beyond the
    /// board's capacity.
    #[inline]
    fn row_at<const N: usize>(board: &Board<N>, y: usize) -> u64 {
        if y / TLINES as usize >= N {
            0
        } else {
            row_bits(board, y)
        }
    }

    #[inline]
    pub fn row_transitions<const N: usize>(board: &Board<N>, max_height: usize) -> i32 {
        const LANE_LSB: u64 = 0x0001_0001_0001_0001;
        const LANE_LOW9: u64 = 0x01FF_01FF_01FF_01FF;
        const LANE_SHIFT_GUARD: u64 = 0x7FFF_7FFF_7FFF_7FFF;

        let mut total = 0u32;
        let mut y = 0usize;
        while y < max_height {
            let v = row_at(board, y)
                | row_at(board, y + 1) << 16
                | row_at(board, y + 2) << 32
                | row_at(board, y + 3) << 48;
            let nz = ((v + LANE_SHIFT_GUARD) >> 15) & LANE_LSB;
            let xor = v ^ ((v >> 1) & LANE_SHIFT_GUARD);
            total += (xor & LANE_LOW9).count_ones();
            total += ((!v) & LANE_LSB & nz).count_ones();
            total += ((!(v >> 9)) & LANE_LSB & nz).count_ones();
            y += 4;
        }
        total as i32
    }

    #[inline]
    pub fn holes_and_covered(
        cols: [u64; WIDTH as usize],
        heights: &[usize; WIDTH as usize],
    ) -> (i32, i32) {
        let mut holes = 0i32;
        let mut covered = 0i32;

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

    #[cfg(test)]
    mod parity {
        use super::*;

        fn ref_column_heights(board: &Board<8>) -> [usize; WIDTH as usize] {
            let mut heights = [0; WIDTH as usize];
            for (x, h) in heights.iter_mut().enumerate() {
                for y in (0..board.height()).rev() {
                    if board.get(x as i32, y) {
                        *h = y as usize + 1;
                        break;
                    }
                }
            }
            heights
        }

        fn ref_cols(board: &Board<8>, max_height: usize) -> [u64; WIDTH as usize] {
            let mut cols = [0u64; WIDTH as usize];
            for y in 0..max_height {
                let bits = row_bits(board, y);
                for (x, col) in cols.iter_mut().enumerate() {
                    if (bits >> x) & 1 == 1 {
                        *col |= 1u64 << y;
                    }
                }
            }
            cols
        }

        fn ref_rows(board: &Board<8>, max_height: usize) -> Vec<u64> {
            let mut rows = vec![0u64; max_height.div_ceil(4) * 4];
            for (y, row) in rows.iter_mut().enumerate().take(max_height) {
                *row = row_bits(board, y);
            }
            rows
        }

        fn ref_row_transitions(rows: &[u64], max_height: usize) -> i32 {
            const LANE_LSB: u64 = 0x0001_0001_0001_0001;
            const LANE_LOW9: u64 = 0x01FF_01FF_01FF_01FF;
            const LANE_SHIFT_GUARD: u64 = 0x7FFF_7FFF_7FFF_7FFF;

            let mut total = 0u32;
            let mut y = 0usize;
            while y < max_height {
                let v = rows[y] | rows[y + 1] << 16 | rows[y + 2] << 32 | rows[y + 3] << 48;
                let nz = ((v + LANE_SHIFT_GUARD) >> 15) & LANE_LSB;
                let xor = v ^ ((v >> 1) & LANE_SHIFT_GUARD);
                total += (xor & LANE_LOW9).count_ones();
                total += ((!v) & LANE_LSB & nz).count_ones();
                total += ((!(v >> 9)) & LANE_LSB & nz).count_ones();
                y += 4;
            }
            total as i32
        }

        fn test_boards() -> Vec<Board<8>> {
            let mut boards = Vec::new();

            boards.push(Board::<8>::empty());

            let mut b = Board::<8>::empty();
            for x in 0..10 {
                if x != 5 {
                    b.set(x, 0);
                }
            }
            boards.push(b);

            let mut b = Board::<8>::empty();
            for x in 0..10 {
                b.set(x, 0);
            }
            for x in 2..10 {
                b.set(x, 1);
            }
            boards.push(b);

            let mut b = Board::<8>::empty();
            for y in 0..5 {
                for x in 0..10 {
                    if (x + y) % 3 != 0 {
                        b.set(x, y);
                    }
                }
            }
            boards.push(b);

            let mut b = Board::<8>::empty();
            for y in 0..12 {
                b.set(3, y);
            }
            boards.push(b);

            let mut b = Board::<8>::empty();
            for x in 0..10 {
                b.set(x, 0);
                b.set(x, 2);
            }
            boards.push(b);

            let mut b = Board::<8>::empty();
            for x in 0..10 {
                for y in 0..5 {
                    if (x + y) % 2 == 0 {
                        b.set(x, y);
                    }
                }
            }
            for y in 5..11 {
                b.set(4, y);
                b.set(5, y);
            }
            boards.push(b);

            boards
        }

        #[test]
        fn parity_with_reference() {
            for board in test_boards() {
                let max_height = board.height() as usize;

                let ref_h = ref_column_heights(&board);
                let ref_c = ref_cols(&board, max_height);
                let ref_rt = ref_row_transitions(&ref_rows(&board, max_height), max_height);

                let (new_c, new_h) = cols_and_heights(&board, max_height);
                let new_rt = row_transitions::<8>(&board, max_height);

                assert_eq!(ref_h, new_h);
                assert_eq!(ref_c, new_c);
                assert_eq!(ref_rt, new_rt);
            }
        }
    }
}
