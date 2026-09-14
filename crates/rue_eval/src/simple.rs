use rue_core::game::attack::Attack;
use rue_core::game::search::SearchGame;
use rue_core::piece::Piece;
use rue_core::placement::Move;
use rue_core::rule::Rule;
use rue_core::spin::Spin;

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
    pub well_col: [f32; 10],
    pub col_height: [f32; 10],

    // extrinsic features
    pub combo: f32,
    pub b2b: f32,
    pub b2b_break: f32,
    pub incoming: f32,

    // attack
    pub clear: [[f32; 5]; 3],
    pub base_attack: f32,
    pub attack: f32,
    pub pc: f32,

    // spin-chain readiness, modeled on coldclear freestyle
    pub tslot: [f32; 4],

    // playstyle
    pub well_distance: f32,
    pub t_waste: f32,
    pub height_difference: f32,
}

impl Default for Simple {
    fn default() -> Self {
        Self {
            holes: -4.0,
            cell_coveredness: -4.5,
            height: -0.5,
            height_upper_half: -3.0,
            height_upper_quarter: -5.0,
            bumpiness: -0.3,
            bumpiness_sq: -0.1,
            row_transitions: -0.3,
            attack: 0.5,
            base_attack: 2.0,
            combo: 0.3,
            b2b: 3.0,
            b2b_break: -20.0,
            pc: 2.0,
            incoming: -0.5,
            tslot: [0.0, 5.0, 20.0, 13.0],
            clear: [
                [0.0, -5.0, -5.0, -5.0, -10.0],
                [0.0, 2.5, -1.0, -2.5, 0.0],
                [0.0, -0.75, 5.0, -1.0, 0.0],
            ],
            well_distance: 2.0,
            well_depth: 1.2,
            well_col: [-0.2, -1.0, -0.2, 1.0, 0.4, 0.4, 1.0, -0.2, -1.0, -0.2],
            col_height: [0.1, 0.05, 0.0, -0.05, -0.25, -0.25, -0.05, 0.0, 0.05, 0.1],
            // col_height: [-1.0; 10],
            t_waste: -0.3,
            height_difference: -1.2,
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
        for (x, &h) in heights.iter().enumerate() {
            score += self.col_height[x] * h as f32;
        }

        let (holes, covered) = feature::holes_and_covered(cols, &heights);

        score += self.holes * holes as f32;
        score += self.cell_coveredness * covered as f32;

        let (well_col, well_depth) = feature::find_well(&heights);
        let (bump, bump_sq) = feature::bumpiness(&heights, None);
        let r_transitions = feature::row_transitions::<N>(board, max_height);

        score += self.bumpiness * bump as f32;
        score += self.bumpiness_sq * bump_sq as f32;
        score += self.row_transitions * r_transitions as f32;

        if let Some(col) = well_col {
            score += self.well_col[col] * well_depth.min(4) as f32;
        }

        // Extrinsic terms from the placement and game context.
        score += self.combo * game.combo.map_or(0, |c| c as i32) as f32;
        score += self.b2b * game.b2b.map_or(0, |b| (b as i32) + 1).min(8) as f32;
        score += self.incoming * game.incoming as f32;

        // Attack-specific terms.
        score += self.clear[ctx.spin_type as usize][ctx.line_clears as usize];
        if ctx.is_perfect_clear {
            score += self.pc;
        }
        score += self.attack * ctx.total as f32;
        score += self.base_attack * ctx.base_attack as f32;

        // An ordinary clear of an active chain breaks it. Plain stacking does
        // not: `chain_broken` is also true when no lines clear.
        if ctx.line_clears > 0 && ctx.spin_type == Spin::None {
            score += self.b2b_break;
        }

        if ctx.is_special_clear()
            && let Some(col) = well_col
        {
            let centered_at = placement.x();
            score += self.well_distance * (col as i32 - centered_at).abs() as f32;
        }

        let cutouts = feature::t_available(game);
        if cutouts > 0 {
            let mut probe = game.board;
            for _ in 0..cutouts {
                let Some((px, py)) = feature::well_known_tslot_left::<N>(&probe)
                    .or_else(|| feature::well_known_tslot_right::<N>(&probe))
                else {
                    break;
                };
                let Some(lines) = feature::virtual_t_fire::<N>(&mut probe, px, py) else {
                    break;
                };
                score += self.tslot[lines.min(3) as usize];
                if lines < 2 {
                    break;
                }
            }
        }

        // T waste is defined as T placements that are not FULL spins
        if placement.piece() == Piece::T && (ctx.spin_type != Spin::Full || ctx.line_clears == 0) {
            score += self.t_waste;
        }

        // Height difference is defined to be |left - right| where, if a `well` exists,
        // `left` is the height of the column to the left of the well and `right` is the
        // height of the column to the right of the well.
        if let Some(col) = well_col
            && col > 0
            && col < 9
        {
            let left = heights[col - 1] as f32;
            let right = heights[col + 1] as f32;
            score += self.height_difference * (left - right).abs();
        }

        score
    }
}

mod feature {
    use rue_core::board::Board;
    use rue_core::game::search::SearchGame;
    use rue_core::header::TLINES;
    use rue_core::header::WIDTH;
    use rue_core::piece::Piece;
    use rue_core::placement::Move;
    use rue_core::rotation::Rotation;
    use rue_core::spin::Spin;

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

    /// Returns the height (topmost filled row + 1) of each column.
    fn column_heights<const N: usize>(board: &Board<N>) -> [usize; WIDTH as usize] {
        let mut heights = [0usize; WIDTH as usize];
        for (x, h) in heights.iter_mut().enumerate() {
            let mut y = 0usize;
            let mut band = N;
            while band > 0 {
                band -= 1;
                let w = board.0[band];
                if w == 0 {
                    continue;
                }
                let mut row = TLINES as usize;
                while row > 0 {
                    row -= 1;
                    let bit = row * WIDTH as usize + x;
                    if w & (1u64 << bit) != 0 {
                        y = band * TLINES as usize + row + 1;
                        break;
                    }
                }
                if y != 0 {
                    break;
                }
            }
            *h = y;
        }
        heights
    }

    /// Port of coldclear's `well_known_tslot_left`.
    /// Returns the anchor `(x, y)` for a fitting T in South rotation.
    pub fn well_known_tslot_left<const N: usize>(board: &Board<N>) -> Option<(i32, i32)> {
        let heights = column_heights(board);
        let total = Board::<N>::total_height();
        for x in 0..(WIDTH as usize - 2) {
            let y = heights[x] as i32;
            if heights[x + 1] as i32 >= y {
                continue;
            }
            if y < 1 || y + 1 >= total {
                continue;
            }
            let rx = x + 2;
            if !board.get(rx as i32, y - 1) {
                continue;
            }
            if board.get(rx as i32, y) {
                continue;
            }
            if !board.get(rx as i32, y + 1) {
                continue;
            }
            return Some((x as i32 + 1, y));
        }
        None
    }

    /// Mirror of [`well_known_tslot_left`].
    pub fn well_known_tslot_right<const N: usize>(board: &Board<N>) -> Option<(i32, i32)> {
        let heights = column_heights(board);
        let total = Board::<N>::total_height();
        for x in 2..WIDTH as usize {
            let y = heights[x] as i32;
            if heights[x - 1] as i32 >= y {
                continue;
            }
            if y < 1 || y + 1 >= total {
                continue;
            }
            let lx = x - 2;
            if !board.get(lx as i32, y - 1) {
                continue;
            }
            if board.get(lx as i32, y) {
                continue;
            }
            if !board.get(lx as i32, y + 1) {
                continue;
            }
            return Some((x as i32 - 1, y));
        }
        None
    }

    /// Places a virtual T into the slot at `(x, y)`; returns lines cleared.
    /// Returns `None` when the placement would overlap or clear nothing.
    pub fn virtual_t_fire<const N: usize>(board: &mut Board<N>, x: i32, y: i32) -> Option<u64> {
        if y + 2 >= Board::<N>::total_height() {
            return None;
        }
        let mv = Move::new(Piece::T, x, y, Rotation::South, Spin::None);
        for (cx, cy) in mv.cells() {
            if board.get(cx, cy) {
                return None;
            }
        }
        let lines = board.do_move(mv);
        if lines > 0 { Some(lines) } else { None }
    }

    /// Number of T's available soon: next 7 queue pieces plus the hold.
    pub fn t_available<const N: usize>(game: &SearchGame<N>) -> usize {
        let in_queue = game.queue.iter().take(7).any(|p| *p == Piece::T);
        let held = game.hold == Some(Piece::T);
        usize::from(in_queue) + usize::from(held)
    }

    #[cfg(test)]
    mod parity {
        use super::*;
        use rue_core::buffer::Buffer;
        use rue_core::game::QUEUE_SIZE;

        fn fill(board: &mut Board<8>, cells: &[(i32, i32)]) {
            for &(x, y) in cells {
                board.set(x, y);
            }
        }

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

        fn left_tsd_board() -> Board<8> {
            let mut board = Board::empty();
            fill(
                &mut board,
                &[
                    (0, 5),
                    (0, 6),
                    (1, 5),
                    (1, 6),
                    (2, 5),
                    (2, 6),
                    (3, 0),
                    (3, 1),
                    (3, 2),
                    (3, 3),
                    (3, 4),
                    (3, 5),
                    (4, 0),
                    (4, 1),
                    (4, 2),
                    (5, 5),
                    (5, 7),
                    (6, 5),
                    (6, 6),
                    (7, 5),
                    (7, 6),
                    (8, 5),
                    (8, 6),
                    (9, 5),
                    (9, 6),
                ],
            );
            board
        }

        fn right_tsd_board() -> Board<8> {
            let mut board = Board::empty();
            fill(
                &mut board,
                &[
                    (0, 5),
                    (0, 6),
                    (1, 5),
                    (1, 6),
                    (2, 5),
                    (2, 6),
                    (3, 5),
                    (3, 6),
                    (4, 5),
                    (4, 7),
                    (5, 0),
                    (5, 1),
                    (5, 2),
                    (6, 0),
                    (6, 1),
                    (6, 2),
                    (6, 3),
                    (6, 4),
                    (6, 5),
                    (7, 5),
                    (7, 6),
                    (8, 5),
                    (8, 6),
                    (9, 5),
                    (9, 6),
                ],
            );
            board
        }

        #[test]
        fn tslot_left_clears_two() {
            let board = left_tsd_board();
            assert_eq!(well_known_tslot_left::<8>(&board), Some((4, 6)));
            assert_eq!(well_known_tslot_right::<8>(&board), None);
            let mut probe = board;
            assert_eq!(virtual_t_fire::<8>(&mut probe, 4, 6), Some(2));
        }

        #[test]
        fn tslot_right_clears_two() {
            let board = right_tsd_board();
            assert_eq!(well_known_tslot_left::<8>(&board), None);
            assert_eq!(well_known_tslot_right::<8>(&board), Some((5, 6)));
            let mut probe = board;
            assert_eq!(virtual_t_fire::<8>(&mut probe, 5, 6), Some(2));
        }

        #[test]
        fn tslot_absent_on_flat_and_empty() {
            assert_eq!(well_known_tslot_left::<8>(&Board::empty()), None);
            assert_eq!(well_known_tslot_right::<8>(&Board::empty()), None);

            let mut flat = Board::empty();
            fill(
                &mut flat,
                &(0..10)
                    .flat_map(|x| (0..6).map(move |y| (x, y)))
                    .collect::<Vec<_>>(),
            );
            assert_eq!(well_known_tslot_left::<8>(&flat), None);
            assert_eq!(well_known_tslot_right::<8>(&flat), None);
        }

        #[test]
        fn t_available_counts_queue_and_hold() {
            let mut queue = Buffer::<Piece, { QUEUE_SIZE }>::new();
            queue.push(Piece::I);
            queue.push(Piece::T);
            let game = SearchGame {
                board: Board::empty(),
                queue,
                hold: Some(Piece::T),
                combo: None,
                b2b: None,
                incoming: 0,
            };
            assert_eq!(t_available::<8>(&game), 2);
        }

        #[test]
        fn t_available_zero_without_ts() {
            let mut queue = Buffer::<Piece, { QUEUE_SIZE }>::new();
            queue.push(Piece::I);
            queue.push(Piece::J);
            let game = SearchGame {
                board: Board::empty(),
                queue,
                hold: None,
                combo: None,
                b2b: None,
                incoming: 0,
            };
            assert_eq!(t_available::<8>(&game), 0);
        }
    }
}
