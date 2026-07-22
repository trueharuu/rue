use triangle::{
  engine::{queue::bag::BagType, utils::KickTable},
  types::{
    events::recv,
    game::{GarbageEntry, GarbageTargetBonus, Passthrough, SpinBonuses},
  },
};

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
fn err(msg: &str) -> ConstraintOutput {
  ConstraintOutput {
    level: ConstraintLevel::Error,
    message: msg.to_string(),
  }
}

/// A handler that checks room settings against a set of constraints.
pub struct SettingsHandler {
  /// The list of constraints to check against room settings.
  constraints: Vec<Constraint>,
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
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.spinbonuses.as_ref())
          .is_some_and(|v| matches!(v, SpinBonuses::AllMiniPlus | SpinBonuses::AllPlus));
        if ok {
          None
        } else {
          Some(err("rue does not support spin bonuses outside of all-mini+ and all+."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.passthrough.as_ref())
          .is_some_and(|v| matches!(v, Passthrough::Zero));
        if ok {
          None
        } else {
          Some(err("rue only supports zero passthrough."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.kickset.as_ref())
          .is_some_and(|v| matches!(v, KickTable::SRSPlus));
        if ok {
          None
        } else {
          Some(err(r#"rue only supports the "SRS+" kick table."#))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.allow_harddrop)
          .unwrap_or(false);
        if ok {
          None
        } else {
          Some(err("rue requires hard drop to be enabled in order to play."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.are) == Some(0);
        if ok {
          None
        } else {
          Some(err(r#"rue only supports "0" ARE."#))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.lineclear_are) == Some(0);
        if ok {
          None
        } else {
          Some(err(r#"rue only supports "0" line clear ARE."#))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.room_handling)
          .is_none_or(|v| !v);
        if ok {
          None
        } else {
          Some(err("rue does not support custom room handling."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.boardwidth) == Some(10);
        if ok {
          None
        } else {
          Some(err("rue currently only supports boards with a width of 10."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.g) == Some(0.0);
        if ok {
          None
        } else {
          Some(err("rue requires 0 gravity."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.gincrease) == Some(0.0);
        if ok {
          None
        } else {
          Some(err("rue requires 0 gravity increase."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.locktime)
          .is_some_and(|v| v > 1);
        if ok {
          None
        } else {
          Some(err("rue requires at least 1 lock delay."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.nolockout)
          .unwrap_or(false);
        if ok {
          None
        } else {
          Some(err("rue does not support lockout."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.stock) == Some(0);
        if ok {
          None
        } else {
          Some(err("rue does not support stock, but will soon."))
        }
      }),
      Box::new(|data| {
        let gamemode = &data.r#match.gamemode;
        let ok = gamemode == "versus" || gamemode == "practice";
        if ok {
          None
        } else {
          Some(err("rue does not properly support royale mode."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.garbagephase) == Some(0);
        if ok {
          None
        } else {
          Some(err("rue does not yet support garbage phasing."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.garbageentry.as_ref())
          .is_some_and(|v| *v == GarbageEntry::Instant);
        if ok {
          None
        } else {
          Some(err("rue does not support non-instant garbage entry."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.garbagequeue)
          .is_none_or(|v| !v);
        if ok {
          None
        } else {
          Some(err("rue does not support garbage queue."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.messiness_timeout) == Some(0.0);
        if ok {
          None
        } else {
          Some(err("rue does not support messiness timeout."))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.bagtype.as_ref())
          .is_none_or(|v| matches!(v, BagType::Bag7));
        if ok {
          None
        } else {
          Some(err(r#"rue does not support the "classic" or "total mayhem" bag types."#))
        }
      }),
      Box::new(|data| {
        let ok = data
          .options
          .as_ref()
          .and_then(|o| o.garbagetargetbonus.as_ref())
          .is_some_and(|v| *v == GarbageTargetBonus::None);
        if ok {
          None
        } else {
          Some(err("rue does not support garbage targeting bonuses."))
        }
      }),
    ]
  }

  /// Checks the given room update against the constraints and returns a `CheckResult` if any constraints are violated.
  pub fn check_room_update(&self, data: &recv::room::Update) -> Option<CheckResult> {
    let outputs: Vec<ConstraintOutput> = self.constraints.iter().filter_map(|c| c(data)).collect();

    if outputs.is_empty() {
      return None;
    }

    let level = outputs
      .iter()
      .fold(outputs[0].level, |acc, o| acc.max(o.level));

    Some(CheckResult { level, outputs })
  }
}
