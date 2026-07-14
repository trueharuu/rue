//! Core SPSA (Simultaneous Perturbation Stochastic Approximation) algorithm.
//!
//! Implements Spall's standard SPSA with configurable gain sequences,
//! Bernoulli ±1 perturbation, and bounds clipping. The two function
//! evaluations per iteration (`θ+cΔ` and `θ-cΔ`) run in parallel.

use rue_eval::tunable::Tunable;

use crate::config::SpsaConfig;

use crate::fitness::multi_game;

/// Result of an SPSA run.
pub struct SpsaResult<T> {
    /// Best weights found across all iterations.
    pub best: T,
    /// Fitness of the best weights.
    pub best_fitness: f64,
    /// Number of iterations completed.
    pub iterations: usize,
}

/// Per-iteration data emitted to the logger.
pub struct IterationLog {
    /// SPSA iteration index (0-based).
    pub iteration: usize,
    /// Current step-size gain `a_k`.
    pub ak: f64,
    /// Current perturbation-size gain `c_k`.
    pub ck: f64,
    /// Fitness at `θ + c_k·Δ`.
    pub j_plus: f64,
    /// Fitness at `θ − c_k·Δ`.
    pub j_minus: f64,
    /// L2 norm of the estimated gradient.
    pub gradient_norm: f64,
    /// Best fitness seen so far.
    pub best_fitness: f64,
    /// Current parameter vector.
    pub theta: Vec<f64>,
}

/// Run the SPSA optimisation loop.
///
/// Starting from `initial` weights, iteratively perturbs all parameters
/// simultaneously and estimates the gradient from just two fitness
/// evaluations per iteration. Returns the best weights found.
///
/// The `log` callback is invoked after each iteration with diagnostic data.
pub fn run_spsa<T, const N: usize, F>(
    initial: &T,
    config: &SpsaConfig,
    mut log: F,
) -> SpsaResult<T>
where
    T: Tunable,
    F: FnMut(IterationLog),
{
    let p = T::param_count();
    let mut theta = initial.to_vec();
    let mut best_theta = theta.clone();
    let mut best_fitness = f64::NEG_INFINITY;

    let bounds: Vec<(f64, f64)> = (0..p).map(|i| T::param_bounds(i)).collect();

    for k in 0..config.max_iter {
        // Gain sequences: a_k = a0 / (A + k+1)^alpha, c_k = c0 / (k+1)^gamma
        let k1 = k as f64 + 1.0;
        let ak = config.a0 / (config.A + k1).powf(config.alpha);
        let ck = config.c0 / k1.powf(config.gamma);

        // Bernoulli ±1 perturbation vector
        let delta: Vec<f64> = (0..p)
            .map(|_| if rand::random::<bool>() { 1.0 } else { -1.0 })
            .collect();

        // θ ± c_k · Δ
        let theta_plus: Vec<f64> = theta
            .iter()
            .zip(delta.iter())
            .map(|(t, d)| t + ck * d)
            .collect();
        let theta_minus: Vec<f64> = theta
            .iter()
            .zip(delta.iter())
            .map(|(t, d)| t - ck * d)
            .collect();

        // Evaluate both perturbations in parallel
        let model_plus = T::from_vec(&theta_plus);
        let model_minus = T::from_vec(&theta_minus);
        let fitness = &config.fitness;

        let (j_plus, j_minus) = rayon::join(
            || multi_game::<N, _>(&model_plus, fitness),
            || multi_game::<N, _>(&model_minus, fitness),
        );

        // Gradient estimate: g_k = (J+ - J-) / (2c_k) · Δ⁻¹
        // For Bernoulli ±1, Δ⁻¹ = Δ (since 1/±1 = ±1)
        let inv_2c = 1.0 / (2.0 * ck);
        let mut grad_norm = 0.0_f64;
        for &d_i in &delta {
            let gi = (j_plus - j_minus) * inv_2c * d_i;
            grad_norm += gi * gi;
        }
        grad_norm = grad_norm.sqrt();

        // Parameter update: θ_{k+1} = clip(θ_k - a_k · g_k, bounds)
        for i in 0..p {
            let gi = (j_plus - j_minus) * inv_2c * delta[i];
            theta[i] -= ak * gi;
            // Clamp to parameter bounds
            theta[i] = theta[i].clamp(bounds[i].0, bounds[i].1);
        }

        // Estimated fitness at current θ (average of symmetric evaluations)
        let estimated = f64::midpoint(j_plus, j_minus);
        if estimated > best_fitness {
            best_fitness = estimated;
            best_theta.clone_from(&theta);
        }

        log(IterationLog {
            iteration: k,
            ak,
            ck,
            j_plus,
            j_minus,
            gradient_norm: grad_norm,
            best_fitness,
            theta: theta.clone(),
        });
    }

    SpsaResult {
        best: T::from_vec(&best_theta),
        best_fitness,
        iterations: config.max_iter,
    }
}
