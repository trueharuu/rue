//! Controller inputs and finesse sequences for a given placement.

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
    /// When [`rue_core::rule::Rule::inf_sdf`] is false, lowers the piece by exactly 1
    /// cell (if possible). Otherwise, lowers the piece as far as possible.
    SoftDrop,
    /// Instantly drop to lowest valid position.
    HardDrop,
}
