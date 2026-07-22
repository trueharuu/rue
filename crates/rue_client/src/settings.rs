//! Settings handler for Rue.
use triangle::engine::queue::bag::BagType;
use triangle::engine::utils::KickTable;
use triangle::types::events::recv;
use triangle::types::game::{GarbageEntry, GarbageTargetBonus, Passthrough, SpinBonuses};

/// Result level of a constraint check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
pub enum ConstraintLevel {
    Info,
    Change,
    Warning,
    Error,
}

impl std::fmt::Display for ConstraintLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Change => write!(f, "change"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}
/// The output of a constraint check.
#[derive(Debug, Clone)]
pub struct ConstraintOutput {
    /// The level of the raised check.
    pub level: ConstraintLevel,
    /// The message associated with the raised check.
    pub message: String,
    /// The option to change to fix the check.
    pub fix: String,
}

/// The result of a settings check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// The highest level of the raised checks.
    pub level: ConstraintLevel,
    /// The outputs of the raised checks.
    pub outputs: Vec<ConstraintOutput>,
}

/// A constraint function that checks room settings and returns an optional `ConstraintOutput`.
type Constraint = Box<dyn Fn(&recv::room::Update) -> Option<ConstraintOutput> + Send + Sync>;

/// Creates a new error constraint output.
fn err(msg: &str, fix: &str) -> ConstraintOutput {
    ConstraintOutput {
        level: ConstraintLevel::Error,
        message: msg.to_string(),
        fix: fix.to_string(),
    }
}

/// A handler that checks room settings against a set of constraints.
pub struct SettingsHandler {
    /// The list of constraints to check against room settings.
    constraints: Vec<Constraint>,
}

/// A macro to create a constraint function that checks a specific field in the room settings.
macro_rules! constraint {
    ($o:ident $t:expr => $e:pat, $m:expr, $f:expr) => {
        Box::new(|data| {
            #[allow(clippy::redundant_pattern_matching)]
            let ok = data
                .options
                .as_ref()
                .and_then(|$o| $t)
                .is_some_and(|v| matches!(v, $e));
            if ok { None } else { Some(err($m, $f)) }
        })
    };
}

impl SettingsHandler {
    /// Creates a new `SettingsHandler` with default constraints.
    pub fn new() -> Self {
        Self {
            constraints: Self::default_constraints(),
        }
    }

    /// Returns a list of default constraints that Rue enforces on room settings.
    fn default_constraints() -> Vec<Constraint> {
        vec![
            constraint!(o o.spinbonuses => SpinBonuses::AllMiniPlus | SpinBonuses::AllPlus, "spin bonuses must be all-mini+ or all+", "options.spinbonuses=all-mini+"),
            constraint!(o o.passthrough => Passthrough::Zero, "passthrough must be zero", "options.passthrough=zero"),
            constraint!(o o.kickset => KickTable::SRSPlus, "kick table must be SRS+", "options.kickset=SRS+"),
            constraint!(o o.allow_harddrop => true, "hard drop must be enabled", "options.allow_harddrop=1"),
            constraint!(o o.are => 0, "ARE must be 0", "options.are=0"),
            constraint!(o o.lineclear_are => 0, "line clear ARE must be 0", "options.lineclear_are=0"),
            constraint!(o o.room_handling => false, "custom room handling must be disabled", "options.room_handling=0"),
            constraint!(o o.boardwidth => 10, "board width must be 10", "options.boardwidth=10"),
            constraint!(o o.g => 0.0, "gravity must be 0", "options.g=0.0"),
            constraint!(o o.gincrease => 0.0, "gravity increase must be 0", "options.gincrease=0.0"),
            constraint!(o o.nolockout => true, "lockout must be disabled", "options.nolockout=1"),
            constraint!(o o.stock => 0, "stock must be 0", "options.stock=0"),
            constraint!(o o.garbagephase => 0, "garbage phase must be 0", "options.garbagephase=0"),
            // constraint!(o o.garbageentry => GarbageEntry::Instant, "garbage entry must be instant"),
            constraint!(o o.garbagequeue => false, "garbage queue must be disabled", "options.garbagequeue=0"),
            constraint!(o o.messiness_timeout => 0.0, "messiness timeout must be 0", "options.messiness_timeout=0"),
            constraint!(o o.bagtype => BagType::Bag7, "bag type must be 7-bag", "options.bagtype=7-bag"),
            constraint!(o o.garbagetargetbonus => GarbageTargetBonus::None, "garbage target bonus must be none", "options.garbagetargetbonus=none"),
        ]
    }

    /// Checks the given room update against the constraints and returns a `CheckResult` if any constraints are violated.
    pub fn check_room_update(&self, data: &recv::room::Update) -> Option<CheckResult> {
        let outputs: Vec<ConstraintOutput> =
            self.constraints.iter().filter_map(|c| c(data)).collect();

        if outputs.is_empty() {
            return None;
        }

        let level = outputs
            .iter()
            .fold(outputs[0].level, |acc, o| acc.max(o.level));

        Some(CheckResult { level, outputs })
    }
}
