//! SPSA (Simultaneous Perturbation Stochastic Approximation) optimizer core.

use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;

use rue_eval::tunable::Tunable;

use crate::fitness::self_play_fitness;
use crate::logging::TuningLog;

/// Hyperparameters for the SPSA algorithm.
pub struct SpsaConfig {
    /// Gain sequence numerator.
    pub a: f64,
    /// Perturbation sequence numerator.
    pub c: f64,
    /// Stability constant (prevents gain from being too large early on).
    pub a_stability: f64,
    /// Gain decay exponent (typically 0.602).
    pub alpha: f64,
    /// Perturbation decay exponent (typically 0.101).
    pub gamma: f64,
    /// Total number of SPSA iterations.
    pub iterations: usize,
    /// Games per fitness evaluation.
    pub games_per_eval: usize,
    /// Save a checkpoint every N iterations.
    pub checkpoint_every: usize,
    /// PRNG seed for reproducibility.
    pub seed: u64,
    /// Search depth for fitness games.
    pub depth: usize,
    /// Beam width for fitness games.
    pub beam_width: usize,
    /// Output directory for checkpoints and logs.
    pub output_dir: std::path::PathBuf,
    /// Maximum number of pieces to play in a single game.
    pub max_n: usize,
}

/// Apply a perturbation to parameters, clamping to per-parameter bounds.
fn apply_perturbation<W: Tunable>(theta: &W, delta: &[f64], step: f64) -> W {
    let mut candidate = theta.clone();
    let n = W::param_count().min(delta.len());
    for (i, &d) in delta.iter().enumerate().take(n) {
        let perturbed = theta.get_param(i) + step * d;
        let (lo, hi) = W::param_bounds(i);
        candidate.set_param(i, perturbed.clamp(lo, hi));
    }
    candidate
}

/// Run the SPSA optimization loop.
///
/// Returns the final weight vector.
pub fn run_spsa<W: Tunable + Sync + serde::Serialize>(config: &SpsaConfig, initial: &W) -> Vec<f64> {
    let k = W::param_count();
    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut theta = initial.clone();

    std::fs::create_dir_all(&config.output_dir).expect("failed to create output directory");

    let mut log = TuningLog::new(&config.output_dir);

    println!(
        "[SPSA] starting: {} iterations, {} params, seed={}",
        config.iterations,
        k,
        config.seed,
    );

    for iter in 0..config.iterations {
        // gain sequences
        let a_k = config.a / (config.a_stability + (iter as f64 + 1.0)).powf(config.alpha);
        let c_k = config.c / (iter as f64 + 1.0).powf(config.gamma);

        // bernoulli ±1 perturbation vector
        let delta: Vec<f64> = (0..k)
            .map(|_| if rng.random_bool(0.5) { 1.0 } else { -1.0 })
            .collect();

        // form ± candidates
        let theta_plus = apply_perturbation(&theta, &delta, c_k);
        let theta_minus = apply_perturbation(&theta, &delta, -c_k);

        // evaluate objective (sequential for now)
        let j_plus =
            self_play_fitness(&theta_plus, config.games_per_eval, config.depth, config.beam_width, config.max_n);
        let j_minus =
            self_play_fitness(&theta_minus, config.games_per_eval, config.depth, config.beam_width, config.max_n);

        // gradient estimate: g = (J+ - J-) / (2c) * Δ
        let diff = j_plus - j_minus;
        let g: Vec<f64> = delta
            .iter()
            .map(|&d| diff / (2.0 * c_k) * d)
            .collect();

        // update: θ = θ - a_k * g
        for (i, gi) in g.iter().enumerate().take(k) {
            let new_val = theta.get_param(i) - a_k * gi;
            let (lo, hi) = W::param_bounds(i);
            theta.set_param(i, new_val.clamp(lo, hi));
        }

        // log
        let delta_j = j_plus - j_minus;
        log.append(iter, a_k, c_k, j_plus, j_minus, &theta);

        let sign = if delta_j >= 0.0 { '+' } else { '-' };
        println!(
            "[SPSA] iter {iter:>4}/{} | a={a_k:.6} c={c_k:.6} | J+={j_plus:.1} J-={j_minus:.1} | ΔJ={sign}{:.1}",
            config.iterations,
            delta_j.abs(),
        );

        // checkpoint
        if (iter + 1) % config.checkpoint_every == 0 || iter + 1 == config.iterations {
            let path = config.output_dir.join(format!("iter-{}.json", iter + 1));
            let json = serde_json::to_string_pretty(&theta)
                .expect("failed to serialize weights");
            std::fs::write(&path, json).expect("failed to write checkpoint");
            println!("[SPSA] checkpoint saved: {}", path.display());
        }
    }

    theta.to_vec()
}
