//! Randomiser utilities.
use crate::piece::Piece;

/// A piece randomiser. Follows the implementation in TETR.IO as exactly as possible.
pub struct Rng {
    /// The current seed of the randomiser.
    pub seed: i32,
}

/// A kind of RNG.
pub enum RngKind {
    /// 7-bag randomiser. Always generates a permutation of `ZLOSIJT`.
    Bag7,
}

impl RngKind {
    /// The selectable pool of pieces for this randomizer.
    pub fn slice(self) -> Vec<Piece> {
        match self {
            Self::Bag7 => vec![
                Piece::Z,
                Piece::L,
                Piece::O,
                Piece::S,
                Piece::I,
                Piece::J,
                Piece::T,
            ],
        }
    }
}

impl Rng {
    /// Create a new instance of the given RNG with a seed based on the current time.
    pub fn new() -> Self {
        let now = std::time::SystemTime::now().elapsed().unwrap();
        Self::new_seeded((now.as_nanos() % 2147483647) as i32)
    }

    /// Create a new instance of the given RNG with a set seed.
    pub fn new_seeded(mut seed: i32) -> Self {
        if seed <= 0 {
            seed += 2147483646;
        }

        Self { seed }
    }

    /// Advances the randomiser state.
    pub const fn next(&mut self) -> i32 {
        self.seed = 16807 * self.seed % 2147483647;
        self.seed
    }

    /// Advances the randomiser state, and returns it as a float within `[0, 1)`.
    pub const fn next_float(&mut self) -> f64 {
        (self.next() - 1) as f64 / 2147483647.0
    }

    /// Randomises an array in-place with a Fisher-Yates shuffle.
    pub fn shuffle_array<T>(&mut self, slice: &mut [T]) {
        if slice.is_empty() {
            return;
        }

        let mut i = slice.len() - 1;
        while i != 0 {
            let r = f64::floor(self.next_float() * (i as f64 + 1.0)) as usize;
            slice.swap(i, r);
            i -= 1;
        }
    }
}
