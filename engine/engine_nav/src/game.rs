use engine_rng::rng::Rng;

use engine_core::{
    board::Board, piece::Mino, piece_location::PieceLocation, rotation::Rotation, spin::Spin,
};

use crate::placement_info::PlacementInfo;
#[derive(Debug, Clone)]
pub struct Game {
    pub board: Board,
    pub hold: Option<Mino>,
    pub b2b: i16,
    pub combo: i8,
    pub incoming_garbage: u16,
}

impl Game {
    pub fn new_empty() -> Self {
        Self {
            board: Board::new(),
            hold: None,
            b2b: -1,
            combo: -1,
            incoming_garbage: 0,
        }
    }

    pub fn is_pc(&self) -> bool {
        self.board.fold_or() == 0
    }

    pub fn advance(&mut self, next: Mino, loc: &PieceLocation) -> PlacementInfo {
        if loc.piece != next {
            self.hold = Some(next);
        }
        self.board.put_piece(&loc);
        let line_mask = self.board.remove_lines();

        let mut info = PlacementInfo {
            lines_cleared: line_mask.count_ones() as u8,
            lines_received: 0,
            pc: false,
            b2b_clear: false,
            broke_surge: false,
            spin: loc.spin,
            outgoing_attack: 0,
            mino: loc.piece,
            loc: loc.clone(),
        };

        if info.lines_cleared > 0 {
            self.combo += 1;
            if self.board.cols == [0u64; 10] {
                info.pc = true;
                info.b2b_clear = true;
            }

            if info.lines_cleared == 4 || loc.spin != Spin::None {
                info.b2b_clear = true;
            }

            let attack = self.calculate_attack(
                info.lines_cleared,
                info.spin,
                info.b2b_clear,
                info.pc,
                if info.b2b_clear {
                    0
                } else {
                    self.b2b.max(3) as u16 - 3
                },
                self.combo,
            );
            info.outgoing_attack = attack.saturating_sub(self.incoming_garbage);
            self.incoming_garbage = self.incoming_garbage.saturating_sub(attack);

            if info.b2b_clear {
                self.b2b += 1;
            } else {
                info.broke_surge = self.b2b > 3;
                self.b2b = -1;
            }
        } else {
            self.combo = -1;

            let lines = self.incoming_garbage.min(8);
            self.board
                .add_garbage(Rng::new(0).sample(0usize..10), lines);
            self.incoming_garbage -= lines;
            info.lines_received = lines;
        }
        info
    }

    pub fn calculate_attack(
        &self,
        lines_cleared: u8,
        spin: Spin,
        b2b_clear: bool,
        pc: bool,
        surge: u16,
        combo: i8,
    ) -> u16 {
        const COMBO_TABLE: [u16; 21] = [
            0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3,
        ]; // todo: make const func for this, especially for rounding modes

        if lines_cleared == 0 {
            return 0;
        }

        let mut attack = 0;

        attack += if spin == Spin::Full {
            2 * lines_cleared as u16
        } else {
            match lines_cleared {
                1 => 0,
                2 => 1,
                3 => 2,
                4 => 4,
                _ => unreachable!(),
            }
        };

        attack += surge;

        if pc {
            attack += 5;
        } else if b2b_clear {
            attack += 1;
        }

        if combo > 0 {
            let combo_mult = 1.0 + combo as f32 / 4.0;
            attack = COMBO_TABLE[combo.min(20) as usize].max((combo_mult * attack as f32) as u16);
        }
        attack
    }

    pub fn can_spawn_piece(&self, piece: Mino) -> bool {
        !self.board.obstructed(&PieceLocation {
            piece,
            rotation: Rotation::North,
            spin: Spin::None,
            x: 4,
            y: 21,
        })
    }
}
