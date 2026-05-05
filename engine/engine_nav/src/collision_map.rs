use std::ops::{BitAnd, BitOr, BitXor, Index, IndexMut, Not};

use engine_core::{board::Board, piece::Mino, piece_location::LUT, rotation::Rotation};

#[derive(Clone, Debug)]
pub struct CollisionMap {
    pub cols: [u64; 10],
}

impl CollisionMap {
    #[inline(always)]
    pub fn new(board: &Board, piece: Mino, rotation: Rotation) -> Self {
        let mut obstructed = [0u64; 10];
        for (dx, dy) in LUT[piece as usize][rotation as usize] {
            for x in 0..10usize {
                // println!("{x} {dx}");
                let c = board
                    .cols
                    .get(x.wrapping_add(dx as usize))
                    .copied()
                    .unwrap_or(!0);
                let c = match dy.is_negative() {
                    true => !(!c << -dy),
                    false => c >> dy,
                };
                obstructed[x as usize] |= c;
            }
        }
        Self { cols: obstructed }
    }

    pub fn obstructed(&self, x: i8, y: i8) -> bool {
        if x < 0 || x > 9 || y < 0 {
            return true;
        }
        self[x as usize] & (1 << y) > 0
    }

    pub fn as_board(&self) -> Board {
        Board { cols: self.cols }
    }
}

impl Index<usize> for CollisionMap {
    type Output = u64;
    fn index(&self, index: usize) -> &Self::Output {
        &self.cols[index]
    }
}

impl IndexMut<usize> for CollisionMap {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.cols[index]
    }
}

impl Not for CollisionMap {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self {
            cols: self.cols.map(|x| !x),
        }
    }
}

impl BitXor for CollisionMap {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self {
            cols: std::array::from_fn(|x| self.cols[x] ^ rhs.cols[x]),
        }
    }
}

impl BitOr for CollisionMap {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            cols: std::array::from_fn(|x| self.cols[x] | rhs.cols[x]),
        }
    }
}

impl BitAnd for CollisionMap {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            cols: std::array::from_fn(|x| self.cols[x] & rhs.cols[x]),
        }
    }
}
