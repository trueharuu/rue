//! Rue beam search.
//!
//! `BeamSearch` selects the strongest first placement for a game state. It is
//! generic over the evaluation `Model`, which sees the full game and the
//! attack each placement produces. The `Game`'s queue, hold, combo, and b2b
//! drive the search; no external queue slice is needed.

#![feature(min_adt_const_params)]

mod config;
mod expand;
mod node;

pub use config::{SearchConfig, SearchResult};

use std::cmp::Ordering;
use std::time::Instant;

use rue_core::game::Game;
use rue_core::rule::Rule;
use rue_eval::model::Model;

use crate::expand::{expand_node, expand_root, Ctx};
use crate::node::Node;

/// Depth-limited beam search over root placements.
pub struct BeamSearch<'m, const N: usize, const RULE: Rule, M: Model> {
    model: &'m M,
    config: SearchConfig,
    out: Vec<Node<N, RULE>>,
}

impl<'m, const N: usize, const RULE: Rule, M: Model> BeamSearch<'m, N, RULE, M> {
    /// Creates a search that borrows `model`.
    pub fn new(model: &'m M, config: SearchConfig) -> Self {
        Self {
            model,
            config,
            out: Vec::new(),
        }
    }

    /// Returns the best root placement for `game`.
    ///
    /// With a time budget the search widens from 200 in doubling steps up to
    /// `beam_width`, stopping when the budget elapses. Results are
    /// deterministic for a fixed input.
    pub fn find_best_move(&mut self, game: &Game<N, RULE>) -> Option<SearchResult> {
        let mut best = None;
        match self.config.time_budget {
            None => best = self.search_once(game, self.config.beam_width.max(1)),
            Some(budget) => {
                let started = Instant::now();
                let mut width = self.config.beam_width.clamp(1, 200);
                loop {
                    if best.is_some() && started.elapsed() >= budget {
                        break;
                    }
                    if let Some(result) = self.search_once(game, width)
                        && best.is_none_or(|b| result.score > b.score)
                    {
                        best = Some(result);
                    }
                    if width >= self.config.beam_width {
                        break;
                    }
                    width = width.saturating_mul(2).min(self.config.beam_width);
                }
            }
        }
        best
    }

    /// Runs one full-width, depth-limited search.
    fn search_once(&mut self, game: &Game<N, RULE>, width: usize) -> Option<SearchResult> {
        self.out.clear();
        {
            let mut ctx: Ctx<'_, '_, N, RULE, _> = Ctx {
                model: self.model,
                out: &mut self.out,
            };
            expand_root(&mut ctx, game);
        }
        if self.out.is_empty() {
            return None;
        }
        self.prune_select(0, width);

        let mut start = 0usize;
        let mut levels = 1usize;
        while levels < self.config.depth {
            let before = self.out.len();
            {
                let mut ctx: Ctx<'_, '_, N, RULE, _> = Ctx {
                    model: self.model,
                    out: &mut self.out,
                };
                for i in start..before {
                    let parent = ctx.out[i];
                    expand_node(&mut ctx, parent);
                }
            }
            if self.out.len() == before {
                break;
            }
            self.prune_select(before, width);
            start = before;
            levels += 1;
        }

        let winner = self.out[start];
        Some(SearchResult {
            best_move: winner.root_move,
            score: winner.score,
        })
    }

    /// Selects the top `width` nodes of the level that starts at `start` and
    /// sorts it. Parents before `start` are kept.
    fn prune_select(&mut self, start: usize, width: usize) {
        let end = self.out.len();
        if end <= start {
            return;
        }

        let delta = self.config.futility_delta;
        if delta > 0.0 {
            let mut max = self.out[start].score;
            for n in &self.out[start..end] {
                if n.score > max {
                    max = n.score;
                }
            }
            let cutoff = max - delta;
            let mut keep = start;
            let mut scan = start;
            while scan < end {
                if self.out[scan].score >= cutoff {
                    self.out[keep] = self.out[scan];
                    keep += 1;
                }
                scan += 1;
            }
            self.out.truncate(keep);
        }

        if self.out.len() > start + width {
            self.out.select_nth_unstable_by(start + width, cmp);
            self.out.truncate(start + width);
        }
        self.out[start..].sort_unstable_by(cmp);
    }
}

/// Total order: higher score first, lower raw move first as a tiebreak.
fn cmp<const N: usize, const RULE: Rule>(a: &Node<N, RULE>, b: &Node<N, RULE>) -> Ordering {
    b.score.total_cmp(&a.score).then_with(|| a.root_move.raw().cmp(&b.root_move.raw()))
}

#[cfg(test)]
mod tests {
    use rue_core::board::Board;
    use rue_core::buffer::Buffer;
    use rue_core::game::garbage::GarbageQueue;
    use rue_core::game::ruleset::SEASON_2;
    use rue_core::game::Game;
    use rue_core::piece::Piece;
    use rue_core::rng::Rng;
    use rue_core::rule::DEFAULT;
    use rue_nav::movegen::fast;
use rue_eval::simple::Simple;

use crate::BeamSearch;
use crate::SearchConfig;

    /// Appends one full 7-bag to the queue.
    fn fill(queue: &mut Buffer<Piece, 28>, rng: &mut Rng) {
        let mut bag = Piece::ALL;
        rng.shuffle_array(&mut bag);
        for piece in bag {
            queue.push(piece);
        }
    }

    /// `play` must leave the game and report the attack exactly as `advance`
    /// does when the garbage queue is empty. Only garbage tanking differs.
    #[test]
    fn play_matches_advance() {
        let mut a = Game::<8, DEFAULT> {
            rng: Rng::new_seeded(1),
            grng: Rng::new_seeded(2),
            board: Board::empty(),
            queue: Buffer::new(),
            hold: None,
            combo: None,
            b2b: None,
            garbage_queue: GarbageQueue::new(),
            ruleset: SEASON_2,
        };
        let mut b = a;

        fill(&mut a.queue, &mut a.rng);
        fill(&mut b.queue, &mut b.rng);

        for _ in 0..128 {
            if a.queue.len() <= 14 {
                fill(&mut a.queue, &mut a.rng);
                fill(&mut b.queue, &mut b.rng);
            }

            let y = a.board.height();
            let moves = fast::movegen::<8, DEFAULT>(&a.board, a.queue[0], y, 0);
            let Some(mv) = moves.iter().next() else {
                break;
            };

            let attack_a = a.advance(&mv);
            let attack_b = b.play(mv);

            assert_eq!(attack_a, attack_b, "attack mismatch for {mv}");
            assert_eq!(a.board, b.board, "board mismatch");
            assert_eq!(a.queue, b.queue, "queue mismatch");
            assert_eq!(a.hold, b.hold, "hold mismatch");
            assert_eq!(a.combo, b.combo, "combo mismatch");
            assert_eq!(a.b2b, b.b2b, "b2b mismatch");
            assert_eq!(a.garbage_queue, b.garbage_queue, "garbage mismatch");
        }
    }

    /// The search must return a root placement for the current piece.
    #[test]
    fn find_best_move_empty_board() {
        let mut game = Game::<8, DEFAULT> {
            rng: Rng::new_seeded(1),
            grng: Rng::new_seeded(2),
            board: Board::empty(),
            queue: Buffer::new(),
            hold: None,
            combo: None,
            b2b: None,
            garbage_queue: GarbageQueue::new(),
            ruleset: SEASON_2,
        };
        fill(&mut game.queue, &mut game.rng);

        let model = Simple::default();
        let mut search = BeamSearch::new(
            &model,
            SearchConfig {
                beam_width: 800,
                depth: 7,
                time_budget: None,
                futility_delta: 0.0,
            },
        );

        let result = search.find_best_move(&game).expect("search must move");
        let piece = result.best_move.piece();
        let playable = [game.queue[0], game.queue[1]].contains(&piece);
        assert!(playable, "best move piece {piece} is not reachable");
    }
}