//! Random number generators.

use std::time::SystemTime;

/// TETR.IO's random number generator.
#[derive(Clone, Copy)]
pub struct Rng {
    seed: i32,
}

impl Rng {
    /// Create a new instance of the given RNG with a seed based on the current time.
    ///
    /// # Panics
    /// Panics if the [`SystemTime`] used is later than the current system time.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        let now = SystemTime::now().elapsed().unwrap();
        Self::new_seeded((now.as_nanos() % 2_147_483_647) as i32)
    }

    /// Create a new instance of the given RNG with a set seed.
    #[inline]
    #[must_use]
    pub const fn new_seeded(mut seed: i32) -> Self {
        if seed <= 0 {
            seed += 2_147_483_646;
        }

        Self { seed }
    }

    /// Advances the randomiser state.
    #[inline]
    #[must_use]
    pub const fn next(&mut self) -> i32 {
        self.seed = (self.seed as f64 * 16_807.0 % 2_147_483_647.0) as i32;
        self.seed
    }

    /// Advances the randomiser state, and returns the seed as a float within `[0, 1)`.
    #[inline]
    #[must_use]
    pub const fn next_float(&mut self) -> f64 {
        (self.next() - 1) as f64 / 2_147_483_647.0
    }

    /// Randomises an array in-place with a Fisher-Yates shuffle.
    #[inline]
    pub const fn shuffle_array<T>(&mut self, slice: &mut [T]) {
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

impl Default for Rng {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
