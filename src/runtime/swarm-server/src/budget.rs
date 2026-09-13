//! What an agent may cost before the loop stops asking.
//!
//! A swarm running the real coordinator spends about a quarter of a dollar every thirty seconds,
//! and a goal whose text cannot be satisfied never reaches `Reached` — so the loop turns until
//! somebody notices. On 2026-09-12 a swarm whose goal was the placeholder `2342342` ran 78 turns
//! and spent $11.10 that way. Nothing in the system said so and nothing stopped it.
//!
//! **That happened once, not twice, and the record should say so.** The overrun was read as a
//! second instance because the figures differ — $11.10 here and $11.35 elsewhere — but both are the
//! swarm `dsfsdf`, goal `77fc1fcc`, and the difference is one turn. Its `turns/spend.jsonl` holds
//! 44 rows, iterations 35 to 78, `2026-09-12T00:55:37Z` to `01:29:33Z`, summing to $11.345391; the
//! first 43 of them sum to $11.098908, so $11.10 is the same file read at turn 77. There is no
//! other overrun on disk: read at `2026-09-12T12:10Z`, the other five records are `main` 4 rows,
//! `e2e-1789212621` 2, and `cost-check`, `mail-turn-1789197656` and `test` 1 each — none past turn
//! 4 and none past $0.44. An earlier revision of this paragraph said every one of them was a single
//! row, which was false of two and is the sort of round number worth re-counting before writing.
//! `data/` is live, so these grow; the conclusion does not depend on them staying still.
//!
//! So the cap did not fire late and was not lifted. It was **never consulted**, because it did not
//! exist in the binary that ran: this module and both of `capped`'s call sites arrived together in
//! `993731e`, committed `2026-09-12T09:15:35+02:00` — `07:15:35Z`, five hours and 46 minutes after
//! that swarm's last turn. The first 34 turns left no spend row at all for the same reason; the
//! file begins at turn 35 because that is when a binary that records one started.
//!
//! Driven rather than argued: `trigger::bounds` turns the same loop with `SWARM_MAX_TURNS=3` and
//! stops at three, both for a goal that answers and for one that never does. The overrun could not
//! be reproduced against this code, which is the finding and not a gap in it.
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
//!   SWARM_MAX_TURNS       default 20      turns of one agent, across every goal it works
//!   SWARM_MAX_SPEND_USD   default 5.00    dollars of one agent, across every goal it works
//! ```
//!
//! Either may be set to `0`, `off` or `none` to lift it, and to nothing else: a value that is
//! merely wrong — `-1`, `nan`, `inf`, `banana` — keeps the default and says so in a warning. Lifting both
//! restores the old behaviour, which is a loop that stops when the goal is reached or when a person
//! pauses it.

use serde::Serialize;

/// The default turn cap: twenty turns is ten minutes at the declared cadence.
const TURNS: u64 = 20;
/// The default spend cap, in US dollars, per goal.
const SPEND_USD: f64 = 5.00;

/// What one agent may use up, across every goal it works.
///
/// A cap's unit of account is THE AGENT, not the goal — `story:spend-is-bounded-per-goal-only`,
/// 2026-09-12. It was the goal until that day, and a bound keyed on a goal binds a swarm to the
/// number of goals it has rather than to anything an operator chose: one agent at $1.00 a turn
/// over two goals reaches $6.00 against a $5.00 cap with every per-goal fold reporting $3.00 and
/// inside its bounds. Measured, and driven in `trigger::bounds`.
///
/// Two consequences a reader of these numbers has to know, because neither is visible in them:
///
/// * a goal that has used up NOTHING can be refused. `trigger::capped` applies these caps to the
///   goal's record and then to the agent's, and the second can fire on a goal with zero turns and
///   $0.00 of its own because another goal spent the agent's money. That is the bound doing what
///   it exists to do, and it is why what a capped goal publishes are the figures of the bound that
///   fired rather than the goal's own (`budget::Capped`);
/// * an existing setting was TIGHTENED. `SWARM_MAX_SPEND_USD=5` with three goals permitted $15
///   before 2026-09-12 and permits $5 now. Tightening is the direction this story exists to move —
///   the server runs metaharness by default and spends real money every thirty seconds — but an
///   operator whose setting quietly means something new is the harm, so it is written here, on
///   `Server::caps`, on `Status::caps`, and on the `Caps` interface the UI reads.
///
/// What the caps bound is one agent, across every goal it works. There is exactly one agent today,
/// `trigger::COORDINATOR`, so today every row in a swarm is that agent's — a consequence of
/// `story:spawn-a-second-agent` being open, not a design choice, and one that stops being true the
/// day a second agent exists. The unit of account does not change when it does.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Caps {
    /// Turns of one agent, across every goal it works. `None` means no cap.
    pub max_turns: Option<u64>,
    /// Dollars of one agent, across every goal it works, as the vendor priced them. `None` means
    /// no cap.
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
    /// A value that does not parse, that is below zero, or that is not a finite number at all, is
    /// a mistake worth refusing loudly rather than silently treating as "no cap", so it keeps the
    /// default and says so. `read` carries the reason that sentence is worth more than one line —
    /// and the reason it took three attempts to write.
    pub fn configured() -> Self {
        Self {
            max_turns: read("SWARM_MAX_TURNS", TURNS),
            max_spend_usd: read("SWARM_MAX_SPEND_USD", SPEND_USD),
        }
    }

    /// Whether the record these figures were folded from has used up either cap, and which.
    ///
    /// It says nothing about WHOSE figures they are: `trigger::capped` calls this twice, once with
    /// a goal's record and once with an agent's, and [`Bound`] is what carries the difference.
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

impl Reached {
    /// The measurement alone, with no advice and no subject: "the spend cap was reached: $6.00 of
    /// $5.00". [`Display`](std::fmt::Display) adds the advice a goal's own reader wants;
    /// [`Capped::why`] adds the subject when the figures are an agent's.
    /// The environment variable that sets the cap this verdict is about. A reader told to raise a
    /// cap and not told which one has to go and find out.
    pub fn variable(&self) -> &'static str {
        match self {
            Self::Turns { .. } => "SWARM_MAX_TURNS",
            Self::Spend { .. } => "SWARM_MAX_SPEND_USD",
        }
    }

    pub fn measured(&self) -> String {
        match self {
            Self::Turns { turns, max } => format!("the turn cap was reached: {turns} of {max}"),
            Self::Spend { spent, max } => {
                format!("the spend cap was reached: ${spent:.2} of ${max:.2}")
            }
        }
    }
}

impl std::fmt::Display for Reached {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Turns { .. } | Self::Spend { .. } => write!(
                f,
                "{}. The goal is not met; raise {} or abandon it",
                self.measured(),
                self.variable()
            ),
        }
    }
}

/// Which record a bound was measured on.
///
/// `Reached` cannot carry this itself. Its two variants and their field names are pinned by a case
/// this unit may not edit (`tests/the_ceiling_under_attack.rs:148-165` matches
/// `Reached::Spend { spent, max }` and `Reached::Turns { turns, max }` exhaustively), and a third
/// field on either variant stops that file compiling. So the bound rides beside the verdict rather
/// than inside it, and [`Capped`] is the pair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Bound {
    /// The goal's own record: what this goal has used up.
    Goal,
    /// One agent's whole record, across every goal it works. Carries the agent, because a figure
    /// summed over goals is unreadable without the name it was summed for.
    Agent(String),
}

/// A bound that fired, with the figures it fired on.
///
/// The reason this type exists rather than a bare [`Reached`]: a goal stopped by the agent bound
/// was published with its OWN turns and spend — `turns: 2, spent_usd: null` — beside a `why`
/// reading "the turn cap was reached: 4 of 3", which tells a reader to raise a cap that goal used
/// none of. Two folds that no longer agree cannot both be published as one goal's figures, so what
/// is published are the figures of the bound that actually fired.
#[derive(Clone, Debug)]
pub struct Capped {
    /// Whose record the caps were applied to.
    pub bound: Bound,
    /// Which cap was reached, and at what numbers.
    pub reached: Reached,
    /// Turns on that record — the goal's, or the agent's across every goal it works.
    pub turns: u64,
    /// Dollars on that record. `None` when nothing on it was priced.
    pub spent_usd: Option<f64>,
}

impl Capped {
    /// What a reader is told, naming the record the cap was measured on.
    ///
    /// For the goal bound this is [`Reached`]'s own sentence, unchanged. For the agent bound that
    /// sentence would name the wrong subject, so the subject is said out loud: the goal may be
    /// within every bound of its own and still be refused, and a reader who is not told that reads
    /// the figures as this goal's and the cap as this goal's to raise.
    pub fn why(&self) -> String {
        match &self.bound {
            Bound::Goal => self.reached.to_string(),
            Bound::Agent(agent) => format!(
                "{}, by the agent `{agent}` across every goal it works. This goal is within its \
                 own bounds; raise {}, or stop one of the agent's other goals",
                self.reached.measured(),
                self.reached.variable()
            ),
        }
    }
}

/// One cap read from the environment. `0`, `off` or `none` lifts it, and nothing else does.
///
/// Lifting a cap is `== T::default()` and not `<= T::default()`, because the two differ on exactly
/// the values a typo produces. `SWARM_MAX_SPEND_USD=-1` parses as `f64`, is below zero, and under
/// the old comparison removed the spend ceiling outright — while the same typo in `SWARM_MAX_TURNS`
/// does the documented thing, because `u64` refuses to parse it and the default is kept. One
/// character, on one of two caps, silently uncapping the money: that asymmetry is the whole reason
/// this reads the way it does.
///
/// So a negative value joins an unparseable one in the branch this module's own doc promised —
/// refused loudly, default kept. A cap is a bound, and a reader that treats a mistake as "no bound"
/// is answering a question nobody asked it.
///
/// The same argument reaches one value further, and the first version of this function stopped
/// short of it. `==` and `<` are both FALSE of NaN, so `SWARM_MAX_SPEND_USD=nan` was neither lifted
/// nor refused: it fell through and became the ceiling, and `spent >= NaN` is false for every spend
/// there has ever been. `inf` does the same by being a number no spend reaches. Both are caught by
/// [`Cap::is_usable`] BEFORE the comparisons, because a classification that decides by comparison
/// cannot classify a value that compares false with everything.
fn read<T: Cap>(name: &str, fallback: T) -> Option<T> {
    let Ok(raw) = std::env::var(name) else {
        return Some(fallback);
    };
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("off") || raw.eq_ignore_ascii_case("none") {
        return None;
    }
    let refuse = |why: &'static str| {
        tracing::warn!(
            cap = name,
            value = raw,
            why,
            "unusable cap; keeping the default"
        );
        Some(fallback)
    };
    match raw.parse::<T>() {
        Ok(value) if !value.is_usable() => refuse("it is not a finite number"),
        Ok(value) if value == T::default() => None,
        Ok(value) if value < T::default() => refuse("a cap below zero is a mistake, not a lift"),
        Ok(value) => Some(value),
        Err(_) => refuse("it does not parse"),
    }
}

/// A type a cap can be read into.
///
/// The one thing [`read`] cannot do generically is decide whether a parsed value is a number at
/// all. Integers always are; floats have three values that are not — `NaN`, `inf`, `-inf` — and
/// all three defeat a classification written in `==` and `<`.
trait Cap: std::str::FromStr + PartialOrd + Default + Copy {
    /// Whether this value can serve as a bound something might exceed.
    fn is_usable(self) -> bool;
}

impl Cap for u64 {
    fn is_usable(self) -> bool {
        true
    }
}

impl Cap for f64 {
    fn is_usable(self) -> bool {
        self.is_finite()
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
