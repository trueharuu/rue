//! Rue beam search.
//!
//! `BeamSearch` selects the strongest first placement for a game state. It is
//! generic over the evaluation `Model`, which sees the compact search game
//! and the attack each placement produces. Nodes hold a [`SearchGame`], the
//! subset of a `Game` a placement can change, so the beam stays small.
//!
//! Level expansion is parallelized with rayon. Results are deterministic:
//! expanding one parent depends only on that parent's state, the level merge
//! preserves parent order, and pruning is a pure top-k.
//!
//! [`SearchGame`]: rue_core::game::SearchGame

#![feature(min_adt_const_params)]

mod config;
mod expand;
mod node;

pub use config::{SearchConfig, SearchResult};

use rayon::prelude::*;

use std::cmp::Ordering;
use std::time::Instant;

use rue_core::game::Game;
use rue_core::game::ruleset::Ruleset;
use rue_core::placement::Move;
use rue_core::rule::Rule;
use rue_eval::model::Model;

use crate::expand::{expand_node, expand_root, Ctx};
use crate::node::Node;

/// Depth-limited beam search over root placements.
pub struct BeamSearch<'m, const N: usize, const RULE: Rule, M: Model + Sync> {
    model: &'m M,
    config: SearchConfig,
    out: Vec<Node<N>>,
}

impl<'m, const N: usize, const RULE: Rule, M: Model + Sync> BeamSearch<'m, N, RULE, M> {
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
    /// With a time budget the search widens from 200 toward `beam_width`,
    /// scheduling each pass width from the measured throughput so the pass
    /// fits in the remaining budget. Without a budget the search runs once at
    /// `beam_width` and is deterministic. With a budget the result is
    /// load-dependent. The result carries the accepted beam width, the wall
    /// time spent, and the configured budget.
    pub fn find_best_move(&mut self, game: &Game<N, RULE>) -> Option<SearchResult> {
        let started = Instant::now();
        let budget = self.config.time_budget;

        let (result, width) = match budget {
            None => {
                let width = self.config.beam_width.max(1);
                let (res, _) = self.search_once(game, width, None);
                (res, width)
            }
            Some(budget) => {
                let mut best: Option<(Move, f32)> = None;
                let mut best_width = 0;
                let start_width = self.config.beam_width.clamp(1, 200);
                let mut width = start_width;
                let mut throughput: Option<f64> = None;

                loop {
                    let pass_start = Instant::now();
                    let (result, finished) =
                        self.search_once(game, width, Some(started + budget));

                    if finished || best.is_none() {
                        if let Some(result) = result
                            && best.is_none_or(|b| result.1 > b.1)
                        {
                            best = Some(result);
                            best_width = width;
                        }
                        if finished {
                            let rate = pass_start.elapsed().as_secs_f64() / width as f64;
                            throughput = Some(match throughput {
                                None => rate,
                                Some(prev) => prev * 0.5 + rate * 0.5,
                            });
                        }
                    }

                    if width >= self.config.beam_width {
                        break;
                    }

                    let remaining = budget.saturating_sub(started.elapsed());
                    if remaining.is_zero() {
                        break;
                    }

                    let Some(rate) = throughput else {
                        break;
                    };

                    let target = (remaining.as_secs_f64() * self.config.budget_safety / rate)
                        as usize;
                    let target = target
                        .max(start_width)
                        .min(width.saturating_mul(2))
                        .min(self.config.beam_width);
                    if target <= width {
                        break;
                    }

                    width = target;
                }

                (best, best_width)
            }
        };

        result.map(|(best_move, score)| SearchResult {
            best_move,
            score,
            width,
            elapsed: started.elapsed(),
            budget,
        })
    }

    /// Runs one full-width, depth-limited search.
    ///
    /// Returns the best root placement and whether the full depth was
    /// reached. When `deadline` passes between levels the search stops early
    /// and reports `finished == false`, still returning the best found move.
    fn search_once(
        &mut self,
        game: &Game<N, RULE>,
        width: usize,
        deadline: Option<Instant>,
    ) -> (Option<(Move, f32)>, bool) {
        let ruleset = &game.ruleset;

        self.out.clear();
        {
            let mut ctx: Ctx<'_, '_, N, RULE, M> = Ctx {
                model: self.model,
                ruleset,
                out: &mut self.out,
            };
            expand_root(&mut ctx, game);
        }
        if self.out.is_empty() {
            return (None, true);
        }
        self.prune_select(0, width);
        self.out.reserve(width.saturating_mul(24));

        let mut start = 0usize;
        let mut levels = 1usize;
        while levels < self.config.depth {
            if let Some(deadline) = deadline
                && Instant::now() >= deadline
            {
                let winner = self.out[start];
                return (Some((winner.root_move, winner.score)), false);
            }

            let before = self.out.len();
            let parents = self.out[start..before].to_vec();
            if parents.is_empty() {
                break;
            }

            let added = self.expand_level(&parents, ruleset);
            if added == 0 {
                break;
            }
            self.prune_select(before, width);
            start = before;
            levels += 1;
        }

        let winner = self.out[start];
        (Some((winner.root_move, winner.score)), true)
    }

    /// Expands `parents` on the rayon pool and appends the children to `out`
    /// in parent order.
    fn expand_level(&mut self, parents: &[Node<N>], ruleset: &Ruleset) -> usize {
        let threads = rayon::current_num_threads().max(1);
        let chunk = parents.len().div_ceil(threads.saturating_mul(4)).max(1);

        let parts: Vec<Vec<Node<N>>> = parents
            .par_chunks(chunk)
            .map(|group| {
                let mut out = Vec::with_capacity(group.len() * 24);
                let mut ctx: Ctx<'_, '_, N, RULE, M> = Ctx {
                    model: self.model,
                    ruleset,
                    out: &mut out,
                };
                for &parent in group {
                    expand_node(&mut ctx, parent);
                }
                out
            })
            .collect();

        let mut added = 0;
        for part in parts {
            added += part.len();
            self.out.extend(part);
        }
        added
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
fn cmp<const N: usize>(a: &Node<N>, b: &Node<N>) -> Ordering {
    b.score.total_cmp(&a.score).then_with(|| a.root_move.raw().cmp(&b.root_move.raw()))
}

#[cfg(test)]
mod tests {
    use rue_core::board::Board;
    use rue_core::buffer::Buffer;
    use rue_core::game::QUEUE_SIZE;
use rue_core::game::garbage::GarbageQueue;
    use rue_core::game::ruleset::SEASON_2;
    use rue_core::game::search::SearchGame;
    use rue_core::game::Game;
    use rue_core::piece::Piece;
    use rue_core::rng::Rng;
    use rue_core::rule::DEFAULT;
    use rue_eval::simple::Simple;
    use rue_nav::movegen::fast;

    use crate::BeamSearch;
    use crate::SearchConfig;

    /// Appends one full 7-bag to the queue.
    fn fill(queue: &mut Buffer<Piece, { QUEUE_SIZE }>, rng: &mut Rng) {
        let mut bag = Piece::ALL;
        rng.shuffle_array(&mut bag);
        for piece in bag {
            queue.push(piece);
        }
    }

    /// `Game::play` must leave the game and report the attack exactly as
    /// `Game::advance` does when the garbage queue is empty. Only garbage
    /// tanking differs. `SearchGame::play` must match `Game::play` exactly.
    #[test]
    fn play_matches_advance_and_search() {
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

            // SearchGame twin of `b` (same pre-state), played independently.
            let mut sg = SearchGame::from(&b);
            let attack_sg = sg.play(mv, &b.ruleset);

            let attack_a = a.advance(&mv);
            let attack_b = b.play(mv);

            assert_eq!(attack_a, attack_b, "play/advance mismatch for {mv}");
            assert_eq!(attack_b, attack_sg, "search/play mismatch for {mv}");

            assert_eq!(a.board, b.board, "board mismatch");
            assert_eq!(a.queue, b.queue, "queue mismatch");
            assert_eq!(a.hold, b.hold, "hold mismatch");
            assert_eq!(a.combo, b.combo, "combo mismatch");
            assert_eq!(a.b2b, b.b2b, "b2b mismatch");
            assert_eq!(a.garbage_queue, b.garbage_queue, "garbage mismatch");

            assert_eq!(sg.board, b.board, "search board mismatch");
            assert_eq!(sg.queue, b.queue, "search queue mismatch");
            assert_eq!(sg.hold, b.hold, "search hold mismatch");
            assert_eq!(sg.combo, b.combo, "search combo mismatch");
            assert_eq!(sg.b2b, b.b2b, "search b2b mismatch");
            assert_eq!(sg.incoming, b.garbage_queue.total(), "search incoming mismatch");
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
                budget_safety: 0.85,
            },
        );

        let result = search.find_best_move(&game).expect("search must move");
        let piece = result.best_move.piece();
        let playable = [game.queue[0], game.queue[1]].contains(&piece);
        assert!(playable, "best move piece {piece} is not reachable");
    }

    /// The parallel expansion must give identical results across runs and
    /// instances for identical, seeded inputs.
    #[test]
    fn search_is_deterministic() {
        let mut a = Game::<8, DEFAULT> {
            rng: Rng::new_seeded(7),
            grng: Rng::new_seeded(8),
            board: Board::empty(),
            queue: Buffer::new(),
            hold: None,
            combo: None,
            b2b: None,
            garbage_queue: GarbageQueue::new(),
            ruleset: SEASON_2,
        };
        fill(&mut a.queue, &mut a.rng);
        let b = a;

        let model = Simple::default();
        let config = SearchConfig {
            beam_width: 200,
            depth: 7,
            time_budget: None,
            futility_delta: 0.0,
            budget_safety: 0.85,
        };

        let mut s1 = BeamSearch::new(&model, config);
        let mut s2 = BeamSearch::new(&model, config);

        let r1 = s1.find_best_move(&a).expect("search must move");
        let r1b = s1.find_best_move(&a).expect("search must move");
        let r2 = s2.find_best_move(&b).expect("search must move");

        assert_eq!(r1.best_move, r1b.best_move);
        assert_eq!(r1.score, r1b.score);
        assert_eq!(r1.best_move, r2.best_move);
        assert_eq!(r1.score, r2.score);
    }
}