use std::{fmt::Debug};

use crate::piece::Mino;

pub type Q = u64;

#[derive(Clone, PartialEq, Eq, Copy, Default, Hash, PartialOrd, Ord)]
pub struct Queue(pub Q);

impl Queue {
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn one(piece: Mino) -> Self {
        Self(piece as Q + 1)
    }

    pub fn get(self, index: usize) -> Option<Mino> {
        let shifted = self.0 >> (index * 3);
        let value = shifted & 7;
        if value == 0 {
            None
        } else {
            Some(unsafe { std::mem::transmute::<u8, Mino>(value as u8 - 1) })
        }
    }

    pub fn get_unchecked(self, index: usize) -> Mino {
        let shifted = self.0 >> (index * 3);
        let value = shifted & 7;
        unsafe { std::mem::transmute::<u8, Mino>(value as u8 - 1) }
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn len(self) -> Q {
        let highest_one = Q::BITS - self.0.leading_zeros();
        ((highest_one + 2) / 3) as Q
    }

    pub fn push_last(self, piece: Mino) -> Self {
        let next_slot = self.len() * 3;
        let new = ((piece as Q) + 1) << next_slot;
        Self(self.0 | new)
    }
}

impl Debug for Queue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.len() {
            write!(f, "{:?}", self.get(i as usize).unwrap())?;
        }
        Ok(())
    }
}

impl FromIterator<Mino> for Queue {
    fn from_iter<I: IntoIterator<Item = Mino>>(iter: I) -> Self {
        let mut queue = Self::empty();
        for piece in iter {
            queue = queue.push_last(piece);
        }
        queue
    }
}
