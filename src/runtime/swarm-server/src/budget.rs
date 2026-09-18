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
//!   SWARM_MAX_TURNS            default 20      turns of one agent, across every goal it works
//!   SWARM_MAX_SPEND_USD        default 5.00    dollars of one agent, across every goal it works
//!   SWARM_MAX_IN_FLIGHT        default 4       turns of one swarm at once, over every agent in it
//!   SWARM_MAX_TOTAL_SPEND_USD  default 20.00   dollars of one swarm, over every agent in it
//! ```
//!
//! Any of them may be set to `0`, `off` or `none` to lift it, and to nothing else: a value that is
//! merely wrong — `-1`, `nan`, `inf`, `banana` — keeps the default and says so in a warning. Lifting them
//! all restores the old behaviour, which is a loop that stops when the goal is reached or when a
//! person pauses it.
//!
//! # The last two are the swarm's, and they exist because the first two are not
//!
//! The first two bound ONE AGENT. Under them a swarm's worst case is the number of agents in it
//! times $5.00, and nothing in this file used to bound that number: `trigger` spawned one task per
//! Pursuing goal and one per open assignment, and no code anywhere counted them. At breadth one
//! that is invisible. At breadth ten it is the run above, ten times, with every per-agent fold
//! reporting itself inside its bounds — the same shape as the per-goal defect of 2026-09-12, one
//! level up, and it arrives the day a swarm runs more than one worker at once
//! (`story:run-two-workers-at-once`).
//!
//! So the swarm is a unit of account too, and it is bounded twice: how many turns it may have IN
//! FLIGHT at one moment, and how many dollars it may have spent ACROSS ALL of them. The two numbers
//! agree on purpose — four turns at once, each of them an agent that may reach $5.00, is $20.00 of
//! exposure, which is what `SWARM_MAX_TOTAL_SPEND_USD` bounds.
//!
//! The breadth ceiling is not a refusal and is not recorded as one: a turn that does not start
//! because the swarm is full is taken at the next period, nothing in the model changes, and nothing
//! is published. The swarm-wide spend cap IS a refusal — the money is gone and no later period
//! makes it come back — so it is published like every other bound that fires, through
//! `trigger::report_capped`, and `turns/capped.jsonl` names it: `"bound": "swarm"`, `"cap":
//! "SWARM_MAX_TOTAL_SPEND_USD"`. A record that named `SWARM_MAX_SPEND_USD` there would send its
//! reader to raise a cap that was never reached.
//!
//! Both are checked BEFORE a turn is launched, which is the property the $0.00 refusal in
//! `examples/two-agents/evidence/` has and which a cap checked afterwards cannot: a bound that
//! fires once the money is spent is a bound that has not stopped anything.

use serde::Serialize;

/// The default turn cap: twenty turns is ten minutes at the declared cadence.
const TURNS: u64 = 20;
/// The default spend cap, in US dollars, per goal.
const SPEND_USD: f64 = 5.00;
/// The default breadth: four turns of one swarm running at the same moment.
///
/// Four rather than one, because a swarm that may run only one turn at a time is a coordinator
/// with extra steps and `story:run-two-workers-at-once` exists to end that. Four rather than
/// unbounded, because the number of tasks `trigger` spawns is the number of units of work a
/// coordinator has posted, and a coordinator writes that number itself.
const IN_FLIGHT: u64 = 4;
/// The default swarm-wide spend cap, in US dollars, across every agent of one swarm.
///
/// [`IN_FLIGHT`] times [`SPEND_USD`]: the exposure the breadth ceiling admits at one moment, if
/// every turn in flight belonged to a different agent and every one of those agents were at its
/// own cap.
const TOTAL_SPEND_USD: f64 = IN_FLIGHT as f64 * SPEND_USD;

/// The variable that sets the swarm-wide spend cap. Named once, because it is read in two places
/// and written into a durable record in a third.
const TOTAL_SPEND: &str = "SWARM_MAX_TOTAL_SPEND_USD";
/// The variable that sets the breadth ceiling.
const BREADTH: &str = "SWARM_MAX_IN_FLIGHT";

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
    /// Turns of one swarm running at the same moment, over every agent in it. `None` means no
    /// ceiling, which is what this runtime had until `story:run-two-workers-at-once`.
    pub max_in_flight: Option<u64>,
    /// Dollars of one swarm, over every agent in it, as the vendor priced them. `None` means no
    /// cap.
    pub max_total_spend_usd: Option<f64>,
}

impl Default for Caps {
    fn default() -> Self {
        Self {
            max_turns: Some(TURNS),
            max_spend_usd: Some(SPEND_USD),
            max_in_flight: Some(IN_FLIGHT),
            max_total_spend_usd: Some(TOTAL_SPEND_USD),
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
            max_in_flight: read(BREADTH, IN_FLIGHT),
            max_total_spend_usd: read(TOTAL_SPEND, TOTAL_SPEND_USD),
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

    /// Whether one whole swarm — every agent in it, every unit they work — has spent what it may.
    ///
    /// A separate question from [`Self::exceeded`] and not a second call to it: the number this
    /// compares against is `SWARM_MAX_TOTAL_SPEND_USD`, and a verdict that carried the per-agent
    /// figure would tell its reader to raise a cap nothing reached. There is deliberately no turn
    /// half — turns summed over a swarm bound nothing an operator can act on, because a swarm's
    /// turn count grows with the agents in it, and how many of those may run at once is
    /// [`Self::at_breadth`]'s question.
    pub fn swarm_exceeded(&self, spent_usd: Option<f64>) -> Option<Reached> {
        match (self.max_total_spend_usd, spent_usd) {
            (Some(max), Some(spent)) if spent >= max => Some(Reached::Spend { spent, max }),
            _ => None,
        }
    }

    /// Whether a swarm already holds as many turns at once as it may, and the ceiling it is at.
    ///
    /// Asked with the count of turns this swarm has IN FLIGHT, before another is launched. `>=`
    /// rather than `>` for the reason [`Self::exceeded`] uses it: the count is what is already
    /// running, so a swarm at its ceiling is full and the next turn is one too many.
    ///
    /// The answer is not a [`Reached`] and never becomes a [`Capped`]. Being full is not a refusal
    /// — the turn is taken at the next period, nothing was spent, and nothing in the model changed
    /// — and recording it as one would fill `capped.jsonl` with rows that say a swarm was stopped
    /// when it was merely busy.
    pub fn at_breadth(&self, in_flight: u64) -> Option<u64> {
        self.max_in_flight.filter(|max| in_flight >= *max)
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
    /// One swarm's whole record: every agent in it, every unit they work.
    ///
    /// Carries no name, and that is not an oversight. A figure summed over agents is read off ONE
    /// swarm and every record it reaches already says which — `CappedGoal::swarm`, and the
    /// `capped.jsonl` it is written to is that swarm's own file.
    Swarm,
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
    /// The environment variable that sets the bound that FIRED.
    ///
    /// Not [`Reached::variable`] for every bound, and the difference is the whole reason this
    /// method exists. A swarm stopped by `SWARM_MAX_TOTAL_SPEND_USD` reaches a `Reached::Spend`,
    /// whose own `variable()` says `SWARM_MAX_SPEND_USD` — a different number, which nothing
    /// reached, and raising which would not release the swarm by a cent. `Reached` cannot tell the
    /// two apart because it carries the measurement and not the subject; [`Bound`] is the subject,
    /// so the pair decides.
    pub fn variable(&self) -> &'static str {
        match (&self.bound, &self.reached) {
            (Bound::Goal | Bound::Agent(_), reached) => reached.variable(),
            (Bound::Swarm, Reached::Spend { .. }) => TOTAL_SPEND,
            (Bound::Swarm, Reached::Turns { .. }) => BREADTH,
        }
    }

    /// What a reader is told, naming the record the cap was measured on.
    ///
    /// For the goal bound this is [`Reached`]'s own sentence, unchanged. For the agent bound that
    /// sentence would name the wrong subject, so the subject is said out loud: the goal may be
    /// within every bound of its own and still be refused, and a reader who is not told that reads
    /// the figures as this goal's and the cap as this goal's to raise.
    ///
    /// For the swarm bound both of those are true at once — the unit is inside its bounds and so
    /// is the agent working it — so the sentence says whose money it was and which of the four
    /// variables moves it.
    pub fn why(&self) -> String {
        match &self.bound {
            Bound::Goal => self.reached.to_string(),
            Bound::Agent(agent) => format!(
                "{}, by the agent `{agent}` across every goal it works. This goal is within its \
                 own bounds; raise {}, or stop one of the agent's other goals",
                self.reached.measured(),
                self.reached.variable()
            ),
            Bound::Swarm => format!(
                "{}, by this swarm across every agent in it. This unit and the agent working it \
                 are each within their own bounds; raise {}, or stop the swarm",
                self.reached.measured(),
                self.variable()
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
            max_in_flight: None,
            max_total_spend_usd: None,
        };
        assert!(caps.exceeded(10_000, Some(1_000.0)).is_none());
        assert!(caps.swarm_exceeded(Some(1_000.0)).is_none());
        assert!(caps.at_breadth(1_000).is_none());
    }

    /// The swarm bound fires on a swarm no agent of which is over anything.
    ///
    /// Four agents at $2.00 each is $8.00 against a per-agent cap of $5.00 that every one of them
    /// is inside, and that is the shape the per-agent caps cannot see: a bound keyed on the agent
    /// binds a swarm to the number of agents it has, exactly as a bound keyed on the goal bound it
    /// to the number of goals before 2026-09-12.
    #[test]
    fn a_swarm_can_be_over_while_every_agent_in_it_is_inside() {
        let caps = Caps {
            max_turns: None,
            max_spend_usd: Some(5.00),
            max_in_flight: Some(4),
            max_total_spend_usd: Some(5.00),
        };
        for agent in [2.00_f64; 4] {
            assert!(
                caps.exceeded(1, Some(agent)).is_none(),
                "every agent is inside its own cap"
            );
        }
        let reached = caps.swarm_exceeded(Some(8.00)).expect("the swarm is not");
        assert!(matches!(reached, Reached::Spend { spent, max }
                         if (spent - 8.00).abs() < 1e-9 && (max - 5.00).abs() < 1e-9));
        assert!(
            caps.swarm_exceeded(Some(4.99)).is_none(),
            "and a swarm under it runs"
        );
        assert!(
            caps.swarm_exceeded(None).is_none(),
            "a swarm nothing in which was priced is not a swarm that spent nothing"
        );
    }

    /// The retained record names the bound that fired, not the one whose shape it shares.
    ///
    /// Both bounds reach `Reached::Spend`, whose own `variable()` is the per-agent cap. A record
    /// that published that for a swarm bound would send a reader to raise $5.00 while $20.00 was
    /// what refused them.
    #[test]
    fn a_swarm_bound_names_its_own_variable() {
        let capped = Capped {
            bound: Bound::Swarm,
            reached: Reached::Spend {
                spent: 8.00,
                max: 5.00,
            },
            turns: 4,
            spent_usd: Some(8.00),
        };
        assert_eq!(capped.variable(), "SWARM_MAX_TOTAL_SPEND_USD");
        assert_ne!(
            capped.variable(),
            capped.reached.variable(),
            "which is NOT the variable the measurement alone would name"
        );
        let why = capped.why();
        assert!(why.contains("SWARM_MAX_TOTAL_SPEND_USD"), "{why}");
        assert!(
            why.contains("across every agent in it"),
            "and says what it was summed over: {why}"
        );

        // The two bounds that were here before still name theirs.
        let agent = Capped {
            bound: Bound::Agent("coordinator".to_owned()),
            reached: Reached::Spend {
                spent: 6.00,
                max: 5.00,
            },
            turns: 6,
            spent_usd: Some(6.00),
        };
        assert_eq!(agent.variable(), "SWARM_MAX_SPEND_USD");
        assert_eq!(
            Capped {
                bound: Bound::Goal,
                reached: Reached::Turns { turns: 20, max: 20 },
                turns: 20,
                spent_usd: None,
            }
            .variable(),
            "SWARM_MAX_TURNS"
        );
    }

    /// A swarm is full at its ceiling, not one turn after it.
    #[test]
    fn a_swarm_at_its_breadth_admits_no_further_turn() {
        let caps = Caps::default();
        assert_eq!(caps.max_in_flight, Some(4));
        assert!(caps.at_breadth(0).is_none(), "an idle swarm admits a turn");
        assert!(
            caps.at_breadth(3).is_none(),
            "and so does one with three running, which is the whole point of the ceiling"
        );
        assert_eq!(caps.at_breadth(4), Some(4), "the fourth fills it");
        assert_eq!(caps.at_breadth(9), Some(4), "and it stays full");
        assert!(
            Caps {
                max_in_flight: None,
                ..Caps::default()
            }
            .at_breadth(1_000)
            .is_none(),
            "a lifted ceiling stops nothing, which is what this runtime did before it existed"
        );
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
            ..Caps::default()
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
