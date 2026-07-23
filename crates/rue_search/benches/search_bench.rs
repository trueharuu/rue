#![allow(missing_docs, clippy::missing_docs_in_private_items)]
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use rue_core::board::Board;
use rue_core::game::Game;
use rue_core::game::garbage::GarbageQueue;
use rue_core::game::ruleset::SEASON_2;
use rue_core::piece::Piece;
use rue_core::rng::Rng;
use rue_eval::simple::Simple;
use rue_search::SearchConfig;
use rue_search::beam_search;

const N: usize = 7;

fn zero_weights() -> Simple {
    Simple {
        b2b: 0.0,
        pc: 0.0,
        height: 0.0,
        height_half: 0.0,
        height_three_quarters: 0.0,
        bumpiness: 0.0,
        bumpiness_sq: 0.0,
        cell_coveredness: 0.0,
        holes: 0.0,
        row_transitions: 0.0,
        active: [[0.0; 3]; 5],
        combo: 0.0,
        sent: 0.0,
        well_col: [0.0; 10],
        well_depth: 0.0,
        garbage: 0.0,
        tsd_overhangs: 0.0,
        waste: [0.0; Piece::NB],
    }
}

fn empty_game(queue: Vec<Piece>) -> Game<N> {
    Game {
        board: Board::EMPTY,
        hold: None,
        queue,
        garbage_queue: GarbageQueue::new(),
        b2b_count: None,
        combo_count: None,
        ruleset: SEASON_2,
        rng: Rng::new(),
    }
}

fn bench_early_game(c: &mut Criterion) {
    let game = empty_game(vec![
        Piece::T,
        Piece::I,
        Piece::O,
        Piece::L,
        Piece::J,
        Piece::S,
        Piece::Z,
    ]);
    let config = SearchConfig {
        beam_width: 800,
        depth: 14,
        ..SearchConfig::default()
    };
    let weights = zero_weights();

    c.bench_function("beam_search_early_d14_w800", |b| {
        b.iter(|| beam_search(&game, &config, &weights));
    });
}

fn bench_mid_game(c: &mut Criterion) {
    let mut game = empty_game(vec![
        Piece::I,
        Piece::T,
        Piece::O,
        Piece::L,
        Piece::S,
        Piece::Z,
        Piece::J,
    ]);
    // Partially fill the board
    for y in 0..5 {
        for x in 0..10 {
            if (x + y) % 3 != 0 {
                game.board.set(x, y);
            }
        }
    }
    let config = SearchConfig {
        beam_width: 400,
        depth: 8,
        ..SearchConfig::default()
    };
    let weights = zero_weights();

    c.bench_function("beam_search_mid_d8_w400", |b| {
        b.iter(|| beam_search(&game, &config, &weights));
    });
}

fn bench_shallow(c: &mut Criterion) {
    let game = empty_game(vec![
        Piece::T,
        Piece::I,
        Piece::O,
        Piece::L,
        Piece::J,
        Piece::S,
        Piece::Z,
    ]);
    let config = SearchConfig {
        beam_width: 800,
        depth: 4,
        ..SearchConfig::default()
    };
    let weights = zero_weights();

    c.bench_function("beam_search_shallow_d4_w800", |b| {
        b.iter(|| beam_search(&game, &config, &weights));
    });
}

criterion_group!(benches, bench_early_game, bench_mid_game, bench_shallow);
criterion_main!(benches);
