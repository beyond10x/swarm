//! What a goal may cost before the loop stops asking.
//!
//! A swarm running the real coordinator spends about a quarter of a dollar every thirty seconds,
//! and a goal whose text cannot be satisfied never reaches `Reached` — so the loop turns until
//! somebody notices. On 2026-09-12 a swarm whose goal was the placeholder `2342342` ran 78 turns
//! and spent $11.10 that way. Nothing in the system said so and nothing stopped it.
//!
//! A cap is a HOST decision, not a model one, and that is why it lives here rather than in the
//! specification. The periodic binding's contract already says `eligibility: host_boolean` — the
//! host decides whether an occurrence runs at all, which is where a paused swarm is excluded. A
//! goal past its cap is excluded the same way.
//!
//! It follows that reaching a cap **changes nothing in the model**. The goal stays in whatever
//! state it was in, no event is appended, and raising the cap resumes the loop exactly where it
//! stopped. A runtime that moved the goal to some `Halted` state would be writing a fact the
//! specification never declared, and a replay would then disagree with the log.
//!
//! ```text
//!   SWARM_MAX_TURNS       default 20      turns of one goal
//!   SWARM_MAX_SPEND_USD   default 5.00    dollars on one goal
//! ```
//!
//! Either may be set to `0` or `off` to lift it. Lifting both restores the old behaviour, which is
//! a loop that stops when the goal is reached or when a person pauses it.

use serde::Serialize;

/// The default turn cap: twenty turns is ten minutes at the declared cadence.
const TURNS: u64 = 20;
/// The default spend cap, in US dollars, per goal.
const SPEND_USD: f64 = 5.00;

/// What one goal may use up.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Caps {
    /// Turns of one goal. `None` means no cap.
    pub max_turns: Option<u64>,
    /// Dollars spent on one goal, as the vendor priced them. `None` means no cap.
    pub max_spend_usd: Option<f64>,
}

impl Default for Caps {
    fn default() -> Self {
        Self {
            max_turns: Some(TURNS),
            max_spend_usd: Some(SPEND_USD),
        }
    }
}

impl Caps {
    /// Reads the caps from the environment, falling back to the defaults.
    ///
    /// A value that does not parse is a mistake worth refusing loudly rather than silently
    /// treating as "no cap", so it keeps the default and says so.
    pub fn configured() -> Self {
        Self {
            max_turns: read("SWARM_MAX_TURNS", TURNS),
            max_spend_usd: read("SWARM_MAX_SPEND_USD", SPEND_USD),
        }
    }

    /// Whether this goal has used up either cap, and which.
    pub fn exceeded(&self, turns: u64, spent_usd: Option<f64>) -> Option<Reached> {
        if let Some(max) = self.max_turns
            && turns >= max
        {
            return Some(Reached::Turns { turns, max });
        }
        if let (Some(max), Some(spent)) = (self.max_spend_usd, spent_usd)
            && spent >= max
        {
            return Some(Reached::Spend { spent, max });
        }
        None
    }
}

/// Which cap a goal reached.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(tag = "cap", rename_all = "snake_case")]
pub enum Reached {
    Turns { turns: u64, max: u64 },
    Spend { spent: f64, max: f64 },
}

impl std::fmt::Display for Reached {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Turns { turns, max } => write!(
                f,
                "the turn cap was reached: {turns} of {max}. The goal is not met; raise \
                 SWARM_MAX_TURNS or abandon it"
            ),
            Self::Spend { spent, max } => write!(
                f,
                "the spend cap was reached: ${spent:.2} of ${max:.2}. The goal is not met; raise \
                 SWARM_MAX_SPEND_USD or abandon it"
            ),
        }
    }
}

/// One cap read from the environment. `0` or `off` lifts it.
fn read<T>(name: &str, fallback: T) -> Option<T>
where
    T: std::str::FromStr + PartialOrd + Default + Copy,
{
    let Ok(raw) = std::env::var(name) else {
        return Some(fallback);
    };
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("off") || raw.eq_ignore_ascii_case("none") {
        return None;
    }
    match raw.parse::<T>() {
        Ok(value) if value <= T::default() => None,
        Ok(value) => Some(value),
        Err(_) => {
            tracing::warn!(
                cap = name,
                value = raw,
                "unreadable cap; keeping the default"
            );
            Some(fallback)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_goal_under_both_caps_runs() {
        let caps = Caps::default();
        assert!(caps.exceeded(3, Some(0.75)).is_none());
        assert!(caps.exceeded(0, None).is_none());
    }

    #[test]
    fn turns_are_checked_before_spend() {
        let caps = Caps::default();
        // The 2026-09-12 run: 78 turns, $11.10. Either cap alone would have stopped it.
        let reached = caps.exceeded(78, Some(11.10)).expect("capped");
        assert!(matches!(reached, Reached::Turns { turns: 78, max: 20 }));
        assert!(matches!(
            caps.exceeded(1, Some(11.10)).expect("capped"),
            Reached::Spend { max: 5.0, .. }
        ));
    }

    #[test]
    fn a_lifted_cap_never_stops_anything() {
        let caps = Caps {
            max_turns: None,
            max_spend_usd: None,
        };
        assert!(caps.exceeded(10_000, Some(1_000.0)).is_none());
    }

    #[test]
    fn an_unpriced_turn_cannot_trip_the_spend_cap() {
        // A coordinator that reports no cost is not a coordinator that spent nothing, so the
        // spend cap abstains rather than guessing. The turn cap still applies.
        let caps = Caps::default();
        assert!(caps.exceeded(1, None).is_none());
        assert!(caps.exceeded(20, None).is_some());
    }
}

#[cfg(test)]
mod retries {
    use super::*;

    /// The hole this file had on the day it was written.
    ///
    /// A turn that is cut off before it writes a verdict leaves the goal where it was, so the next
    /// attempt carries the same number. Measured on the goal's own count, the cap reads 1 on every
    /// pass and never trips; measured on attempts, it trips. Observed live 2026-09-12: attempt one
    /// ran out of vendor turns, attempt two answered, and both were turn 1.
    #[test]
    fn a_turn_that_never_answers_still_counts() {
        let caps = Caps {
            max_turns: Some(3),
            max_spend_usd: None,
        };
        let iterations = 1;
        let attempts = 4;
        assert!(
            caps.exceeded(iterations, None).is_none(),
            "the goal's own count never advances"
        );
        assert!(
            caps.exceeded(iterations.max(attempts), None).is_some(),
            "attempts do"
        );
    }
}
