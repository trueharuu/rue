use crate::{board::Board, piece::Mino, piece_location::PieceLocation};

pub fn render(board: &Board, placement: Option<PieceLocation>) {
    println!("\u{250c}{}\u{2510}", "\u{2500}".repeat(20));
    for y in (0..board.max_height().max(8) + 4).rev() {
        print!("\u{2502}");
        for x in 0..10 {
            let cell = (board.cols[x] >> y) & 1;
            // println!("({x}, {y}) = {cell}");
            if cell != 0 {
                print!("\x1b[48;2;127;127;127m  \x1b[0m")
            } else {
                if let Some(p) = &placement
                    && p.blocks().contains(&(x as i8, y as i8))
                {
                    print!("{}", draw_cell(p.piece));
                } else {
                    print!("\x1b[0m  \x1b[0m")
                }
            }
        }
        println!("\u{2502}");
    }
    println!("\u{2514}{}\u{2518}", "\u{2500}".repeat(20));
}

pub fn render_vs(
    board_a: &Board,
    board_b: &Board,
    placement_a: Option<PieceLocation>,
    placement_b: Option<PieceLocation>,
) {
    println!("\u{250c}{}\u{252c}{}\u{2510}", "\u{2500}".repeat(20), "\u{2500}".repeat(20));
    for y in (0..25).rev() {
        print!("\u{2502}");
        for x in 0..20 {
            if x == 10 {
                print!("\u{2502}");
                // continue;
            }
            let b = if x < 10 { board_a } else { board_b };
            let bx = x % 10;
            let cell = (b.cols[bx] >> y) & 1;
            // println!("({x}, {y}) = {cell}");
            if cell != 0 {
                print!("\x1b[48;2;127;127;127m  \x1b[0m")
            } else {
                let pp = if x < 10 { &placement_a } else { &placement_b };
                let px = x % 10;
                if let Some(p) = pp
                    && p.blocks().contains(&(px as i8, y as i8))
                {
                    print!("{}", draw_cell(p.piece));
                } else {
                    print!("\x1b[0m  \x1b[0m")
                }
            }
        }
        println!("\u{2502}");
    }
        println!("\u{2514}{}\u{2534}{}\u{2518}", "\u{2500}".repeat(20), "\u{2500}".repeat(20));
}

pub fn draw_cell(piece: Mino) -> &'static str {
    match piece {
        Mino::Z => "\x1b[48;2;255;127;127m  \x1b[0m",
        Mino::L => "\x1b[48;2;255;192;127m  \x1b[0m",
        Mino::O => "\x1b[48;2;255;255;127m  \x1b[0m",
        Mino::S => "\x1b[48;2;127;255;127m  \x1b[0m",
        Mino::I => "\x1b[48;2;127;255;255m  \x1b[0m",
        Mino::J => "\x1b[48;2;127;127;255m  \x1b[0m",
        Mino::T => "\x1b[48;2;255;127;255m  \x1b[0m",
    }
}
