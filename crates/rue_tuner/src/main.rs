//! SPSA weight tuner binary for Rue.

/// Configuration structs for fitness evaluation and SPSA hyperparameters.
pub mod config;
/// Self-play fitness evaluation.
pub mod fitness;
/// CSV + terminal logging for SPSA iterations.
pub mod logging;
/// Core SPSA optimisation algorithm.
pub mod spsa;

use std::path::PathBuf;

use clap::Parser;
use rue_eval::simple::Simple;
use rue_eval::tunable::Tunable;

use crate::config::{FitnessConfig, SpsaConfig};
use crate::logging::SpsaLogger;
use crate::spsa::run_spsa;

/// SPSA weight tuner for Rue.
#[derive(Parser)]
#[command(name = "rue_tuner", about = "SPSA tuner for Simple evaluation weights")]
pub struct Cli {
    /// Path to initial weights JSON.
    #[arg(long, default_value = "weights/simple-handtuned.json")]
    load: PathBuf,

    /// Path to save the best weights found.
    #[arg(long, default_value = "weights/simple-tuned.json")]
    save: PathBuf,

    /// Path for CSV log output.
    #[arg(long, default_value = "spsa-log.csv")]
    csv: PathBuf,

    /// Maximum number of SPSA iterations.
    #[arg(long, default_value_t = 200)]
    iterations: usize,

    /// SPSA step-size numerator (a0).
    #[arg(long, default_value_t = 0.05)]
    a: f64,

    /// SPSA perturbation-size numerator (c0).
    #[arg(long, default_value_t = 0.1)]
    c: f64,

    /// SPSA stability constant (A). Should be ~10% of max iterations.
    #[arg(long, default_value_t = 10.0)]
    capital_a: f64,

    /// SPSA alpha exponent (for `a_k` gain sequence).
    #[arg(long, default_value_t = 0.602)]
    alpha: f64,

    /// SPSA gamma exponent (for `c_k` gain sequence).
    #[arg(long, default_value_t = 0.101)]
    gamma: f64,

    /// Number of games per fitness evaluation.
    #[arg(long, default_value_t = 8)]
    games: usize,

    /// Number of pieces per game.
    #[arg(long, default_value_t = 500)]
    pieces: usize,

    /// Beam width for search.
    #[arg(long, default_value_t = 500)]
    width: usize,

    /// Search depth.
    #[arg(long, default_value_t = 7)]
    depth: usize,
}

fn main() {
    let cli = Cli::parse();

    let initial: Simple = serde_json::from_str(
        &std::fs::read_to_string(&cli.load).expect("failed to read weights file"),
    )
    .expect("failed to parse weights");

    let fitness = FitnessConfig {
        pieces: cli.pieces,
        games: cli.games,
        beam_width: cli.width,
        depth: cli.depth,
    };

    let spsa = SpsaConfig {
        a0: cli.a,
        c0: cli.c,
        A: cli.capital_a,
        alpha: cli.alpha,
        gamma: cli.gamma,
        max_iter: cli.iterations,
        fitness,
    };

    eprintln!(
        "SPSA tuner: {} iterations, {} games x {} pieces, beam={} depth={}",
        spsa.max_iter,
        spsa.fitness.games,
        spsa.fitness.pieces,
        spsa.fitness.beam_width,
        spsa.fitness.depth,
    );

    // Logger with human-readable column names from Simple.
    let param_names: Vec<&str> = (0..Simple::param_count()).map(Simple::param_name).collect();
    let mut logger = SpsaLogger::create_with_names(&cli.csv, &param_names);

    let result = run_spsa::<Simple, 8, _>(&initial, &spsa, |log| {
        logger.log(&log);
    });

    // Save best weights.
    let json = serde_json::to_string_pretty(&result.best).expect("failed to serialize weights");
    std::fs::write(&cli.save, &json).expect("failed to write weights");
    eprintln!(
        "Done: {} iterations, best fitness={:.4}, saved to {}",
        result.iterations,
        result.best_fitness,
        cli.save.display(),
    );
}
