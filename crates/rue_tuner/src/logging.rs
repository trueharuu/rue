//! CSV and terminal progress logging for SPSA tuning.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use rue_eval::tunable::Tunable;

/// Append-only CSV logger for SPSA iterations.
pub struct TuningLog {
    /// Buffered CSV writer.
    writer: BufWriter<File>,
}

impl TuningLog {
    /// Create a new log, writing `tuning.csv` into `dir`.
    pub fn new(dir: &Path) -> Self {
        let path = dir.join("tuning.csv");
        let file = File::create(&path).expect("failed to create tuning.csv");
        let mut writer = BufWriter::new(file);

        // header
        write!(writer, "iter,a_k,c_k,j_plus,j_minus").unwrap();
        // We don't know param_count at this point without a generic, so we
        // write param columns on the first call instead. For now, write a
        // fixed header that the caller can extend.
        writeln!(writer).unwrap();

        Self { writer }
    }

    /// Append one iteration's data.
    pub fn append<W: Tunable>(
        &mut self,
        iter: usize,
        a_k: f64,
        c_k: f64,
        j_plus: f64,
        j_minus: f64,
        theta: &W,
    ) {
        write!(
            self.writer,
            "{iter},{a_k:.6},{c_k:.6},{j_plus:.4},{j_minus:.4}"
        )
        .unwrap();

        for i in 0..W::param_count() {
            write!(self.writer, ",{:.6}", theta.get_param(i)).unwrap();
        }

        writeln!(self.writer).unwrap();
    }
}
