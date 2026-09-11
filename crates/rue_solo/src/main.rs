//! Solo benchmark: run the beam-search bot on a 7-bag rush.

use std::time::Duration;
use std::time::Instant;

use clap::Parser;
use rue_core::board::Board;
use rue_core::buffer::Buffer;
use rue_core::game::Game;
use rue_core::game::garbage::GarbageQueue;
use rue_core::game::ruleset::SEASON_2;
use rue_core::piece::Piece;
use rue_core::placement::Move;
use rue_core::render;
use rue_core::rng::Rng;
use rue_core::rule::DEFAULT;
use rue_eval::simple::Simple;
use rue_nav::path::Key;
use rue_nav::path::generate_inlined;
use rue_search::BeamSearch;
use rue_search::SearchConfig;

#[derive(Parser)]
#[command(name = "rue_solo", about = "Record a beam-search solo run as a fumen")]
struct Cli {
    /// Beam width.
    #[arg(long, default_value_t = 500)]
    width: usize,

    /// Number of pieces placed per search line.
    #[arg(long, default_value_t = 7)]
    depth: usize,

    /// Stop after this many placements.
    #[arg(short, long)]
    n: Option<usize>,

    /// Cap the placement rate at this many placements per second.
    #[arg(long)]
    pps: Option<f64>,
}

/// Entry point.
fn main() {
    let cli = Cli::parse();

    let model = Simple::default();
    let mut search: BeamSearch<8, DEFAULT, Simple> = BeamSearch::new(
        &model,
        SearchConfig {
            beam_width: cli.width,
            depth: cli.depth,
            futility_delta: 0.0,
            time_budget: cli.pps.map(|pps| Duration::from_secs_f64(1.0 / pps)),
        },
    );

    let mut game = Game::<8, DEFAULT> {
        rng: Rng::new(),
        grng: Rng::new(),
        board: Board::empty(),
        queue: Buffer::new(),
        hold: None,
        combo: None,
        b2b: None,
        garbage_queue: GarbageQueue::new(),
        ruleset: SEASON_2,
    };

    fill(&mut game.queue, &mut game.rng, 3);
    let mut total_attack = 0u32;
    let mut pieces = 0u32;
    let mut chain_pieces = 0u32;
    let mut chain_b2b = 0u32;
    let i_total = Instant::now();

    loop {
        if let Some(n) = cli.n
            && pieces >= n as u32
        {
            break;
        }

        let instant = Instant::now();
        let Some(result) = search.find_best_move(&game) else {
            println!("dead");
            break;
        };
        let best = result.best_move;
        let score = result.score;
        let elapsed = instant.elapsed();

        println!("{}", render::placement(&game.board, &best));
        println!("{best:?}");
        let keys = finesse_keys(&game.board, best);
        assert!(!keys.is_empty(), "can't actually do it");
        println!(
            "{}",
            keys.iter()
                .map(|k| format!("{k:?}"))
                .collect::<Vec<_>>()
                .join(" ")
        );

        let hold = game.hold.map_or_else(String::new, |p| p.to_string());
        let head = game
            .queue
            .iter()
            .skip(1)
            .take(6)
            .map(ToString::to_string)
            .collect::<String>();

        let pre_b2b = game.b2b;
        let attack = game.advance(&best);
        if game.b2b.is_none() {
            chain_pieces = 0;
            chain_b2b = 0;
        } else {
            chain_pieces += 1;
            if game.b2b == Some(pre_b2b.map_or(0, |b| b + 1)) {
                chain_b2b += 1;
            }
        }

        pieces += 1;
        total_attack += attack.outgoing();
        println!(
            "{score:.3} {elapsed:.2?} [{hold}]{head} sent {}/{}",
            attack.outgoing(),
            attack.line_clears,
        );
        let b2b_per_bag = if chain_pieces == 0 {
            0.0
        } else {
            f64::from(chain_b2b) / (f64::from(chain_pieces) / 7.0)
        };
        println!(
            "n={pieces} b2b={:?} combo={:?} pieces/second={:.3} attack/piece={:.3} b2b/bag={:.3} apm={:.3}",
            game.b2b,
            game.combo,
            f64::from(pieces) / i_total.elapsed().as_secs_f64(),
            f64::from(total_attack) / f64::from(pieces),
            b2b_per_bag,
            f64::from(total_attack) / i_total.elapsed().as_secs_f64() * 60.0,
        );
        
        if game.queue.len() <= 14 {
            fill(&mut game.queue, &mut game.rng, 2);
        }

        if let Some(pps) = cli.pps {
            let sleep = (1.0 / pps) - elapsed.as_secs_f64();
            if sleep > 0.0 {
                std::thread::sleep(std::time::Duration::from_secs_f64(sleep));
            }
        }
    }

    println!(
        "placed {} pieces in {:?} (global attack/piece: {:.3})",
        pieces,
        i_total.elapsed(),
        f64::from(total_attack) / f64::from(pieces),
    );
}

/// Appends `bags` full 7-bags to the queue.
fn fill(queue: &mut Buffer<Piece, 28>, rng: &mut Rng, bags: usize) {
    for _ in 0..bags {
        let mut bag = Piece::ALL;
        rng.shuffle_array(&mut bag);
        for piece in bag {
            queue.push(piece);
        }
    }
}

/// Returns the key sequence that reaches `mv` on `board`, empty when
/// unreachable.
fn finesse_keys<const N: usize>(board: &Board<N>, mv: Move) -> Buffer<Key, 16> {
    let y = board.height();
    match mv.piece() {
        Piece::I => generate_inlined::<N, { Piece::I }, DEFAULT>(board, y, 0, mv),
        Piece::O => generate_inlined::<N, { Piece::O }, DEFAULT>(board, y, 0, mv),
        Piece::T => generate_inlined::<N, { Piece::T }, DEFAULT>(board, y, 0, mv),
        Piece::S => generate_inlined::<N, { Piece::S }, DEFAULT>(board, y, 0, mv),
        Piece::Z => generate_inlined::<N, { Piece::Z }, DEFAULT>(board, y, 0, mv),
        Piece::J => generate_inlined::<N, { Piece::J }, DEFAULT>(board, y, 0, mv),
        Piece::L => generate_inlined::<N, { Piece::L }, DEFAULT>(board, y, 0, mv),
    }
}