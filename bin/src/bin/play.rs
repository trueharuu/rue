use rand::seq::SliceRandom;
use rue_core::{
    board::{Board, render_vs},
    game::Game,
    piece::ALL_PIECES,
    queue::Queue,
    ruleset::AttackConfig,
};
use rue_eval::{Model, active::ActiveModel, board::BoardModel};
use rue_search::{beam::Beam, config::SearchConfig};

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
    pub target_ms: Option<u64>,
}

pub fn main() {
    let cfg = GlobalConfig {
        depth: 7,
        width: 5000,
        target_ms: None,
    };
    let left = Model {
        board: BoardModel {
            height: 2.3000000000000007,
            height_half: -0.4,
            height_quar: -3.8999999999999995,
            holes: -0.6999999999999993,
            cell_coveredness: -0.9,
            bumpiness: 2.500000000000001,
            bumpiness_sq: -0.6999999999999997,
            row_transitions: -3.000000000000001,
            well_column: [
                -0.011274052474416492,
                -0.7827282451124764,
                0.5691098193187347,
                2.1615107162015796,
                -0.27909013341441835,
                -0.7346236920410344,
                2.471477678790465,
                0.32949267025451073,
                -1.1862237399454896,
                -0.057460837153199,
            ],
            well_depth: 0.7999999999999999,
            incoming_garbage: -5.299999999999996,
            tsd_overhangs: 3.599999999999999,
            height_difference: [-5.0, 0.0, 0.0],
        },
        active: ActiveModel {
            waste: [
                -0.1255365479342755,
                0.2883110096022584,
                -0.12297061592198662,
                0.3802643165097589,
                0.19738539724685844,
                0.24564533224096047,
                0.17499045430089216,
            ],
            clear: [
                -0.10145846311876575,
                -1.2647952236498041,
                -1.158568072070674,
                -0.25494713207436015,
                4.569196931038301,
            ],
            clear_mini: [
                -0.18505317090427725,
                -0.33232540075553363,
                -0.7104492604423174,
                -1.6107147452979436,
            ],
            clear_spin: [
                0.06958488507706678,
                -4.241539521349635,
                1.5932260858824852,
                -0.5052230771393342,
            ],
            b2b: 6.299999999999993,
            combo: -4.4,
            in_multiplier: 1.6999999999999997,
            perfect_clear: 1.6999999999999997,
        },
        board_weight: 1.0,
        active_weight: 1.0,
    };

    let right = Model {
        active: ActiveModel {
            waste: [0.0; 7],
            clear: [0.0, 0.0, 0.0, 0.0, 0.0],
            clear_mini: [0.0; 4],
            clear_spin: [0.0; 4],
            b2b: 0.0,
            combo: 0.0,
            perfect_clear: 0.0,
            in_multiplier: 0.0,
        },
        board: BoardModel {
            height: -0.1,
            height_half: 0.0,
            height_quar: 0.0,

            holes: 0.0,
            cell_coveredness: 0.0,
            bumpiness: 0.0,
            bumpiness_sq: 0.0,
            well_column: [0.0; 10],
            well_depth: 0.0,
            tsd_overhangs: 0.0,
            row_transitions: 0.0,
            incoming_garbage: 0.0,
            height_difference: [0.0, 0.0, 0.0],
        },
        active_weight: 1.0,
        board_weight: 1.0,
    };

    let mut score_l = 0;
    let mut score_r = 0;

    #[allow(clippy::never_loop)]
    loop {
        let mut rng = rand::rng();
        let mut queue_l = vec![];
        let mut queue_r = vec![];

        for _ in 0..9 {
            let mut bag = ALL_PIECES;
            bag.shuffle(&mut rng);

            queue_l.extend(bag);
            queue_r.extend(bag);
        }

        let mut game_l = Game::new(
            Board::new(),
            queue_l[0],
            Queue::from_slice(&queue_l[1..]),
            AttackConfig::season_one(),
        );
        let mut game_r = Game::new(
            Board::new(),
            queue_r[0],
            Queue::from_slice(&queue_r[1..]),
            AttackConfig::season_one(),
        );

        let mut n = 1;
        loop {
            // if n % 14 == 0 {
            //     game_l.pending_garbage.accept_many(&[1; 2]);
            //     game_l.pending_garbage.accept_many(&[8; 1]);
            // }
            // left move
            let config_l = SearchConfig::new(left, cfg.width, cfg.depth, cfg.target_ms);

            let beam_l = Beam { config: config_l };

            let i = std::time::Instant::now();
            let Some(result_l) = beam_l.search(&game_l) else {
                println!("{:?}", MatchResult::Right);
                score_r += 1;
                break;
            };
            let el_l = i.elapsed();

            let board_l = game_l.board;
            let self_l = game_l.advance(&result_l.best_move);

            // right move
            let config_r = SearchConfig::new(right, cfg.width, cfg.depth, cfg.target_ms);

            let i = std::time::Instant::now();
            let beam_r = Beam { config: config_r };
            let Some(result_r) = beam_r.search(&game_r) else {
                println!("{:?}", MatchResult::Left);
                score_l += 1;
                break;
            };
            let el_r = i.elapsed();

            let board_r = game_r.board;

            render_vs(
                &board_l,
                &board_r,
                Some(result_l.best_move),
                Some(result_r.best_move),
                // None,
            );
            println!(
                "{score_l:>3}-{score_r:<3} | n={n:>3} | {el_l:.3?} ({:.3}pps), {el_r:.3?} ({:.3}pps)",
                1.0 / el_l.as_secs_f64(),
                1.0 / el_r.as_secs_f64()
            );
            println!(
                "{:?} ({:+.3}) sent {} {:?}",
                result_l.best_move, result_l.score, self_l.1, self_l.0
            );
            n += 1;
            println!("{:+.4} vs. {:+.4}", result_l.score, result_r.score);

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
                game_r
                    .pending_garbage
                    .accept((self_l.1 - send_r.1) as usize);
            } else if send_r.1 > self_l.1 {
                game_l
                    .pending_garbage
                    .accept((send_r.1 - self_l.1) as usize);
            }

            // refill queues if needed

            // println!("queue_len of {:?} = {}", game_l.queue, game_l.queue.len());
            while game_l.queue_len() < 14 || game_r.queue_len() < 14 {
                let mut bag = ALL_PIECES;
                bag.shuffle(&mut rng);

                game_l.queue.extend(bag);
                game_r.queue.extend(bag);
            }
        }
    }
}
