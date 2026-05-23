use engine_core::{
    board::{Board, render_vs},
    game::Game,
    piece::ALL_PIECES,
    ruleset::AttackConfig,
};
use engine_eval::{Model, active::ActiveModel, board::BoardModel};
use engine_search::{beam::Beam, config::SearchConfig};
use rand::seq::SliceRandom;

#[derive(Clone)]
pub struct Player {
    pub model: Model,
    pub performance: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MatchResult {
    Left,
    Right,
    // Draw,
}

pub struct GlobalConfig {
    pub depth: usize,
    pub width: usize,
}

pub fn main() {
    let cfg = GlobalConfig {
        depth: 7,
        width: 250,
    };
    let left = Model {
        board: BoardModel {
            height: 0.7000000000000001,
            height_half: -0.20000000000000004,
            height_quar: -3.2999999999999994,
            holes: -3.1,
            cell_coveredness: -0.09999999999999998,
            bumpiness: 0.30000000000000004,
            bumpiness_sq: -0.5000000000000001,
            row_transitions: -1.2,
            well_column: [
                -0.23493245468668988,
                -1.0,
                0.28588165386343267,
                2.1880127933527347,
                0.12574325617873364,
                -0.49309820190995224,
                2.053785684928208,
                0.29023439663627143,
                -1.041909318975018,
                0.08488948281316683,
            ],
            well_depth: -0.6000000000000001,
            incoming_garbage: -1.5999999999999996,
            tsd_overhangs: 4.9,
        },
        active: ActiveModel {
            waste: [
                -0.06838496425855684,
                0.04343228270537561,
                -0.12792381038432626,
                0.0,
                -0.22691531202267004,
                0.0539578605591538,
                -0.14909853092930536,
            ],
            clear: [
                -0.09457799295287939,
                -1.1203486324123277,
                -0.8727972827529278,
                -0.9529601509372329,
                4.053935946771951,
            ],
            clear_mini: [
                -0.6107870748642772,
                0.5085022877672654,
                0.523645497923773,
                -0.9463716800468318,
            ],
            clear_spin: [
                -0.25196715675900316,
                -0.45406037187276316,
                0.9652149173843194,
                -0.5539153249904268,
            ],
            b2b: 2.400000000000001,
            combo: 3.1,
            in_multiplier: 0.9999999999999999,
            perfect_clear: 3.1,
        },
        board_weight: 1.0,
        active_weight: 1.0,
    };

    let right = Model {
        active: ActiveModel {
            waste: [0.0; 7],
            clear: [0.0; 5],
            clear_mini: [0.0; 4],
            clear_spin: [0.0; 4],
            b2b: 0.0,
            combo: 0.0,
            perfect_clear: 0.0,
            in_multiplier: 0.0,
        },
        board: BoardModel {
            height: -0.1,
            height_half: -0.2,
            height_quar: -0.5,

            holes: 0.0,
            cell_coveredness: 0.0,
            bumpiness: 0.0,
            bumpiness_sq: 0.0,
            well_column: [0.0; 10],
            well_depth: 0.0,
            tsd_overhangs: 0.0,
            row_transitions: 0.0,
            incoming_garbage: 0.0,
        },
        active_weight: 1.0,
        board_weight: 1.0,
    };

    let mut score_l = 0;
    let mut score_r = 0;

    loop {
        let mut rng = rand::rng();
        let mut queue_l = vec![];
        let mut queue_r = vec![];

        for _ in 0..100 {
            let mut bag = ALL_PIECES;
            bag.shuffle(&mut rng);

            queue_l.extend(bag);
            queue_r.extend(bag);
        }

        let mut game_l = Game::new(
            Board::new(),
            queue_l[0],
            queue_l[1..].to_vec(),
            AttackConfig::tetra_league(),
        );
        let mut game_r = Game::new(
            Board::new(),
            queue_r[0],
            queue_r[1..].to_vec(),
            AttackConfig::tetra_league(),
        );

        loop {
            // left move
            let config_l = SearchConfig {
                model: left,
                width: cfg.width,
                depth: cfg.depth,
            };

            let beam_l = Beam { config: config_l };
            let Some(result_l) = beam_l.search(&game_l) else {
                println!("{:?}", MatchResult::Right);
                score_r += 1;
                break;
            };

            let board_l = game_l.board.clone();
            let self_l = game_l.advance(&result_l.best_move);

            // right move
            let config_r = SearchConfig {
                model: right,
                width: cfg.width,
                depth: cfg.depth,
            };

            let beam_r = Beam { config: config_r };
            let Some(result_r) = beam_r.search(&game_r) else {
                println!("{:?}", MatchResult::Left);
                score_l += 1;
                break;
            };

            let board_r = game_r.board.clone();

            render_vs(
                &board_l,
                &board_r,
                Some(result_l.best_move),
                Some(result_r.best_move),
            );
            println!("{score_l}-{score_r}");
            // println!("{:+.4} vs. {:+.4}", result_l.score, result_r.score);

            let send_r = game_r.advance(&result_r.best_move);

            if Game::spawn_envelope_blocked(&game_l.board) {
                println!("{:?}", MatchResult::Right);
                score_r += 1;
                break;
            }

            if Game::spawn_envelope_blocked(&game_r.board) {
                println!("{:?}", MatchResult::Left);
                score_l += 1;
                break;
            }

            if self_l.1 > send_r.1 {
                game_r.pending_garbage += self_l.1 - send_r.1;
            } else if send_r.1 > self_l.1 {
                game_l.pending_garbage += send_r.1 - self_l.1;
            }

            // refill queues if needed
            if game_l.queue_len() < 14 || game_r.queue_len() < 14 {
                let mut bag = ALL_PIECES;
                bag.shuffle(&mut rng);

                queue_l.extend(bag);
                queue_r.extend(bag);
            }
        }
    }
}
