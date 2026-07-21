//! CSV + terminal logging for SPSA iterations.

use std::io::BufWriter;
use std::path::Path;

use crate::spsa::IterationLog;

/// Writes per-iteration SPSA diagnostics to a CSV file and stdout.
pub struct SpsaLogger {
    writer: csv::Writer<BufWriter<std::fs::File>>,
    param_count: usize,
}

impl SpsaLogger {
    /// Create a new logger writing to `csv_path`.
    ///
    /// The CSV header includes `iteration,ak,ck,j_plus,j_minus,gradient_norm,best_fitness`
    /// followed by `theta_0..theta_{p-1}`.
    #[must_use]
    pub fn create(csv_path: &Path, param_count: usize) -> Self {
        let file = std::fs::File::create(csv_path)
            .unwrap_or_else(|e| panic!("failed to create {}: {e}", csv_path.display()));
        let mut writer = csv::Writer::from_writer(BufWriter::new(file));

        let mut headers: Vec<String> = vec![
            "iteration".into(),
            "ak".into(),
            "ck".into(),
            "j_plus".into(),
            "j_minus".into(),
            "gradient_norm".into(),
            "best_fitness".into(),
        ];
        for i in 0..param_count {
            headers.push(format!("theta_{i}"));
        }

        writer
            .write_record(&headers)
            .expect("failed to write CSV header");

        Self {
            writer,
            param_count,
        }
    }

    /// Create a new logger with human-readable column names from `param_names`.
    pub fn create_with_names(csv_path: &Path, param_names: &[&str]) -> Self {
        let file = std::fs::File::create(csv_path)
            .unwrap_or_else(|e| panic!("failed to create {}: {e}", csv_path.display()));
        let mut writer = csv::Writer::from_writer(BufWriter::new(file));

        let mut headers: Vec<String> = vec![
            "iteration".into(),
            "ak".into(),
            "ck".into(),
            "j_plus".into(),
            "j_minus".into(),
            "gradient_norm".into(),
            "best_fitness".into(),
        ];
        for name in param_names {
            headers.push((*name).to_string());
        }

        writer
            .write_record(&headers)
            .expect("failed to write CSV header");

        Self {
            writer,
            param_count: param_names.len(),
        }
    }

    /// Log one SPSA iteration.
    pub fn log(&mut self, data: &IterationLog) {
        let mut record = vec![
            data.iteration.to_string(),
            format!("{:.6}", data.ak),
            format!("{:.6}", data.ck),
            format!("{:.4}", data.j_plus),
            format!("{:.4}", data.j_minus),
            format!("{:.6}", data.gradient_norm),
            format!("{:.4}", data.best_fitness),
        ];
        for i in 0..self.param_count {
            record.push(format!("{:.6}", data.theta[i]));
        }
        self.writer
            .write_record(&record)
            .expect("failed to write CSV record");

        self.writer.flush().ok();

        eprintln!(
            "iter {:4} | a={:.6} c={:.6} | J+={:.4} J-={:.4} | ‖g‖={:.6} | best={:.4}",
            data.iteration,
            data.ak,
            data.ck,
            data.j_plus,
            data.j_minus,
            data.gradient_norm,
            data.best_fitness,
        );
    }
}

impl Drop for SpsaLogger {
    fn drop(&mut self) {
        self.writer.flush().ok();
    }
}
