//! Test crate for the singleplayer gameplay loop.

use std::time::Instant;

use clap::Parser;
use fumen::Fumen;
use rue_core::{
    board::Board,
    game::{Game, garbage::GarbageQueue, ruleset::SEASON_2},
    piece::Piece,
    placement::Move,
    render::render_with,
    rng::{Rng, RngKind},
};
use rue_eval::{simple::Simple, weights::Weights};

#[allow(missing_docs)]
#[derive(clap::Parser)]
pub struct Cli {
    #[arg(long, default_value = "weights/simple-handtuned.json")]
    pub load: String,

    #[arg(long)]
    pub pps: Option<f64>,

    #[arg(long, default_value_t = 500)]
    pub width: usize,

    #[arg(long, default_value_t = 7)]
    pub depth: usize,

    #[arg(short)]
    pub n: Option<usize>,
}
/// Entry point.
pub fn main() {
    let cli = Cli::parse();
    println!("Loading weights from {} ({})", cli.load, std::path::absolute(&cli.load).unwrap().display());
    let current = &cli.load;
    let model: Simple = serde_json::from_str(
        &std::fs::read_to_string(current.clone()).expect("failed to read weights"),
    )
    .expect("failed to parse weights");

    println!("Loaded model: {current}");

    let mut game = Game {
        board: Board::<8>::EMPTY,
        hold: None,
        queue: vec![],
        garbage_queue: GarbageQueue::new(),
        b2b_count: None,
        combo_count: None,
        ruleset: SEASON_2,
        rng: Rng::new(),
    };
    // game.ruleset.spins = Spins::T;

    let mut fu = Fumen::default();

    fill(&mut game.queue, &mut game.rng, 3);
    let mut total_attack = 0u32;
    let mut pieces = 0u32;
    let mut chain_pieces = 0u32;
    let mut chain_b2b = 0u32;
    let i_total = Instant::now();
    loop {
        if let Some(n) = cli.n
            && pieces >= n as u32 {
                break;
            }
        if pieces.is_multiple_of(14) {
            // game.garbage_queue.recieve(4, u32::MAX);
        }
        let instant = Instant::now();
        let best = best_placement(&game, &model, &cli);
        let elapsed = instant.elapsed();
        if best.is_none() {
            println!("dead");
            break;
        }

        let (best, score) = best.unwrap();
        println!("{}", render_with(game.board, &best));
        println!(
            "{:?}",
            pathfinder::get_input(&game.board, best, &game.ruleset, true, false)
        );
        let page = fu.add_page();
        for fy in 0..23 {
            for fx in 0..10 {
                let cell = game.board.get(fx, fy);
                if cell {
                    page.field[fy as usize][fx as usize] = fumen::CellColor::Grey;
                } else {
                    page.field[fy as usize][fx as usize] = fumen::CellColor::Empty;
                }
            }
        }

        for c in best.cells() {
            let (x, y) = c;
            if y < 23 {
                page.field[y as usize][x as usize] = match best.piece() {
                    Piece::I => fumen::CellColor::I,
                    Piece::O => fumen::CellColor::O,
                    Piece::T => fumen::CellColor::T,
                    Piece::S => fumen::CellColor::S,
                    Piece::Z => fumen::CellColor::Z,
                    Piece::J => fumen::CellColor::J,
                    Piece::L => fumen::CellColor::L,
                };
            }
        }

        let h = game.hold.map_or_else(String::new, |x| x.to_string());
        let q = game.queue[..6]
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<String>();
        let active = game.active();
        let out = game.tick(best);
        if game.b2b_count.is_none() {
            chain_pieces = 0;
            chain_b2b = 0;
        } else {
            chain_pieces += 1;
            if out.is_b2b() {
                chain_b2b += 1;
            }
        }
        pieces += 1;
        total_attack += out.attack_sent as u32;
        println!(
            "{score:.3} {elapsed:.2?} [{h}]{q} sent {}/{}",
            out.attack_sent,
            out.clear_type.count()
        );
        println!("choosable active pieces: {active:?}");
        println!(
            "n={pieces} b2b={:?} combo={:?} pieces/second={:.3} attack/piece={:.3} b2b/bag={:.3}",
            game.b2b_count,
            game.combo_count,
            f64::from(pieces) / i_total.elapsed().as_secs_f64(),
            f64::from(total_attack) / f64::from(pieces),
            f64::from(chain_b2b) / (f64::from(chain_pieces) / 7.0)
        );
        page.comment = Some(format!(
            "{score:.3}\nsent {}/{}\nb2b={:?}\nattack/piece={:.3}",
            out.attack_sent,
            out.clear_type.count(),
            game.b2b_count,
            f64::from(total_attack) / f64::from(pieces),
        ));
        // clear run_output.txt
        std::fs::write("run.txt", "").expect("failed to clear run.txt");
        std::fs::write("run.txt", fu.encode()).expect("failed to write fumen to run.txt");
        // break;
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

    println!("placed {} pieces in {:?} (global attack/piece: {:.3})", pieces, i_total.elapsed(), f64::from(total_attack) / f64::from(pieces));
}

/// Appends 14 pieces to the end of the queue.
fn fill(p: &mut Vec<Piece>, r: &mut Rng, n: usize) {
    for _ in 0..n {
        let mut slice: Vec<Piece> = RngKind::Bag7.slice();
        r.shuffle_array(&mut slice);
        p.extend_from_slice(&slice);
    }
}
use rue_nav::pathfinder;
use rue_search::{SearchConfig, beam_search};

/// The best placement at any given time.
#[must_use]
pub fn best_placement<const N: usize>(game: &Game<N>, model: &impl Weights, cli: &Cli) -> Option<(Move, f64)> {
    let cfg = SearchConfig {
        beam_width: cli.width,
        depth: cli.depth,
        futility_delta: 0.0,
        time_budget_ms: None,
        ..SearchConfig::default()
    };

    let result = beam_search(game, &cfg, model);
    result.map(|x| (x.best.root_move, x.best.score))
}
