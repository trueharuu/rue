use std::{
    ops::{BitAnd, BitOr, BitXor, Index, IndexMut, Not},
    str::FromStr,
};

use crate::{piece_location::PieceLocation, util::clear_lines};

#[derive(Debug, Clone)]
pub struct Board {
    pub cols: [u64; 10],
}

impl Board {
    pub fn new() -> Self {
        Self { cols: [0; 10] }
    }

    pub fn col_heights(&self) -> [i8; 10] {
        let mut heights = [0i8; 10];
        for x in 0..10 {
            heights[x] = 64 - self.cols[x].leading_zeros() as i8;
        }
        heights
    }

    pub fn fold_and(&self) -> u64 {
        self.cols.iter().fold(!0, |a, &b| a & b)
    }

    pub fn fold_or(&self) -> u64 {
        self.cols.iter().fold(!0, |a, &b| a | b)
    }

    pub fn fold_xor(&self) -> u64 {
        self.cols.iter().fold(!0, |a, &b| a ^ b)
    }

    pub fn add_garbage(&mut self, garb_col: usize, lines: u16) {
        for x in 0..10 {
            self.cols[x] = if x == garb_col {
                self.cols[x] << lines
            } else {
                !(!self.cols[x] << lines)
            };
        }
    }

    pub fn put_piece(&mut self, loc: &PieceLocation) {
        for &(x, y) in &loc.blocks() {
            self.cols[x as usize] |= 1 << y;
        }
    }

    pub fn remove_lines(&mut self) -> u64 {
        let lines = self.fold_and();
        for c in &mut self.cols {
            clear_lines(c, lines);
        }
        lines
    }

    pub fn obstructed(&self, loc: &PieceLocation) -> bool {
        for (x, y) in loc.blocks() {
            if x < 0 || x > 9 || y < 0 {
                continue;
            }
            if self.cols[x as usize] & (1 << y) > 0 {
                return true;
            }
        }
        false
    }

    pub fn distance_to_ground(&self, loc: &PieceLocation) -> i8 {
        loc.blocks()
            .iter()
            .map(|&(x, y)| {
                if y == 0 {
                    0
                } else {
                    (!self.cols[x as usize] << (64 - y)).leading_ones() as i8
                }
            })
            .min()
            .unwrap()
    }

    pub fn max_height(&self) -> i8 {
        let heights = self.cols.map(|c| 64 - c.leading_zeros() as i8);
        heights.into_iter().max().unwrap_or(0)
    }

    pub fn height_at(&self, x: usize) -> i8 {
        64 - self.cols[x].leading_zeros() as i8
    }

    pub fn get(&self, x: usize, y: usize) -> bool {
        self.cols[x] & (1 << y) != 0
    }

    pub fn get_row(&self, y: usize) -> u64 {
        self.cols.map(|c| c >> y).iter().fold(0, |a, b| a | b)
    }
}

impl FromStr for Board {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut b = Board::new();

        for (y, line) in s.split('|').enumerate() {
            for (x, c) in line.chars().enumerate() {
                if c == 'X' {
                    b.cols[x] |= 1 << y;
                }
            }
        }
        Ok(b)
    }
}

impl Index<usize> for Board {
    type Output = u64;
    fn index(&self, index: usize) -> &Self::Output {
        &self.cols[index]
    }
}

impl IndexMut<usize> for Board {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.cols[index]
    }
}

impl Not for Board {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self {
            cols: self.cols.map(|x| !x),
        }
    }
}

impl BitXor for Board {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self {
            cols: std::array::from_fn(|x| self.cols[x] ^ rhs.cols[x]),
        }
    }
}

impl BitOr for Board {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            cols: std::array::from_fn(|x| self.cols[x] | rhs.cols[x]),
        }
    }
}

impl BitAnd for Board {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            cols: std::array::from_fn(|x| self.cols[x] & rhs.cols[x]),
        }
    }
}
