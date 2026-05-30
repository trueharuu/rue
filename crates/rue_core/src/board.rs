use std::fmt;

use crate::{coordinates::Coordinates, piece::Piece, placement::{Move, is_ok_move}};

pub type Bitboard = u64;
pub const COL_NB: usize = 10;
pub const ROW_NB: usize = 40;

pub const BOARD_HEIGHT: usize = 40;
pub const FULL_ROW: u16 = (1 << COL_NB) - 1; // 0x3FF

#[derive(Copy, Debug, PartialEq, Eq)]
pub struct Board {
    pub rows: [u16; BOARD_HEIGHT],
    pub cols: [Bitboard; COL_NB],
}

impl Board {
    pub fn new() -> Self {
        Board {
            rows: [0; BOARD_HEIGHT],
            cols: [0; COL_NB],
        }
    }

    pub fn set(&mut self, x: usize, y: usize) {
        debug_assert!(is_ok_x(x as i32));
        debug_assert!(is_ok_y(y as i32));
        self.rows[y] |= 1 << x;
        self.cols[x] |= 1u64 << y;
    }

    pub fn occupied(&self, x: i32, y: i32) -> bool {
        let yu = y as usize;
        if yu >= BOARD_HEIGHT {
            return false;
        }
        self.rows[yu] & (1 << x) != 0
    }

    pub fn occupied_coord(&self, c: &Coordinates) -> bool {
        self.occupied(c.x as i32, c.y as i32)
    }

    pub fn obstructed(&self, x: i32, y: i32) -> bool {
        !is_ok_x(x) || !is_ok_y(y) || self.occupied(x, y)
    }

    pub fn obstructed_coord(&self, c: &Coordinates) -> bool {
        self.obstructed(c.x as i32, c.y as i32)
    }

    pub fn obstructed_move(&self, m: &Move) -> bool {
        let pc = m.cells();
        let off = Coordinates::new(m.x(), m.y());
        self.obstructed_coord(&off)
            || self.obstructed_coord(&(pc[0] + off))
            || self.obstructed_coord(&(pc[1] + off))
            || self.obstructed_coord(&(pc[2] + off))
    }

    pub fn legal_lock_placement(&self, m: &Move) -> bool {
        if !is_ok_move(m) || self.obstructed_move(m) {
            return false;
        }

        let pc = m.cells();
        let off = Coordinates::new(m.x(), m.y());
        let cells = [off, pc[0] + off, pc[1] + off, pc[2] + off];

        cells
            .iter()
            .any(|c| self.obstructed(c.x as i32, c.y as i32 - 1))
    }

    /// Build a column bitboard on-the-fly from row data.
    /// Bit y of result is set iff cell (x, y) is occupied.
    pub fn col(&self, x: usize) -> Bitboard {
        let mask = 1u16 << x;
        let mut result: Bitboard = 0;
        for y in 0..BOARD_HEIGHT {
            if self.rows[y] & mask != 0 {
                result |= 1u64 << y;
            }
        }
        result
    }

    /// Return cached column bitboards — O(1).
    /// Maintained in sync with rows by place/clear_lines/spawn_garbage/clear.
    #[inline(always)]
    pub fn compute_cols(&self) -> [Bitboard; COL_NB] {
        self.cols
    }

    /// Rebuild cols cache from rows. Used after bulk mutations (clear_lines, spawn_garbage).
    fn rebuild_cols(&mut self) {
        self.cols = [0; COL_NB];
        for y in 0..BOARD_HEIGHT {
            let row = self.rows[y];
            if row == 0 {
                continue;
            }
            let mut bits = row as u64;
            while bits != 0 {
                let x = bits.trailing_zeros() as usize;
                self.cols[x] |= 1u64 << y;
                bits &= bits - 1;
            }
        }
    }

    pub fn empty(&self) -> bool {
        self.rows.iter().all(|&r| r == 0)
    }

    pub fn line_clears(&self) -> Bitboard {
        let mut result: Bitboard = 0;
        for y in 0..BOARD_HEIGHT {
            if self.rows[y] == FULL_ROW {
                result |= 1u64 << y;
            }
        }
        result
    }

    pub fn clear(&mut self) {
        self.rows = [0; BOARD_HEIGHT];
        self.cols = [0; COL_NB];
    }

    /// Remove filled lines and compact remaining rows down.
    pub fn clear_lines(&mut self, l: Bitboard) {
        debug_assert!(l != 0);
        let mut write = 0usize;
        for read in 0..BOARD_HEIGHT {
            if l & (1u64 << read) == 0 {
                self.rows[write] = self.rows[read];
                write += 1;
            }
        }
        for y in write..BOARD_HEIGHT {
            self.rows[y] = 0;
        }
        self.rebuild_cols();
    }

    pub fn place(&mut self, m: &Move) {
        let pc = m.cells();
        let x = m.x();
        let y = m.y();

        let xu = x as usize;
        let yu = y as usize;
        if xu < COL_NB && yu < BOARD_HEIGHT {
            self.rows[yu] |= 1 << x;
            self.cols[xu] |= 1u64 << y;
        }

        for i in 0..3 {
            let cx = (pc[i].x as i32 + x) as usize;
            let cy = (pc[i].y as i32 + y) as usize;
            if cx < COL_NB && cy < BOARD_HEIGHT {
                self.rows[cy] |= 1 << cx;
                self.cols[cx] |= 1u64 << cy;
            }
        }
    }

    pub fn spawn_garbage(&mut self, lines: i32, x: i32) {
        debug_assert!(is_ok_x(x));
        debug_assert!(lines > 0);
        let n = lines as usize;
        for y in (n..BOARD_HEIGHT).rev() {
            self.rows[y] = self.rows[y - n];
        }
        let garbage_row = FULL_ROW & !(1u16 << x);
        for y in 0..n {
            self.rows[y] = garbage_row;
        }
        self.rebuild_cols();
    }

    pub fn do_move(&mut self, m: &Move) -> i32 {
        if !self.legal_lock_placement(m) {
            return 0;
        }

        self.place(m);
        let clears = self.line_clears();
        if clears == 0 {
            return 0;
        }

        self.clear_lines(clears);
        popcount(clears) as i32
    }

    /// Max occupied row index + 1 (= height)
    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(|&r| r == 0)
    }

    pub fn height(&self) -> u32 {
        for y in (0..BOARD_HEIGHT).rev() {
            if self.rows[y] != 0 {
                return y as u32 + 1;
            }
        }
        0
    }

    pub fn to_string_with_move(&self, m: &Move) -> String {
        let mut output = self.to_string();
        if !self.obstructed_move(m) {
            let lines: i32 = 20;
            let pc = m.cells();
            let x = m.x();
            let y = m.y();
            for i in 0..4usize {
                let inverse_y = lines - if i == 0 { y } else { pc[i - 1].y as i32 + y };
                if inverse_y < 0 {
                    continue;
                }
                let cell_x = if i == 0 { x } else { pc[i - 1].x as i32 + x };
                let idx = (inverse_y * 86 + cell_x * 4 + 47) as usize;
                if idx < output.len() {
                    unsafe {
                        output.as_bytes_mut()[idx] = b'.';
                    }
                }
            }
        }
        output
    }

    pub fn row(&self, y: usize) -> u16 {
        self.rows[y]
    }
}

pub const fn is_ok_x(x: i32) -> bool {
    x >= 0 && x < COL_NB as i32
}

pub const fn is_ok_y(y: i32) -> bool {
    y >= 0 && y < ROW_NB as i32
}

impl Clone for Board {
    fn clone(&self) -> Self { *self }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lines = 20;
        let mut output = String::with_capacity((lines + 1) * 86 + 44);
        output.push_str("\n +---+---+---+---+---+---+---+---+---+---+\n");
        for y in (0..=lines).rev() {
            for x in 0..COL_NB {
                output.push_str(" | ");
                output.push(if self.rows[y] & (1 << x) != 0 {
                    '#'
                } else {
                    ' '
                });
            }
            output.push_str(" |\n +---+---+---+---+---+---+---+---+---+---+\n");
        }
        write!(f, "{}", output)
    }
}

pub const fn clz(v: Bitboard) -> u32 {
    if v != 0 { v.leading_zeros() } else { 64 }
}

pub const fn ctz(v: Bitboard) -> u32 {
    debug_assert!(v != 0);
    v.trailing_zeros()
}

pub const fn popcount(v: Bitboard) -> u32 {
    v.count_ones()
}

pub const fn bitlen(v: Bitboard) -> u32 {
    64 - clz(v)
}


pub fn render_vs(
    board_a: &Board,
    board_b: &Board,
    placement_a: Option<Move>,
    placement_b: Option<Move>,
) {
    println!("\u{250c}{}\u{252c}{}\u{2510}", "\u{2500}".repeat(20), "\u{2500}".repeat(20));
    for y in (0..20).rev() {
        print!("\u{2502}");
        for x in 0..20 {
            if x == 10 {
                print!("\u{2502}");
                // continue;
            }
            let b = if x < 10 { board_a } else { board_b };
            let bx = x % 10;
            let cell = (b.cols[bx] >> y) & 1;
            // println!("({x}, {y}) = {cell}");
            if cell != 0 {
                print!("\x1b[48;2;127;127;127m  \x1b[0m")
            } else {
                let pp = if x < 10 { &placement_a } else { &placement_b };
                let px = x % 10;
                if let Some(p) = pp
                    && p.blocks().contains(&Coordinates{ x: px as i8, y: y as i8 })
                {
                    print!("{}", draw_cell(p.piece()));
                } else {
                    print!("\x1b[0m  \x1b[0m")
                }
            }
        }
        println!("\u{2502}");
    }
        println!("\u{2514}{}\u{2534}{}\u{2518}", "\u{2500}".repeat(20), "\u{2500}".repeat(20));
}

pub fn draw_cell(piece: Piece) -> &'static str {
    match piece {
        Piece::Z => "\x1b[48;2;255;127;127m  \x1b[0m",
        Piece::L => "\x1b[48;2;255;192;127m  \x1b[0m",
        Piece::O => "\x1b[48;2;255;255;127m  \x1b[0m",
        Piece::S => "\x1b[48;2;127;255;127m  \x1b[0m",
        Piece::I => "\x1b[48;2;127;255;255m  \x1b[0m",
        Piece::J => "\x1b[48;2;127;127;255m  \x1b[0m",
        Piece::T => "\x1b[48;2;255;127;255m  \x1b[0m",
    }
}