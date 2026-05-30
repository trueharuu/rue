use std::fmt::Display;

pub struct Difference(pub f64, pub f64);

impl Display for Difference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dd = self.0 - self.1;
        if dd < 0.0 {
            write!(f, "\x1b[31m{dd:+.3}\x1b[0m")
        } else if dd > 0.0 {
            write!(f, "\x1b[32m{dd:+.3}\x1b[0m")
        } else {
            write!(f, "{dd:+.3}")
        }
    }
}
