//! ANSI terminal rendering helpers for boards and placements.

use crate::{board::Board, header::WIDTH, piece::Piece, placement::Move};

#[must_use]
/// Returns the ANSI glyph used for a filled board cell.
pub fn cell() -> &'static str {
    "\x1b[48;2;127;127;127m  \x1b[0m"
}

#[must_use]
/// Returns the ANSI glyph used for an empty board cell.
pub fn empty_cell() -> &'static str {
    "\x1b[0m  \x1b[0m"
}

#[must_use]
/// Returns the ANSI glyph used to render a cell for a specific piece color.
pub fn colored_cell(p: Piece) -> &'static str {
    match p {
        Piece::I => "\x1b[48;2;127;191;255m  \x1b[0m",
        Piece::J => "\x1b[48;2;127;127;255m  \x1b[0m",
        Piece::L => "\x1b[48;2;255;191;127m  \x1b[0m",
        Piece::O => "\x1b[48;2;255;255;127m  \x1b[0m",
        Piece::S => "\x1b[48;2;127;255;127m  \x1b[0m",
        Piece::Z => "\x1b[48;2;255;127;127m  \x1b[0m",
        Piece::T => "\x1b[48;2;255;127;255m  \x1b[0m",
    }
}

/// Horizontal border segment.
pub const HORIZ: &str = "──";
/// Top-left border corner.
pub const TOP_LEFT: &str = "┌";
/// Top-right border corner.
pub const TOP_RIGHT: &str = "┐";
/// Bottom-left border corner.
pub const BOTTOM_LEFT: &str = "└";
/// Bottom-right border corner.
pub const BOTTOM_RIGHT: &str = "┘";
/// Vertical border segment.
pub const VERT: &str = "│";

#[must_use]
/// Renders a board to an ANSI string.
pub fn render<const N: usize>(board: Board<N>) -> String {
    let mut s = String::new();
    s.push_str(TOP_LEFT);
    for _ in 0..WIDTH {
        s.push_str(HORIZ);
    }
    s.push_str(TOP_RIGHT);
    s.push('\n');
    for y in (0..board.max_y() + 4).rev() {
        s.push_str(VERT);
        for x in 0..WIDTH {
            if board.get(x, y) {
                s.push_str(cell());
            } else {
                s.push_str(empty_cell());
            }
        }
        s.push_str(VERT);
        s.push('\n');
    }
    s.push_str(BOTTOM_LEFT);
    for _ in 0..WIDTH {
        s.push_str(HORIZ);
    }
    s.push_str(BOTTOM_RIGHT);
    s
}

#[must_use]
/// Renders a board with one highlighted placement overlay.
pub fn render_with<const N: usize>(board: Board<N>, placement: &Move) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    s.push_str(TOP_LEFT);
    for _ in 0..WIDTH {
        s.push_str(HORIZ);
    }
    s.push_str(TOP_RIGHT);
    s.push('\n');
    for y in (0..board.max_y() + 4).rev() {
        s.push_str(VERT);
        for x in 0..WIDTH {
            if board.get(x, y) {
                s.push_str(cell());
            } else if placement
                .cells()
                .into_iter()
                .any(|(cx, cy)| cx == x && cy == y)
            {
                s.push_str(colored_cell(placement.piece()));
            } else {
                s.push_str(empty_cell());
            }
        }
        s.push_str(VERT);
        s.push('\n');
    }
    s.push_str(BOTTOM_LEFT);
    for _ in 0..WIDTH {
        s.push_str(HORIZ);
    }
    s.push_str(BOTTOM_RIGHT);
    s.push('\n');
    let _ = write!(s, "{placement:?}");
    s
}

/// Renders two boards overlayed on top of each other.
#[must_use]
pub fn merge<const N: usize>(board_a: Board<N>, board_b: Board<N>) -> String {
    let mut s = String::new();
    s.push_str(TOP_LEFT);
    for _ in 0..WIDTH {
        s.push_str(HORIZ);
    }
    s.push_str(TOP_RIGHT);
    s.push('\n');
    for y in (0..board_a.max_y().max(board_b.max_y()) + 4).rev() {
        s.push_str(VERT);
        for x in 0..WIDTH {
            let a = board_a.get(x, y);
            let b = board_b.get(x, y);
            if a && b {
                s.push_str(colored_cell(Piece::T));
            } else if a {
                s.push_str(colored_cell(Piece::Z));
            } else if b {
                s.push_str(colored_cell(Piece::I));
            } else {
                s.push_str(empty_cell());
            }
        }
        s.push_str(VERT);
        s.push('\n');
    }
    s.push_str(BOTTOM_LEFT);
    for _ in 0..WIDTH {
        s.push_str(HORIZ);
    }
    s.push_str(BOTTOM_RIGHT);
    s
}
