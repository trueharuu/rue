//! SPSA weight tuner binary for Rue.

mod fitness;
mod logging;
mod spsa;

use std::path::PathBuf;

use clap::Parser;
use rue_eval::{simple::Simple, tunable::Tunable};

use crate::spsa::{SpsaConfig, run_spsa};

/// CLI for the SPSA weight tuner.
#[derive(Parser)]
#[command(name = "rue-tuner", about = "SPSA weight tuner for Rue evaluation models")]
struct Cli {
    /// Path to initial weights JSON.
    #[arg(short, long, default_value = "weights/simple-6a51a732.json")]
    weights: PathBuf,

    /// Output directory for checkpoints and logs.
    #[arg(short, long, default_value = "weights/tuner")]
    output: PathBuf,

    /// Games per fitness evaluation.
    #[arg(short = 'G', long, default_value_t = 20)]
    games: usize,

    /// SPSA iterations.
    #[arg(short, long, default_value_t = 1000)]
    iterations: usize,

    /// SPSA gain numerator `a`.
    #[arg(long, default_value_t = 0.05)]
    gain_a: f64,

    /// SPSA perturbation numerator `c`.
    #[arg(long, default_value_t = 0.1)]
    perturb_c: f64,

    /// Random seed (0 = time-based).
    #[arg(short, long, default_value_t = 0)]
    seed: u64,

    /// Checkpoint every N iterations.
    #[arg(long, default_value_t = 50)]
    checkpoint: usize,

    /// Search depth for fitness games.
    #[arg(long, default_value_t = 7)]
    depth: usize,

    /// Beam width for fitness games.
    #[arg(long, default_value_t = 500)]
    beam: usize,

    /// Maximum number of pieces to play in a single game.
    #[arg(long, default_value_t = 100)]
    max_n: usize,
}

/// Entrypoint.
fn main() {
    let cli = Cli::parse();

    let seed = if cli.seed == 0 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    } else {
        cli.seed
    };

    println!("[tuner] loading weights from {}", cli.weights.display());
    let data = std::fs::read_to_string(&cli.weights).expect("failed to read weights file");
    let initial: Simple = serde_json::from_str(&data).expect("failed to parse weights file");
    println!(
        "[tuner] loaded {} parameters from {}",
        Simple::param_count(),
        cli.weights.display(),
    );

    let config = SpsaConfig {
        a: cli.gain_a,
        c: cli.perturb_c,
        a_stability: 10.0,
        alpha: 0.602,
        gamma: 0.101,
        iterations: cli.iterations,
        games_per_eval: cli.games,
        checkpoint_every: cli.checkpoint,
        seed,
        depth: cli.depth,
        beam_width: cli.beam,
        output_dir: cli.output,
        max_n: cli.max_n,
    };

    println!("[tuner] SPSA config: a={:.4} c={:.4} A={:.1} α={:.3} γ={:.3}", 
        config.a, config.c, config.a_stability, config.alpha, config.gamma);
    println!(
        "[tuner] {} iterations, {} games/eval, depth={}, beam={}, seed={}",
        config.iterations, config.games_per_eval, config.depth, config.beam_width, config.seed,
    );

    let final_params = run_spsa(&config, &initial);

    // save final weights
    let final_weights = Simple::from_vec(&final_params);
    let final_path = config.output_dir.join("final.json");
    let json = serde_json::to_string_pretty(&final_weights)
        .expect("failed to serialize final weights");
    std::fs::write(&final_path, json).expect("failed to write final weights");
    println!("[tuner] final weights saved to {}", final_path.display());
}
