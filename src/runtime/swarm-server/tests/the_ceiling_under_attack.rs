//! Adversarial cases against unit A of the 2026-09-12d wave
//! (`story:spend-is-bounded-per-goal-only`).
//!
//! The unit added a second bound to `trigger::capped`: after the per-goal check, the same `Caps`
//! are applied to `Swarm::spend_by_agent(agent)`, a fold that asks nothing about the goal. Both
//! call sites pass `trigger::COORDINATOR`, and `COORDINATOR` is the only agent the runtime ever
//! attributes a spend row to — so in every swarm that exists today the second bound is a
//! SWARM-WIDE bound, summed over every goal.
//!
//! That may well be what the story asked for. What it is not is what three unchanged documents
//! say, and it is not what the struct the UI reads reports. Both cases below drive the
//! implementation against a document the unit did not change and the same commit falsified.
//!
//! Neither case constructs an agent name the runtime does not pass: both use
//! `trigger::COORDINATOR`, and both write their rows through `Swarm::record_spend`, which is the
//! door `one_turn` (`trigger.rs:214`) and `take_a_turn` (`coordinator.rs:803`) write through.

use std::path::PathBuf;
use std::sync::Arc;

use ess_runtime::Spec;
use swarm_server::budget::{Bound, Caps, Reached};
use swarm_server::coordinator::Spent;
use swarm_server::swarm::Swarm;
use swarm_server::trigger::{COORDINATOR, bounded, capped};

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

async fn swarm(slug: &str) -> (Arc<Swarm>, tempdir::TempDir) {
    let data = tempdir::TempDir::new("swarm-ceiling-attack").expect("a scratch directory");
    let spec = Arc::new(Spec::load(kernel()).expect("the kernel resolves"));
    let swarm = Swarm::open(spec, data.path(), slug)
        .await
        .expect("a swarm opens");
    (Arc::new(swarm), data)
}

fn cost(usd: f64) -> Spent {
    let mut spent = Spent::default();
    spent.cost_usd = Some(usd);
    spent
}

/// A swarm whose coordinator has spent $6.00 on `goal-one`, and a `goal-two` it has never touched.
///
/// Six rows, all attributed to `COORDINATOR`, all on one goal. This is the runtime's own shape: a
/// swarm with more than one Pursuing goal, which `fire` iterates one row of the AWAITING view at a
/// time, and a coordinator that took its turns on the first of them.
async fn one_spent_goal_and_one_untouched(slug: &str) -> (Arc<Swarm>, tempdir::TempDir) {
    let (swarm, data) = swarm(slug).await;
    for turn in 1..=6 {
        swarm.record_spend(
            COORDINATOR,
            "77fc1fcc-goal-one",
            turn,
            &cost(1.00),
            Some(false),
            None,
        );
    }
    (swarm, data)
}

/// A goal that has used up NOTHING is refused for what its agent spent elsewhere — and every
/// document says so.
///
/// # Rewritten on the coordinator's instruction, 2026-09-13
///
/// This case was `a_cap_is_what_one_goal_may_use_up`, and it asserted `verdict.is_none()` against a
/// list of eleven documents that all said a cap is what ONE GOAL may use up. Its own last line left
/// the fork open — *"NEEDS-CHANGE either way: the behaviour is what the story asked for or the
/// documents are, but they cannot both stand"* — and the coordinator took that fork on 2026-09-13:
/// **the behaviour stands, the documents were wrong.** The caps' unit of account is the agent, one
/// `Caps` governs both folds, and there is no new environment variable
/// (`story:spend-is-bounded-per-goal-only`). F1 of correction round 1 corrected all eleven lines.
///
/// So the premise this case asserted is retired, and what it measures now is the decision: the
/// refusal is real, it is the AGENT bound that makes it, and the documents say what the code does.
/// Nothing was deleted — both preconditions below are the originals, verbatim, and they are what
/// stop this case measuring an empty swarm.
///
/// # The documents, as they read now
///
/// The same eleven lines, in their corrected wording, asserted against the source rather than
/// listed in prose — a list only a reader can check goes stale the next time somebody edits one of
/// them, and the whole finding was that eleven such lines had gone stale together. Revert F1 in any
/// of these files and this case goes red:
///
/// * `budget.rs` — the module title, the `SWARM_MAX_*` table, `Caps`, `max_turns`, `max_spend_usd`
///   and `Caps::exceeded`
/// * `state.rs` — the `caps` field, `Server::caps`, `Status::caps` and `CappedGoal`
/// * `src/web/src/runtime.ts` — the `Caps` interface, a consumer contract in another package
///
/// # What it measures
///
/// `goal-two` has used up neither cap: zero turns of twenty, and nothing at all of $5.00. It is
/// refused anyway, because the agent working it spent the money on `goal-one`. That is the bound
/// doing what the story asked for, and it is the reason the documents had to move.
///
/// What reaches it: `fire` (`trigger.rs:253`) calls this exact function with this exact agent for
/// every row of the AWAITING view, so a second goal in a swarm whose coordinator has spent $5.00
/// elsewhere is made ineligible and never takes a single turn. No flag, no opt-in; it is the
/// default `Caps` and the default coordinator.
#[tokio::test]
async fn a_goal_that_spent_nothing_is_refused_for_what_its_agent_spent() {
    let (swarm, _data) = one_spent_goal_and_one_untouched("per-goal-doc").await;

    // Both preconditions, unchanged from the case this replaces.
    let (spent, turns) = swarm.spend_on("77fc1fcc-goal-two");
    assert_eq!(turns, 0, "goal-two has taken no turn");
    assert_eq!(spent.cost_usd, None, "and has cost nothing");

    let caps = Caps::default();
    let verdict = capped(caps, &swarm, COORDINATOR, "77fc1fcc-goal-two", 0);
    assert!(
        verdict.is_some(),
        "goal-two has used up neither cap of its own — 0 turns of {:?} and $0.00 of {:?} — and is \
         refused anyway, because the agent working it spent $6.00 of the $5.00 on another goal. \
         That is the ceiling the story asked for; nothing refused: {:?}",
        caps.max_turns,
        caps.max_spend_usd,
        verdict
    );

    // WHICH bound refused it. A per-goal bound cannot be the one that fired here, and if it ever
    // is, this case has stopped measuring the ceiling and is measuring something else.
    let bound = bounded(caps, &swarm, COORDINATOR, "77fc1fcc-goal-two", 0)
        .expect("the same refusal, with the figures it was measured on");
    assert_eq!(
        bound.bound,
        Bound::Agent(COORDINATOR.to_owned()),
        "the agent's record is what refused goal-two, not goal-two's: {bound:?}"
    );
    assert!(
        matches!(bound.reached, Reached::Spend { spent, max }
                 if (spent - 6.00).abs() < 1e-9 && (max - 5.00).abs() < 1e-9),
        "at $6.00 of $5.00, summed over every goal the agent works: {bound:?}"
    );

    // And the documents. Eleven lines said a cap is what one goal may use up; a goal with 0 turns
    // and $0.00 being refused is exactly what that wording promised could not happen.
    for (what, text) in documents() {
        for stale in [
            "What one goal may use up",
            "Turns of one goal",
            "Dollars spent on one goal",
            "turns of one goal",
            "dollars on one goal",
        ] {
            assert!(
                !text.contains(stale),
                "{what} still documents a cap as `{stale}`, which the refusal above makes false"
            );
        }
        assert!(
            text.contains("across every goal it works"),
            "{what} does not say what a cap is measured on, and a reader of it would not expect \
             the refusal above"
        );
    }
}

/// The three files whose cap documentation this case holds to the behaviour, with their text.
///
/// Read from the source rather than quoted into a doc comment, because the finding these two cases
/// come from was eleven quoted lines that had all gone stale at once.
fn documents() -> Vec<(&'static str, String)> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        ("budget.rs", crate_root.join("src/budget.rs")),
        ("state.rs", crate_root.join("src/state.rs")),
        (
            "src/web/src/runtime.ts",
            crate_root.join("../../web/src/runtime.ts"),
        ),
    ]
    .into_iter()
    .map(|(what, at)| {
        let text = std::fs::read_to_string(&at)
            .unwrap_or_else(|why| panic!("{what} is readable at {}: {why}", at.display()));
        (what, text)
    })
    .collect()
}

/// What a capped goal PUBLISHES belongs to the bound that fired.
///
/// # Rewritten on the coordinator's instruction, 2026-09-13
///
/// This case was `a_capped_goal_reports_what_that_goal_spent`. It reproduced `report_capped`'s
/// composition inside its own body — `turns` and `spent_usd` from `swarm.spend_on(goal)`, `reached`
/// from `capped` — and asserted that the two agreed, which they did not: a goal that had spent
/// nothing was published as `{"turns": 0, "spent_usd": null}` beside `"the spend cap was reached:
/// $6.00 of $5.00"`. That was F2 of correction round 1, and it was confirmed and fixed: the
/// figures published are now the ones the bound that fired was measured on
/// (`trigger::report_capped`, `budget::Capped`).
///
/// The old assertion cannot be satisfied by any implementation — it demands that a fold over
/// `goal-two`, which has no rows, equal a figure summed over an agent that spent $6.00 — and it
/// reproduced a composition the code no longer performs. Its target moved, so the coordinator
/// instructed this rewrite on 2026-09-13. Nothing was deleted: the same fixture, the same goal, the
/// same match over `Reached`'s two variants, and the same question — do the figures published beside
/// a refusal describe the thing that was measured?
///
/// # Through the public surface, not around it
///
/// `report_capped` is private, but what it composes from is not: `trigger::bounded` returns
/// `budget::Capped { bound, reached, turns, spent_usd }`, and `report_capped`'s four fields are
/// those four. So this case drives the real composition rather than reproducing it, which is
/// exactly what the case it replaces could not do.
///
/// The one step it cannot reach from here is the publication itself — `Server::report_capped`,
/// `capped_goals()`, `Status.capped` and `What::Capped`. That is carried by
/// `trigger::bounds::an_agent_is_stopped_by_its_own_record_across_two_goals`, an in-crate case that
/// drives two goals through `fire` and `ask_the_coordinator` and reads the published `CappedGoal`
/// off `server.capped_goals()`.
///
/// What reaches it: `fire` (`trigger.rs:253-257`) on any swarm with a second goal, into
/// `Server::report_capped` (`state.rs`), into `capped_goals()`, into `Status.capped`, into
/// `CappedGoal` in `src/web/src/runtime.ts` and onto the watch stream as `What::Capped`.
#[tokio::test]
async fn a_capped_goal_reports_the_figures_the_bound_was_measured_on() {
    let (swarm, _data) = one_spent_goal_and_one_untouched("capped-report").await;

    // The goal `fire` would name, and its own figures — which are NOT what may be published.
    let goal = "77fc1fcc-goal-two";
    let turns = 0_u64;
    let (its_own, its_turns) = swarm.spend_on(goal);
    assert_eq!(
        (its_turns, its_own.cost_usd),
        (0, None),
        "goal-two spent nothing of its own"
    );

    let bound = bounded(Caps::default(), &swarm, COORDINATOR, goal, turns)
        .expect("the ceiling refuses goal-two; if it does not, this case has stopped measuring");

    // The four fields `report_capped` publishes, against the fold the bound was measured on.
    let (measured, counted) = match &bound.bound {
        Bound::Goal => swarm.spend_on(goal),
        Bound::Agent(agent) => swarm.spend_by_agent(agent),
    };
    assert_eq!(
        (bound.turns, bound.spent_usd),
        (counted, measured.cost_usd),
        "a CappedGoal published for `{goal}` carries the figures of the bound that fired \
         ({:?}), not another fold's: {bound:?}",
        bound.bound
    );

    // And they agree with the numbers inside the verdict, which is the inconsistency the case this
    // replaces measured: same match, same two variants, now against the published figures.
    match bound.reached {
        Reached::Spend { spent, max } => assert_eq!(
            bound.spent_usd,
            Some(spent),
            "a CappedGoal published for `{goal}` says it reached the spend cap at ${spent:.2} of \
             ${max:.2}, and must publish that same ${spent:.2} rather than a different fold's \
             figure. `why` reads: {}",
            bound.why()
        ),
        Reached::Turns {
            turns: counted,
            max,
        } => assert_eq!(
            bound.turns, counted,
            "a CappedGoal published for `{goal}` says it reached the turn cap at {counted} of \
             {max}, and must publish that same count"
        ),
    }

    // A reader must be able to tell that these figures are not this goal's. `why` is the only field
    // that can say so, and for the agent bound it names the agent and the goal's own standing.
    if let Bound::Agent(agent) = &bound.bound {
        let why = bound.why();
        assert!(
            why.contains(agent),
            "the reason names whose record the cap was measured on: {why}"
        );
        assert_ne!(
            bound.spent_usd, its_own.cost_usd,
            "and it is not this goal's figure that was published: {bound:?}"
        );
    }
}
