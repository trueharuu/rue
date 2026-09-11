pub mod ruleset;
pub mod garbage;
pub mod attack;
pub mod search;

use crate::board::Board;
use crate::buffer::Buffer;
use crate::header::WIDTH;
use crate::piece::Piece;
use crate::placement::Move;
use crate::rng::Rng;
use crate::rule::Rule;
use crate::game::ruleset::Ruleset;
use crate::game::garbage::GarbageQueue;
use crate::game::attack::Attack;

pub use search::SearchGame;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Game<const N: usize, const RULE: Rule> {
    pub rng: Rng,
    pub grng: Rng,
    pub board: Board<N>,
    pub queue: Buffer<Piece, 28>,
    pub hold: Option<Piece>,
    pub combo: Option<u32>,
    pub b2b: Option<u32>,
    pub garbage_queue: GarbageQueue,
    pub ruleset: Ruleset,
}

impl<const N: usize, const RULE: Rule> Game<N, RULE> {
    /// Applies `placement` and returns the attack it produces.
    ///
    /// Consumes queue and hold exactly as `play` does, then tanks or cancels
    /// garbage with the RNG and garbage queue.
    pub fn advance(&mut self, placement: &Move) -> Attack {
        let mut attack = self.play(*placement);

        if attack.line_clears == 0 {
            let segments = self.garbage_queue.tank(self.ruleset.garbage_cap);

            for segment in segments {
                let col = self.grng.next() as u32 % WIDTH as u32;
                self.board.insert_garbage(segment, col);
            }

            return attack;
        }

        // cancel garbage if any
        let garbage_canceled = self.garbage_queue.tank(attack.total);
        attack.canceled = garbage_canceled.iter().sum();

        attack
    }

    /// Applies `placement` without garbage handling and returns the attack it
    /// produces.
    ///
    /// Updates the board, queue, hold, combo, and b2b exactly as `advance`
    /// does. The search uses this to simulate children without an RNG or
    /// garbage queue.
    pub fn play(&mut self, placement: Move) -> Attack {
        let mut sim = SearchGame {
            board: self.board,
            queue: self.queue,
            hold: self.hold,
            combo: self.combo,
            b2b: self.b2b,
            incoming: self.garbage_queue.total(),
        };

        let attack = sim.play(placement, &self.ruleset);

        self.board = sim.board;
        self.queue = sim.queue;
        self.hold = sim.hold;
        self.combo = sim.combo;
        self.b2b = sim.b2b;

        attack
    }
}

