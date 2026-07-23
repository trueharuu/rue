//! Packed multi-band bitboard representation and board update primitives.

use std::cmp::Ordering;
use std::simd::Simd;

use crate::header::COL0;
use crate::header::COL9;
use crate::header::PCELLS;
use crate::header::PMASK;
use crate::header::TALL;
use crate::header::TLINES;
use crate::header::WIDTH;
use crate::header::dx_mask;
use crate::piece::Piece;
use crate::placement::Move;

/// A row-major banded bitboard. Each u64 represents a 10x6 band of the board with the 4 high-most bits left empty.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Board<const N: usize>(Simd<u64, N>);

impl<const N: usize> Board<N> {
    /// Empty board with all bits cleared.
    pub const EMPTY: Self = Self(Simd::splat(0));
    /// Height of the board in rows for this band count.
    pub const H: i32 = TLINES * N as i32;

    /// Raw access to the underlying SIMD vector.
    #[inline]
    #[must_use]
    pub fn vector(&self) -> &Simd<u64, N> {
        &self.0
    }

    /// Create a new board from a raw SIMD vector.
    #[inline]
    #[must_use]
    pub fn from_vector(v: Simd<u64, N>) -> Self {
        Self(v)
    }

    /// Inserts `lines` garbage rows onto the bottom of the board, shifting existing content up.
    /// Each garbage row has all columns filled except column `gap`.
    #[inline]
    pub fn insert_garbage(&mut self, lines: u32, gap_col: u32) {
        debug_assert!(lines > 0 && lines <= TLINES as u32 * N as u32);
        debug_assert!(gap_col < WIDTH as u32);

        let mut remaining = lines;
        while remaining > 0 {
            let band = (self.max_y() / TLINES) as usize;
            if band >= N {
                break;
            }

            let band_lines = std::cmp::min(remaining, TLINES as u32);
            self.push_garbage(band_lines as u8, gap_col as u8);
            remaining -= band_lines;
        }
    }

    /// Returns the board as a column-major bitboard.
    #[inline]
    #[must_use]
    pub fn as_cols(&self) -> [u64; WIDTH as usize] {
        let mut cols = [0u64; WIDTH as usize];
        let mut i = 0;
        while i < N {
            let mut j = 0;
            while j < TLINES {
                let mut k = 0;
                while k < WIDTH {
                    if self.0[i] & (1u64 << (j * WIDTH + k)) != 0 {
                        cols[k as usize] |= 1u64 << j;
                    }
                    k += 1;
                }
                j += 1;
            }
            i += 1;
        }
        cols
    }

    #[inline]
    #[must_use]
    /// Returns the height of column `col` in rows, or `0` if empty
    pub fn col_height(&self, col: usize) -> i32 {
        debug_assert!((0..WIDTH).contains(&(col as i32)));
        let mut h = 0;
        let mut i = 0;
        while i < N {
            let bits = self.0[i];
            if bits != 0 {
                let idx = 63 - bits.leading_zeros() as i32;
                h = i as i32 * TLINES + idx / WIDTH + 1;
            }
            i += 1;
        }
        h
    }

    #[inline]
    #[must_use]
    /// Returns `true` if any cell is occupied.
    pub fn any(&self) -> bool {
        let mut t = 0;
        let mut i = 0;
        while i < N {
            t |= self.0[i];
            i += 1;
        }

        t != 0
    }

    #[inline]
    #[must_use]
    /// Counts occupied cells across all bands.
    pub fn popcount(&self) -> u32 {
        let mut t = 0u32;
        let mut i = 0;
        while i < N {
            t += self.0[i].count_ones();
            i += 1;
        }
        t
    }

    #[inline]
    /// Sets one occupied cell at `(x, y)`.
    pub fn set(&mut self, x: i32, y: i32) {
        debug_assert!((0..WIDTH).contains(&x) && y >= 0 && y < Self::H);
        self.0[(y / TLINES) as usize] |= 1u64 << ((y % TLINES) * WIDTH + x);
    }

    #[inline]
    /// Sets multiple occupied cells.
    pub fn set_many(&mut self, cells: &[(i32, i32)]) {
        for &(x, y) in cells {
            self.set(x, y);
        }
    }

    #[inline]
    #[must_use]
    /// Returns whether cell `(x, y)` is occupied.
    pub fn get(&self, x: i32, y: i32) -> bool {
        debug_assert!((0..WIDTH).contains(&x) && y >= 0 && y < Self::H);
        self.0[(y / TLINES) as usize] & (1u64 << ((y % TLINES) * WIDTH + x)) != 0
    }

    #[inline]
    #[must_use]
    /// Builds a board where every row has column `x` set.
    pub fn col_mask(x: i32) -> Self {
        let mut b = Self::EMPTY;
        let mut i = 0;
        while i < Self::H {
            b.set(x, i);
            i += 1;
        }
        b
    }

    #[inline]
    #[must_use]
    /// Builds a board where every column has row `y` set.
    pub fn row_mask(y: i32) -> Self {
        let mut b = Self::EMPTY;
        let mut i = 0;
        while i < WIDTH {
            b.set(i, y);
            i += 1;
        }
        b
    }

    #[inline]
    #[must_use]
    /// Returns a translated board shifted by `(dx, dy)` with out-of-bounds bits dropped.
    pub fn shifted(&self, dx: i32, dy: i32) -> Self {
        let mut out = Simd::splat(0);

        match dy.cmp(&0) {
            Ordering::Equal => out = self.0,

            Ordering::Greater => {
                let q = ((dy - 1) / TLINES) as usize;
                let hi = ((((dy - 1) % TLINES) + 1) * WIDTH) as u32; // 10..=60
                let lo = 60 - hi;
                let mut i = 0;
                while i < N {
                    let mut w = 0u64;
                    if i >= q {
                        // hi can reach 60; split the shift so it stays < 64.
                        w |= (self.0[i - q] << (hi - 1)) << 1;
                    }
                    if i > q {
                        w |= self.0[i - q - 1] >> lo;
                    }
                    out[i] = w;
                    i += 1;
                }
            }

            Ordering::Less => {
                let q = ((-dy - 1) / TLINES) as usize;
                let lo = ((((-dy - 1) % TLINES) + 1) * WIDTH) as u32; // 10..=60
                let hi = 60 - lo;
                let mut i = 0;
                while i < N {
                    let mut w = 0u64;
                    if i + q < N {
                        w |= (self.0[i + q] >> (lo - 1)) >> 1;
                    }
                    if i + q + 1 < N {
                        w |= self.0[i + q + 1] << hi;
                    }
                    out[i] = w;
                    i += 1;
                }
            }
        }

        let m = dx_mask(dx);
        let mut i = 0;
        while i < N {
            let w = out[i];
            out[i] = match dx.cmp(&0) {
                Ordering::Greater => w << dx,
                Ordering::Less => w >> -dx,
                Ordering::Equal => w,
            } & m;
            i += 1;
        }

        Self(out)
    }

    #[inline]
    #[must_use]
    /// Returns a mask selecting full lines.
    pub fn line_clears(&self) -> Self {
        let mut out = Simd::splat(0);
        let mut i = 0;
        while i < N {
            let d = self.0[i];
            out[i] = d & ((d & !COL9) + COL0) & COL9;
            i += 1;
        }

        Self(out)
    }

    /// Clears all lines flagged in `lines` and compacts remaining rows downward.
    pub fn clear_lines(&mut self, lines: &Self) {
        let mut prefix = [0i32; N];
        let mut i = 0;
        while i + 1 < N {
            prefix[i + 1] = prefix[i] + lines.0[i].count_ones() as i32;
            i += 1;
        }

        let mut packed = [0u64; N];
        let mut i = 0;
        while i < N {
            let ld = self.0[i];
            let ll = lines.0[i];
            let mut p = 0u64;
            let mut dest = 0u32;
            let mut row = 0;
            while row < TLINES {
                let src = (row * WIDTH) as u32;
                if (ll >> (src + 9)) & 1 == 0 {
                    p |= ((ld >> src) & 0x3FF) << dest;
                    dest += WIDTH as u32;
                }
                row += 1;
            }
            packed[i] = p;
            i += 1;
        }

        let mut dest = 0;
        while dest < N {
            let mut result = 0u64;
            let mut src = dest;
            while src < N {
                let relative = ((src - dest) as i32 * TLINES) - prefix[src];
                if (0..TLINES).contains(&relative) {
                    result |= packed[src] << (relative * WIDTH);
                } else if relative < 0 && relative > -TLINES {
                    result |= packed[src] >> (-relative * WIDTH);
                }
                src += 1;
            }
            self.0[dest] = result & TALL;
            dest += 1;
        }
    }

    #[inline]
    /// Places a piece by explicit cell writes, applies line clear, and returns cleared line count.
    pub fn do_move(&mut self, placement: Move) -> u32 {
        self.set(placement.x(), placement.y());
        let cells = PCELLS[placement.piece() as usize][placement.rotation() as usize];
        self.set(
            placement.x() + i32::from(cells[0].0),
            placement.y() + i32::from(cells[0].1),
        );
        self.set(
            placement.x() + i32::from(cells[1].0),
            placement.y() + i32::from(cells[1].1),
        );
        self.set(
            placement.x() + i32::from(cells[2].0),
            placement.y() + i32::from(cells[2].1),
        );
        let clears = self.line_clears();
        if clears.any() {
            let n = clears.popcount();
            self.clear_lines(&clears);
            n
        } else {
            0
        }
    }

    #[inline]
    /// Places a piece via precomputed masks, applies line clear, and returns cleared line count.
    pub fn do_move_masked(&mut self, piece: Piece, rc: usize, x: i32, y: i32) -> u32 {
        let (lo, hi, boff, xb) = PMASK[piece as usize][rc][(y % TLINES) as usize];

        let s = (x - i32::from(xb)) as u32;
        let w = (y / TLINES + i32::from(boff)) as usize;
        self.0[w] |= lo << s;

        let mut probe = self.0[w] & ((self.0[w] & !COL9) + COL0) & COL9;

        if hi != 0 {
            self.0[w + 1] |= hi << s;
            probe |= self.0[w + 1] & ((self.0[w + 1] & !COL9) + COL0) & COL9;
        }

        if probe == 0 {
            debug_assert!(!self.line_clears().any());
            return 0;
        }

        let clears = self.line_clears();
        let count = clears.popcount();
        self.clear_lines(&clears);
        count
    }

    /// Pushes `count` garbage rows onto the bottom of the board, shifting existing content up.
    /// Each garbage row has all columns filled except column `gap`.
    pub fn push_garbage(&mut self, count: u8, gap: u8) {
        *self = self.shifted(0, i32::from(count));

        let row_mask = 0x3FFu64 & !(1u64 << gap);
        let mut i = 0;
        while i < count {
            let band = (i32::from(i) / TLINES) as usize;
            let offset = (i32::from(i) % TLINES) as u32;
            if band < N {
                self.0[band] |= row_mask << (offset * WIDTH as u32);
            }
            i += 1;
        }
    }

    #[inline]
    #[must_use]
    /// Returns one plus the highest occupied y-coordinate, or `0` if empty.
    pub fn max_y(&self) -> i32 {
        let mut i = N;
        while i > 0 {
            i -= 1;
            let bits = self.0[i];
            if bits != 0 {
                let idx = 63 - bits.leading_zeros() as i32;
                return i as i32 * TLINES + idx / WIDTH + 1;
            }
        }
        0
    }

    #[inline]
    #[must_use]
    /// Casts this board into a different band count, truncating or zero-extending as needed.
    pub fn cast<const M: usize>(&self) -> Board<M> {
        let mut out = Simd::splat(0);
        let mut i = 0;
        while i < M && i < N {
            out[i] = self.0[i];
            i += 1;
        }

        Board(out)
    }

    #[inline]
    /// Invokes `f(x, y)` for every occupied cell.
    pub fn for_each_set_bit(&self, mut f: impl FnMut(i32, i32)) {
        let mut i = 0;
        while i < N {
            let mut bits = self.0[i];
            while bits != 0 {
                let idx = bits.trailing_zeros() as i32;
                f(idx % WIDTH, i as i32 * TLINES + idx / WIDTH);
                bits &= bits - 1;
            }
            i += 1;
        }
    }
}

impl<const N: usize> std::ops::BitAnd for Board<N> {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        let o: &Self = &rhs;
        let mut out = Simd::splat(0);
        let mut i = 0;
        while i < N {
            out[i] = self.0[i] & o.0[i];
            i += 1;
        }

        Board(out)
    }
}

impl<const N: usize> std::ops::BitAndAssign for Board<N> {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl<const N: usize> std::ops::BitOr for Board<N> {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        let o: &Self = &rhs;
        let mut out = Simd::splat(0);
        let mut i = 0;
        while i < N {
            out[i] = self.0[i] | o.0[i];
            i += 1;
        }

        Board(out)
    }
}

impl<const N: usize> std::ops::BitOrAssign for Board<N> {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl<const N: usize> std::ops::BitXor for Board<N> {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        let o: &Self = &rhs;
        let mut out = Simd::splat(0);
        let mut i = 0;
        while i < N {
            out[i] = self.0[i] ^ o.0[i];
            i += 1;
        }
        Board(out)
    }
}

impl<const N: usize> std::ops::BitXorAssign for Board<N> {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl<const N: usize> std::ops::Not for Board<N> {
    type Output = Self;

    #[inline]
    fn not(mut self) -> Self::Output {
        for z in 0..N {
            self.0[z] = TALL & !self.0[z];
        }
        self
    }
}
