//! Input path-finding for placements.

use smallvec::SmallVec;

use rue_core::board::Board;
use rue_core::game::ruleset::Ruleset;
use rue_core::placement::Move;

/// An individual controller input in a path sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
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
    /// Move as far down as possible.
    SoftDrop,
    /// Instantly drop to lowest valid position.
    HardDrop,
}

/// A sequence of controller inputs that reach a target placement.
#[derive(Debug)]
pub struct Inputs(pub SmallVec<[Input; 16]>);

/// Finds a sequence of inputs that reaches a target placement from the spawn position.
///
/// Returns an empty sequence if the target is unreachable.
#[must_use]
pub fn get_input<const N: usize>(
    board: &Board<N>,
    target: Move,
    ruleset: &Ruleset,
    finesse: bool,
    force: bool,
) -> Inputs {
    Inputs(SmallVec::new())
}
