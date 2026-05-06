use std::{
    error::Error,
    fmt::{Display, Formatter},
    time::Instant,
};

use engine_core::{board::Board, piece::Mino, spin::Spin};
use engine_nav::{game::Game, keyfinder::keygen, placement_info::PlacementInfo};

use rayon::prelude::*;

use crate::reward::{Reward, Value};

#[derive(Debug, Clone)]
pub struct Model {
    pub back_to_back: i32,
    pub bumpiness: i32,
    pub bumpiness_sq: i32,
    pub row_transitions: i32,
    pub height: i32,
    pub top_half: i32,
    pub top_quarter: i32,
    pub jeopardy: i32,
    pub cavity_cells: i32,
    pub cavity_cells_sq: i32,
    pub overhang_cells: i32,
    pub overhang_cells_sq: i32,
    pub covered_cells: i32,
    pub covered_cells_sq: i32,
    // pub tslot: [i32; 4],
    pub well_depth: i32,
    pub max_well_depth: i32,
    pub well_column: [i32; 10],
    pub b2b_clear: i32,
    pub clear: [i32; 4],
    pub spin: [i32; 4],
    pub spin_mini: [i32; 4],
    pub perfect_clear: i32,
    pub combo_garbage: i32,
    pub waste: [i32; 7],
    pub incoming_garbage: i32,
    pub outgoing_garbage: i32,
    pub b2b_cap: i32,
    pub broke_surge: i32,

    pub name: String,
    // pub kpp: i32,
}

pub const COMBO_GARBAGE: [u16; 21] = [
    0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3,
];

impl Default for Model {
    fn default() -> Self {
        Self {
            back_to_back: 200,
            bumpiness: -24,
            bumpiness_sq: -7,
            row_transitions: -5,
            height: -39,
            top_half: -150,
            top_quarter: -511,
            jeopardy: -11,
            cavity_cells: -173,
            cavity_cells_sq: -3,
            overhang_cells: -34,
            overhang_cells_sq: -1,
            covered_cells: -17,
            covered_cells_sq: -1,
            well_depth: 57,
            max_well_depth: 17,
            well_column: [20, 23, 20, 500, 59, 21, 590, 10, -10, 24],
            waste: [-152, -152, 0, 0, 0, 0, 0],
            b2b_clear: 104,
            clear: [-143, -100, -58, 390],
            spin: [0, 121, 999, 602],
            spin_mini: [0, -158, -93, -600],
            perfect_clear: 999,
            combo_garbage: 150,
            incoming_garbage: -5,
            outgoing_garbage: 100,
            name: "default".to_string(),
            // kpp: -10000,
            b2b_cap: 8,
            broke_surge: 50,
        }
    }
}

impl Model {
    pub fn evaluate(&self, game: &Game, info: &PlacementInfo) -> (Value, Reward) {
        let mut te = 0;
        let mut ae = 0;

        te += self.incoming_garbage * game.total_garbage() as i32;
        ae += self.outgoing_garbage * info.outgoing_attack as i32;

        if info.broke_surge {
            ae += game.b2b as i32 * self.broke_surge;
        }

        if game.is_pc() {
            ae += self.perfect_clear;
        }

        if !game.is_pc() {
            if info.b2b_clear {
                ae += self.b2b_clear;
            }

            if game.combo > 0 {
                let c = 1.0 + game.combo as f32 / 4.0;

                ae += self.combo_garbage
                    * COMBO_GARBAGE[game.combo.min(20) as usize]
                        .max((c * info.outgoing_attack as f32) as u16) as i32;
            }

            match (info.mino, info.lines_cleared, info.spin) {
                (Mino::T, 0, Spin::Mini) => ae += self.spin_mini[0],
                (Mino::T, 1, Spin::Mini) => ae += self.spin_mini[1],
                (Mino::T, 2, Spin::Mini) => ae += self.spin_mini[2],
                (Mino::T, 3, Spin::Mini) => ae += self.spin_mini[3],
                (Mino::T, 0, Spin::Full) => ae += self.spin[0],
                (Mino::T, 1, Spin::Full) => ae += self.spin[1],
                (Mino::T, 2, Spin::Full) => ae += self.spin[2],
                (Mino::T, 3, Spin::Full) => ae += self.spin[3],
                (_, 1, _) => ae += self.clear[0],
                (_, 2, _) => ae += self.clear[1],
                (_, 3, _) => ae += self.clear[2],
                (_, 4, _) => ae += self.clear[3],
                _ => {}
            }
        }

        te += self.back_to_back * game.b2b.min(self.b2b_cap as i16) as i32;

        match (info.mino, info.loc.spin) {
            (_, Spin::Mini | Spin::Full) => {}
            _ => ae += self.waste[info.mino.idx()],
        }

        let highest_point = *game.board.col_heights().iter().max().unwrap() as i32;
        te += self.top_quarter * (highest_point - 15).max(0);
        te += self.top_half * (highest_point - 10).max(0);

        ae += self.jeopardy * (highest_point - 10).max(0);

        let highest_point = *game.board.col_heights().iter().max().unwrap() as i32;
        te += self.height * highest_point;

        let mut well = 0;
        for x in 1..10 {
            if game.board.height_at(x) <= game.board.height_at(well) {
                well = x;
            }
        }

        let mut depth = 0;
        'y: for y in game.board.height_at(well)..20 {
            for x in 0..10 {
                if x as usize != well && !game.board.get(x, y as usize) {
                    break 'y;
                }
            }

            depth += 1;
        }

        let depth = depth.min(self.max_well_depth);
        te += self.well_depth * depth;
        if depth != 0 {
            te += self.well_column[well];
        }

        if self.row_transitions != 0 {
            te += self.row_transitions
                * (0..40)
                    .map(|y| game.board.get_row(y))
                    .map(|r| (r | 0b1_00000_00000) ^ (1 | r << 1))
                    .map(|d| d.count_ones() as i32)
                    .sum::<i32>()
        }

        if self.bumpiness | self.bumpiness_sq != 0 {
            let (bump, bump_sq) = bumpiness(&game.board, well);
            te += self.bumpiness * bump;
            te += self.bumpiness_sq * bump_sq;
        }

        if self.cavity_cells | self.cavity_cells_sq | self.overhang_cells | self.overhang_cells_sq
            != 0
        {
            let (cavity_cells, overhang_cells) = cavities_and_overhangs(&game.board);
            te += self.cavity_cells * cavity_cells;
            te += self.cavity_cells_sq * cavity_cells * cavity_cells;
            te += self.overhang_cells * overhang_cells;
            te += self.overhang_cells_sq * overhang_cells * overhang_cells;
        }

        if self.covered_cells | self.covered_cells_sq != 0 {
            let (covered_cells, covered_cells_sq) = covered_cells(&game.board);
            te += self.covered_cells * covered_cells;
            te += self.covered_cells_sq * covered_cells_sq;
        }

        (
            Value {
                value: te,
                spike: 0,
            },
            Reward {
                value: ae,
                attack: if info.outgoing_attack > 0 {
                    info.outgoing_attack as i32
                } else {
                    -1
                },
            },
        )
    }
}

fn bumpiness(board: &Board, well: usize) -> (i32, i32) {
    let mut bumpiness = -1;
    let mut bumpiness_sq = -1;

    let mut prev = if well == 0 { 1 } else { 0 };
    for i in 1..10 {
        if i == well {
            continue;
        }
        let dh = (board.col_heights()[prev] - board.col_heights()[i]).abs();
        bumpiness += dh;
        bumpiness_sq += dh * dh;
        prev = i;
    }

    (bumpiness.abs() as i32, bumpiness_sq.abs() as i32)
}
fn cavities_and_overhangs(board: &Board) -> (i32, i32) {
    let mut cavities = 0;
    let mut overhangs = 0;

    for y in 0..*board.col_heights().iter().max().unwrap() {
        for x in 0..10 {
            if board.get(x, y as usize) || y >= board.col_heights()[x] {
                continue;
            }

            if x > 1 {
                if board.col_heights()[x - 1] <= y - 1 && board.col_heights()[x - 2] <= y {
                    overhangs += 1;
                    continue;
                }
            }

            if x < 8 {
                if board.col_heights()[x + 1] <= y - 1 && board.col_heights()[x + 2] <= y {
                    overhangs += 1;
                    continue;
                }
            }

            cavities += 1;
        }
    }

    (cavities, overhangs)
}

fn covered_cells(board: &Board) -> (i32, i32) {
    let mut covered = 0;
    let mut covered_sq = 0;

    for x in 0..10 {
        for y in (0..board.col_heights()[x] - 2).rev() {
            if !board.get(x, y as usize) {
                let cells = 6.min(board.col_heights()[x] - y - 1) as i32;
                covered += cells;
                covered_sq += cells * cells;
            }
        }
    }

    (covered, covered_sq)
}

pub fn concentration(gb: &[u8]) -> f32 {
    if gb.is_empty() {
        return 0.0;
    }

    let sum: f32 = gb.iter().map(|&x| x as f32).sum();
    if sum == 0.0 {
        return 0.0;
    }

    let sum_sq: f32 = gb
        .iter()
        .map(|&x| {
            let x = x as f32;
            x * x
        })
        .sum();

    sum_sq / (sum * sum)
}
