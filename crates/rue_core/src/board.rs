//! A row-major banded bitboard representation for a 10-column board.
//! Each 10x6 band is represented by a [`u64`] value.
//!
//! The `Board` struct creates an empty board, gets and sets bits at specific
//! coordinates, and accesses the board as a SIMD vector.
//!
//! The board is efficient for operations that SIMD can vectorize.
//! It provides fast manipulation of the board state.
//! It supports the features in [`crate`].

use std::cmp::Ordering;
use std::ops::BitAnd;
use std::ops::BitAndAssign;
use std::ops::BitOr;
use std::ops::BitOrAssign;
use std::ops::BitXor;
use std::ops::BitXorAssign;
use std::ops::Not;
use std::simd::Simd;
use std::simd::num::SimdUint;

use crate::data::CELLS;
use crate::data::PMASK;
use crate::header::COL0;
use crate::header::COL9;
use crate::header::TALL;
use crate::header::TLINES;
use crate::header::WIDTH;
use crate::header::dx_mask;
use crate::piece::Piece;
use crate::placement::Move;

/// A constant-banded row-major bitboard. Each [`Column`] represents a single
/// column of [`HEIGHT`] rows, with the least-significant bit at the bottom.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Board<const N: usize>(pub Simd<u64, N>);

impl<const N: usize> Board<N> {
    /// Creates a new, empty board.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self(Simd::splat(0))
    }

    #[inline]
    #[must_use]
    pub const fn total_height() -> i32 {
        N as i32 * TLINES
    }

    #[inline]
    #[must_use]
    pub fn height(&self) -> i32 {
        let mut i = N;
        while i > 0 {
            i -= 1;
            let bits = self.0[i];
            if bits != 0 {
                let idx = 64 - 1 - bits.leading_zeros() as i32;
                return i as i32 * TLINES + idx / WIDTH + 1;
            }
        }

        0
    }

    #[inline]
    #[must_use]
    pub fn any(&self) -> bool {
        self.0.reduce_or() != 0
    }

    #[inline]
    #[must_use]
    pub fn popcount(&self) -> u64 {
        self.0.count_ones().reduce_sum()
    }

    #[inline]
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> bool {
        let band = (y / TLINES) as usize;
        let row = (y % TLINES) as u32;
        let bit_index = (row * WIDTH as u32 + x as u32) as usize;
        let mask = 1u64 << bit_index;
        (self.0[band] & mask) != 0
    }

    pub fn set(&mut self, x: i32, y: i32) {
        let band = (y / TLINES) as usize;
        assert!(
            band < N,
            "set OOB: x={x} y={y} band={band} N={N} top={}",
            Self::total_height()
        );
        let row = (y % TLINES) as u32;
        let bit_index = (row * WIDTH as u32 + x as u32) as usize;
        let mask = 1u64 << bit_index;

        self.0[band] |= mask;
    }

    pub fn clear(&mut self, x: i32, y: i32) {
        let band = (y / TLINES) as usize;
        let row = (y % TLINES) as u32;
        let bit_index = (row * WIDTH as u32 + x as u32) as usize;
        let mask = !(1u64 << bit_index);

        self.0[band] &= mask;
    }

    #[inline]
    #[must_use]
    pub fn shifted(&self, dx: i32, dy: i32) -> Self {
        let (lo, hi, s): (Simd<u64, N>, Simd<u64, N>, u32) = match dy.cmp(&0) {
            Ordering::Equal => (self.0, Simd::splat(0), 0),
            Ordering::Greater => {
                let q = ((dy - 1) / TLINES) as usize;
                let s = ((((dy - 1) % TLINES) + 1) * WIDTH) as u32;
                let src = self.0.to_array();
                let mut lo_w = [0u64; N];
                let mut hi_w = [0u64; N];
                if q < N {
                    lo_w[q..].copy_from_slice(&src[..N - q]);
                }
                if q + 1 < N {
                    hi_w[q + 1..].copy_from_slice(&src[..N - q - 1]);
                }
                (Simd::from_array(lo_w), Simd::from_array(hi_w), s)
            }
            Ordering::Less => {
                let q = ((-dy - 1) / TLINES) as usize;
                let lo_bits = ((((-dy - 1) % TLINES) + 1) * WIDTH) as u32;
                let s = (TLINES * WIDTH) as u32 - lo_bits;
                let src = self.0.to_array();
                let mut lo_w = [0u64; N];
                let mut hi_w = [0u64; N];
                if q < N {
                    lo_w[..N - q].copy_from_slice(&src[q..]);
                }
                if q + 1 < N {
                    hi_w[..N - q - 1].copy_from_slice(&src[q + 1..]);
                }
                // The high word is returned first for a downward shift.
                // This is the reverse of the upward case above.
                (Simd::from_array(hi_w), Simd::from_array(lo_w), s)
            }
        };

        let mut result = if s == 0 {
            lo
        } else {
            // Shift in two steps to avoid a shift-by-64 overflow.
            (lo << Simd::splat(u64::from(s - 1)) << Simd::splat(1u64))
                | (hi >> Simd::splat(u64::from(TLINES as u32 * WIDTH as u32 - s)))
        };

        let m = dx_mask(dx);

        result = match dx.cmp(&0) {
            Ordering::Equal => result,
            Ordering::Greater => result << Simd::splat(dx as u64),
            Ordering::Less => result >> Simd::splat((-dx) as u64),
        } & Simd::splat(m);

        Self(result)
    }

    /// Shifts the entire board left by 1 column.
    #[inline]
    #[must_use]
    pub fn shl(&self) -> Self {
        Self((self.0 >> Simd::splat(1)) & Simd::splat(dx_mask(-1)))
    }

    /// Shifts the entire board right by 1 column.
    #[inline]
    #[must_use]
    pub fn shr(&self) -> Self {
        Self((self.0 << Simd::splat(1)) & Simd::splat(dx_mask(1)))
    }

    /// Casts this board into a different band count.
    /// Truncates or zero-extends as needed.
    #[inline]
    #[must_use]
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

    /// Clears all lines flagged in `lines` and compacts remaining rows
    /// downward.
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
    #[must_use]
    pub fn necessary_bands(&self) -> usize {
        let mut i = N;
        while i > 0 {
            i -= 1;
            if self.0[i] != 0 {
                return i + 1;
            }
        }

        0
    }

    /// Places a piece by explicit cell writes.
    /// Applies the line clear and returns the number of cleared lines.
    #[inline]
    pub fn do_move(&mut self, placement: Move) -> u64 {
        if placement.y() + 2 >= Self::total_height() {
            eprintln!("do_move: {placement:?} top={}", Self::total_height());
        }
        self.set(placement.x(), placement.y());
        let cells = CELLS[placement.piece() as usize][placement.rotation() as usize];
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
    /// Places a piece via precomputed masks.
    /// Applies the line clear and returns the number of cleared lines.
    pub fn do_move_masked(&mut self, piece: Piece, rc: usize, x: i32, y: i32) -> u64 {
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

    /// Returns an iterator over the positions of filled cells in the board.
    #[inline]
    #[must_use]
    pub const fn iter(&self) -> BoardIter<'_, N> {
        BoardIter {
            board: self,
            x: 0,
            y: 0,
        }
    }
}

impl<'a, const N: usize> IntoIterator for &'a Board<N> {
    type Item = (i32, i32);
    type IntoIter = BoardIter<'a, N>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over filled positions within a [`Board`].
pub struct BoardIter<'a, const N: usize> {
    board: &'a Board<N>,
    x: i32,
    y: i32,
}

impl<const N: usize> Iterator for BoardIter<'_, N> {
    type Item = (i32, i32);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.y < Board::<N>::total_height() {
            while self.x < WIDTH {
                if self.board.get(self.x, self.y) {
                    let pos = (self.x, self.y);
                    self.x += 1;
                    return Some(pos);
                }

                self.x += 1;
            }

            self.x = 0;
            self.y += 1;
        }

        None
    }
}

impl<const N: usize> BitAnd for Board<N> {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl<const N: usize> BitOr for Board<N> {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl<const N: usize> BitXor for Board<N> {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl<const N: usize> BitAndAssign for Board<N> {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl<const N: usize> BitOrAssign for Board<N> {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl<const N: usize> BitXorAssign for Board<N> {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl<const N: usize> Not for Board<N> {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self(Simd::splat(TALL) & !self.0)
    }
}
