use engine_core::{
    board::{Board, render_vs},
    game::Game,
    piece::ALL_PIECES,
};
use engine_eval::{Model, active::ActiveModel, board::BoardModel};
use engine_search::{beam::Beam, config::SearchConfig};
use rand::seq::SliceRandom;

#[derive(Clone)]
pub struct Player {
    pub model: Model,
    pub performance: f64,
}

#[derive(Debug)]
pub enum MatchResult {
    Left,
    Right,
    Draw,
}

pub struct GlobalConfig {
    pub depth: usize,
    pub width: usize,
}

pub fn play_match(cfg: &GlobalConfig, red: &Player, blue: &Player) -> MatchResult {
    let mut rng = rand::rng();
    let mut queue_l = vec![];
    let mut queue_r = vec![];

    for _ in 0..100 {
        let mut bag = ALL_PIECES;
        bag.shuffle(&mut rng);

        queue_l.extend(bag);
        queue_r.extend(bag);
    }

    let mut game_l = Game::new(Board::new(), queue_l[0], queue_l[1..].to_vec());
    let mut game_r = Game::new(Board::new(), queue_r[0], queue_r[1..].to_vec());

    loop {
        // left move
        let config_l = SearchConfig {
            model: red.model,
            width: cfg.width,
            depth: cfg.depth,
        };

        let beam_l = Beam { config: config_l };
        let Some(result_l) = beam_l.search(&game_l) else {
            return MatchResult::Right;
        };

        let board_l = game_l.board.clone();
        let self_l = game_l.advance(&result_l.best_move);

        // right move
        let config_r = SearchConfig {
            model: blue.model,
            width: cfg.width,
            depth: cfg.depth,
        };

        let beam_r = Beam { config: config_r };
        let Some(result_r) = beam_r.search(&game_r) else {
            return MatchResult::Left;
        };

        let board_r = game_r.board.clone();

        render_vs(
            &board_l,
            &board_r,
            Some(result_l.best_move),
            Some(result_r.best_move),
        );
        println!("{:+.4} vs. {:+.4}", result_l.score, result_r.score);

        let send_r = game_r.advance(&result_r.best_move);

        if Game::spawn_envelope_blocked(&game_l.board) {
            return MatchResult::Right;
        }

        if Game::spawn_envelope_blocked(&game_r.board) {
            return MatchResult::Left;
        }

        if self_l > send_r {
            game_r.pending_garbage += self_l - send_r;
        } else if send_r > self_l {
            game_l.pending_garbage += send_r - self_l;
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

pub fn main() {
    let red = Player {
        model: Model {
            active: ActiveModel {
                waste: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            },
            board: BoardModel {
                height: -1.0,
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
            },
            w_active: 1.0,
            w_board: 1.0,
        },
        performance: 0.0,
    };

    let blue = Player {
        model: Model {
            active: ActiveModel {
                waste: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            },
            board: BoardModel {
                height: -0.2,
                height_half: -1.0,
                height_quar: -5.0,
                holes: -4.0,
                cell_coveredness: -0.5,
                bumpiness: -0.3,
                bumpiness_sq: -0.1,
                well_column: [0.0; 10],
                well_depth: 0.2,
                tsd_overhangs: 6.0,
                row_transitions: -0.3,
            },
            w_active: 1.0,
            w_board: 1.0,
        },
        performance: 0.0,
    };

    let cfg = GlobalConfig {
        depth: 7,
        width: 100,
    };
    let result = play_match(&cfg, &red, &blue);
    println!("result = {result:?}");
}
