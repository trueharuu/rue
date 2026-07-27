//! Controller inputs and finesse sequences for a given placement.

use std::fmt::Debug;

/// An individual controller input in a path sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// No input.
    None,
    /// Move one cell left.
    ShiftLeft,
    /// Move one cell right.
    ShiftRight,
    /// Move as far left as possible.
    DasLeft,
    /// Move as far right as possible.
    DasRight,
    /// Rotate clockwise.
    RotateCW,
    /// Rotate counterclockwise.
    RotateCCW,
    /// Rotate 180 degrees.
    RotateFlip,
    /// Drops the piece downwards.
    ///
    /// When [`Handling::inf_sdf`] is false, lowers the piece by exactly 1 cell (if possible).
    /// Otherwise, lowers the piece as far as possible.
    SoftDrop,
    /// Instantly drop to lowest valid position.
    HardDrop,
}

/// The maxmimum number of inputs for a single finesse sequence.
pub const FINESSE_BUFFER: usize = 16;

/// A sequence of controller inputs that reach a target placement.
#[derive(Clone, Copy)]
pub struct Finesse([Input; FINESSE_BUFFER], usize);

impl Debug for Finesse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Finesse")
            .field(&&self.0[..self.1])
            .finish()
    }
}

impl Finesse {
    /// Creates a new, empty finesse buffer.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self([Input::None; FINESSE_BUFFER], 0)
    }

    /// Returns a copy of the sequence with a single [`Input`] at the end.
    ///
    /// Panics if the insertion would overflow the buffer.
    #[inline]
    #[must_use]
    pub fn append(mut self, input: Input) -> Self {
        assert!(self.1 < FINESSE_BUFFER);
        self.0[self.1] = input;
        self.1 += 1;
        self
    }

    /// Returns the length of the buffer.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.1
    }

    /// Whether the current buffer is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.1 == 0
    }

    /// Returns the capacity of the buffer.
    /// This is always exactly [`FINESSE_BUFFER`].
    #[inline]
    #[must_use]
    pub const fn capacity(&self) -> usize {
        FINESSE_BUFFER
    }

    /// Returns an iterator on all the non-none inputs in this sequence.
    #[inline]
    #[must_use]
    pub fn into_iter(&self) -> FinesseIter<'_> {
        FinesseIter(self, 0)
    }
}

impl Default for Finesse {
    fn default() -> Self {
        Self::new()
    }
}

/// An iterator over [`Input`]s, ignoring [`Input::None`].
pub struct FinesseIter<'a>(&'a Finesse, usize);

impl Iterator for FinesseIter<'_> {
    type Item = Input;
    fn next(&mut self) -> Option<Self::Item> {
        if self.1 < self.0.len() {
            let v = self.0.0[self.1];
            self.1 += 1;
            Some(v)
        } else {
            None
        }
    }
}