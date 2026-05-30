use std::sync::atomic::AtomicU32;

use rand::seq::SliceRandom;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use rue_core::{board::Board, game::Game, piece::ALL_PIECES, queue::Queue, ruleset::AttackConfig};
use rue_eval::{Model, active::ActiveModel, board::BoardModel};
use rue_search::{beam::Beam, config::SearchConfig};
use std::sync::atomic::Ordering;

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

pub fn play_match(cfg: &GlobalConfig, red: &Model, blue: &Model) -> MatchResult {
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
        AttackConfig::tetra_league(),
    );
    let mut game_r = Game::new(
        Board::new(),
        queue_r[0],
        Queue::from_slice(&queue_r[1..]),
        AttackConfig::tetra_league(),
    );

    loop {
        // left move
        let config_l = SearchConfig::new(*red, cfg.width, cfg.depth, None);

        let beam_l = Beam { config: config_l };
        let Some(result_l) = beam_l.search(&game_l) else {
            return MatchResult::Right;
        };

        // let board_l = game_l.board.clone();
        let self_l = game_l.advance(&result_l.best_move);

        // right move
        let config_r = SearchConfig::new(*blue, cfg.width, cfg.depth, None);

        let beam_r = Beam { config: config_r };
        let Some(result_r) = beam_r.search(&game_r) else {
            return MatchResult::Left;
        };

        // let board_r = game_r.board.clone();

        // render_vs(
        //     &board_l,
        //     &board_r,
        //     Some(result_l.best_move),
        //     Some(result_r.best_move),
        // );
        // println!("{:+.4} vs. {:+.4}", result_l.score, result_r.score);

        let send_r = game_r.advance(&result_r.best_move);

        if Game::spawn_envelope_blocked(&game_l.board) {
            return MatchResult::Right;
        }

        if Game::spawn_envelope_blocked(&game_r.board) {
            return MatchResult::Left;
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
        if game_l.queue_len() < 14 || game_r.queue_len() < 14 {
            let mut bag = ALL_PIECES;
            bag.shuffle(&mut rng);

            queue_l.extend(bag);
            queue_r.extend(bag);
        }
    }
}

pub fn train(config: &GlobalConfig, initial: Model, num_players: usize, epochs: u32) -> Model {
    // initialize players
    let mut players = vec![
        Player {
            model: initial,
            performance: 0,
        };
        num_players
    ]
    .iter()
    .map(|p| {
        let mut pl = p.clone();
        pl.model = mutate(pl.model, 0.1, 1.0);
        pl
    })
    .collect::<Vec<Player>>();

    for epoch in 0..epochs {
        // snapshot weights so all matches see the same epoch’s weights.
        let epoch_weights: Vec<_> = players.iter().map(|p| p.model).collect();

        let finished = AtomicU32::new(0);
        let total = ((players.len() * players.len()) - players.len()) / 2;
        let perf_counters = (0..num_players)
            .map(|_| AtomicU32::new(0))
            .collect::<Vec<_>>();

        // build all (i,j) pairs with i<j
        let pairs: Vec<(usize, usize)> = (0..num_players)
            .flat_map(|i| ((i + 1)..num_players).map(move |j| (i, j)))
            .collect();

        // run all matches in parallel
        pairs.par_iter().for_each(|&(i, j)| {
            let victor = play_match(config, &epoch_weights[i], &epoch_weights[j]);
            finished.fetch_add(1, Ordering::Relaxed);
            println!(
                "[{epoch}:{}/{total}] {} vs {}: {:?}",
                finished.load(Ordering::Relaxed),
                i,
                j,
                victor
            );
            if victor == MatchResult::Left {
                perf_counters[i].fetch_add(1, Ordering::Relaxed);
            } else {
                perf_counters[j].fetch_add(1, Ordering::Relaxed);
            }
        });

        // write back the performances
        for i in 0..num_players {
            players[i].performance = perf_counters[i].load(Ordering::Relaxed);
        }

        // if last epoch, skip breeding/mutation
        if epoch == epochs - 1 {
            break;
        }

        // select top 25%
        let mut sorted = players.clone();
        sorted.sort_by_key(|p| std::cmp::Reverse(p.performance));
        let top_quart = &sorted[..(num_players / 4)];

        // print best performer
        let best = players.iter().max_by_key(|p| p.performance).unwrap().model;
        println!("[{epoch}] {best:?}");
        std::fs::write(
            format!("best/{epoch}.json"),
            serde_json::to_string_pretty(&best).unwrap(),
        )
        .unwrap();

        // redistribute & mutate in parallel
        players.par_iter_mut().enumerate().for_each(|(i, player)| {
            player.performance = 0;
            let parent = &top_quart[i % top_quart.len()].model;
            player.model = mutate(*parent, 0.1, 0.1)
        });
    }

    // pick the best performer
    players
        .into_iter()
        .max_by_key(|p| p.performance)
        .unwrap()
        .model
}

pub fn mutate(model: Model, chance: f64, incr: f64) -> Model {
    Model {
        active: ActiveModel {
            waste: mutate_many(model.active.waste, chance, incr),
            clear: mutate_many(model.active.clear, chance, incr),
            clear_mini: mutate_many(model.active.clear_mini, chance, incr),
            clear_spin: mutate_many(model.active.clear_spin, chance, incr),
            b2b: mutate_one(model.active.b2b, chance, incr),
            combo: mutate_one(model.active.combo, chance, incr),
            perfect_clear: mutate_one(model.active.perfect_clear, chance, incr),
            in_multiplier: mutate_one(model.active.in_multiplier, chance, incr),
        },
        active_weight: model.active_weight,
        board: BoardModel {
            height: mutate_one(model.board.height, chance, incr),
            height_half: mutate_one(model.board.height_half, chance, incr),
            height_quar: mutate_one(model.board.height_quar, chance, incr),

            holes: mutate_one(model.board.holes, chance, incr),
            cell_coveredness: mutate_one(model.board.cell_coveredness, chance, incr),

            bumpiness: mutate_one(model.board.bumpiness, chance, incr),
            bumpiness_sq: mutate_one(model.board.bumpiness_sq, chance, incr),
            row_transitions: mutate_one(model.board.row_transitions, chance, incr),
            well_column: mutate_many(model.board.well_column, chance, incr),
            well_depth: mutate_one(model.board.well_depth, chance, incr),
            tsd_overhangs: mutate_one(model.board.tsd_overhangs, chance, incr),
            incoming_garbage: mutate_one(model.board.incoming_garbage, chance, incr),
            height_difference: mutate_many(model.board.height_difference, chance, incr),
        },
        // opponent: OpponentModel {
        //     well_depth: mutate_one(model.opponent.well_depth, chance, incr),
        //     height: mutate_one(model.opponent.height, chance, incr),
        //     bumpiness: mutate_one(model.opponent.bumpiness, chance, incr),
        //     holes: mutate_one(model.opponent.holes, chance, incr),
        // },
        board_weight: model.board_weight,
    }
}

pub fn mutate_one(value: f64, chance: f64, incr: f64) -> f64 {
    let v = rand::random::<f64>();
    let mul = if v < chance {
        0.0
    } else if v < chance + (1.0 - chance) / 2.0 {
        1.0
    } else {
        -1.0
    };

    value + incr * mul
}

pub fn mutate_many<const N: usize>(value: [f64; N], chance: f64, incr: f64) -> [f64; N] {
    let mut x = [0.0; N];
    for i in 0..N {
        if rand::random_range(0.0..1.0) < chance {
            let dev = rand::random_range(-incr..incr);
            x[i] = value[i] + dev
        } else {
            x[i] = value[i];
        }
    }

    x
}

pub fn main() {
    let initial = Model {
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
        // opponent: OpponentModel {
        //     well_depth: -20.0,
        //     height: -10.0,
        //     bumpiness: -1.0,
        //     holes: -5.0,
        // },
        board_weight: 1.0,
        active_weight: 1.0,
    };

    let cfg = GlobalConfig {
        depth: 7,
        width: 100,
    };

    train(&cfg, initial, 25, 250);
}
