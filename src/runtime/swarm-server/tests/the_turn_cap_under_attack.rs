//! Adversarial cases against unit A of the 2026-09-12c wave (`story:the-turn-cap-did-not-hold`).
//!
//! The unit shipped three claims. Two of them are documents it wrote about itself, and nothing in
//! the suite it wrote drives the implementation against them:
//!
//! * `swarm.rs`'s attribution case asserts "every recorded turn names the agent that spent it" —
//!   over rows the case wrote itself, through the attributed door. The door an ANSWERED turn goes
//!   through is `coordinator.rs:610`, which is still `record_spend`, so the sentence is false of
//!   every successful turn the runtime takes. `an_answered_turn_names_the_agent_that_spent_it`
//!   drives the real coordinator path and reads the row it wrote.
//! * `budget.rs`'s `read` says "`0` or `off` lifts it", and `Caps::configured` says a mistaken
//!   value is "worth refusing loudly rather than silently treating as no cap". A negative spend cap
//!   is neither `0` nor `off` and is silently treated as no cap.
//!   `a_negative_spend_cap_is_not_a_lifted_spend_cap` drives `Caps::configured` from the
//!   environment, which the unit's own `bounds` case stopped doing when `capped` moved from
//!   `&Server` to `Caps`.
//! * The story's `## Acceptance` has four clauses. The fourth — "a ceiling exists that does not
//!   depend on which goal an agent is working" — is unmet, and
//!   `an_agents_spend_is_not_bounded_across_the_goals_it_works` PINS THAT ABSENCE: it asserts what
//!   the crate does today, so it goes red the moment the ceiling lands and somebody has to come
//!   back and turn it into the assertion it wants to be.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value as Json;

use ess_runtime::Spec;
use swarm_server::budget::Caps;
use swarm_server::coordinator::Spent;
use swarm_server::swarm::Swarm;

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

async fn swarm(slug: &str) -> (Arc<Swarm>, tempdir::TempDir) {
    let data = tempdir::TempDir::new("swarm-cap-attack").expect("a scratch directory");
    let spec = Arc::new(Spec::load(kernel()).expect("the kernel resolves"));
    let swarm = Swarm::open(spec, data.path(), slug)
        .await
        .expect("a swarm opens");
    (Arc::new(swarm), data)
}

/// Every row of a swarm's spend record, as JSON.
fn rows(swarm: &Arc<Swarm>) -> Vec<Json> {
    let at = swarm.dir().join("turns").join("spend.jsonl");
    let Ok(text) = std::fs::read_to_string(at) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn cost(usd: f64) -> Spent {
    let mut spent = Spent::default();
    spent.cost_usd = Some(usd);
    spent
}

/// The claim: "every recorded turn names the agent that spent it" (`swarm.rs`, the unit's own case).
///
/// Driven against the path a turn that ANSWERS actually takes — `coordinator::take_a_turn`, with a
/// program coordinator satisfying the documented stdin/stdout contract, exactly as
/// `examples/coordinator-manual.sh` does. The unit's attribution case never reaches this line: it
/// writes its three rows through `record_spend_by` itself, so it proves the attributed door
/// attributes and says nothing about which door the runtime uses.
///
/// What reaches it: every successful coordinator turn. `one_turn` records spend itself only on the
/// UNFINISHED branch; the answered branch records at `coordinator.rs:610`, which is `record_spend`,
/// which writes `"agent": null`.
#[tokio::test]
async fn an_answered_turn_names_the_agent_that_spent_it() {
    let (swarm, data) = swarm("answered").await;

    // A coordinator that satisfies the contract in `examples/coordinator-manual.sh`: read the
    // question on stdin, print a verdict on stdout, and nothing else.
    let program = data.path().join("coordinator.sh");
    std::fs::write(
        &program,
        "#!/bin/sh\ncat > /dev/null\nprintf '{\"reached\": false, \"note\": \"not yet\"}\\n'\n",
    )
    .expect("the coordinator is written");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))
        .expect("the coordinator is runnable");
    // SAFETY: this test binary holds no other case that reads SWARM_COORDINATOR.
    unsafe { std::env::set_var("SWARM_COORDINATOR", &program) };

    let goal = "77fc1fcc-a89c-4fb9-b041-89ecfc75922f";
    swarm
        .ensure_coordinator("coordinator")
        .await
        .expect("the coordinator is a record");

    // The verdict cannot be reported — there is no goal instance to evaluate — but the spend row is
    // written before that, on the same line the runtime writes it on.
    let _ = swarm_server::coordinator::take_a_turn(&swarm, "coordinator", goal, "a goal", 1).await;

    let recorded = rows(&swarm);
    assert_eq!(recorded.len(), 1, "the answered turn was recorded");
    assert_eq!(
        recorded[0].get("agent").and_then(Json::as_str),
        Some("coordinator"),
        "every recorded turn names the agent that spent it, including one that answered: {}",
        recorded[0]
    );
}

/// The claim: "`0` or `off` lifts it" (`budget.rs`, on `read`), and that a mistaken value is
/// "worth refusing loudly rather than silently treating as no cap" (`Caps::configured`).
///
/// `-1` is neither `0` nor `off`, and it is a mistake. It lifts the spend cap outright, so the
/// module whose whole purpose is to stop an unbounded spend stops nothing. The same typo in
/// `SWARM_MAX_TURNS` does the documented thing — `u64` refuses to parse it, so the default is kept
/// — which is the asymmetry this case measures.
///
/// What reaches it: the environment, which is the only way either cap is set
/// (`story:the-turn-cap-did-not-hold`, `## Out of scope`).
#[test]
fn a_negative_spend_cap_is_not_a_lifted_spend_cap() {
    // SAFETY: this test binary holds no other case that reads either cap from the environment.
    unsafe {
        std::env::set_var("SWARM_MAX_SPEND_USD", "-1");
        std::env::set_var("SWARM_MAX_TURNS", "-1");
    }
    let caps = Caps::configured();
    assert_eq!(
        caps.max_turns,
        Some(20),
        "an unparseable turn cap keeps the default, as Caps::configured documents"
    );
    assert!(
        caps.max_spend_usd.is_some(),
        "a mistyped spend cap is refused rather than silently read as no cap: got {:?}",
        caps.max_spend_usd
    );
}

/// The story's fourth acceptance clause, PINNED AT ITS CURRENT — WRONG — ANSWER.
///
/// `story:the-turn-cap-did-not-hold` asks for "a ceiling exists that does not depend on which goal
/// an agent is working". There is none, and this case asserts that there is none. It is not
/// `#[ignore]`d, because a case that pre-excuses its own red is one nobody reads again; and it is
/// not left failing, because `cargo test` reads one exit status and a single deliberate red deletes
/// every result after it — nine other lanes stop reporting anything.
///
/// **So it is red-when-fixed rather than red-now.** The moment a ceiling refuses this agent, the
/// assertion below flips and whoever landed it is sent here.
///
/// # What to change when the ceiling lands
///
/// The ceiling belongs in `state.rs` — `turns_in_flight()` (`state.rs:201`) is the only
/// server-wide count of anything an agent is doing, and it reports to the UI and refuses nothing.
/// A ceiling reading `Swarm::spend_by` across every goal is the missing half. When it exists:
/// rename this case back to `an_agents_spend_is_bounded_across_the_goals_it_works`, and change
/// `assert!(!refused, ...)` to `assert!(refused, ...)`. Nothing else in it needs to move.
///
/// # What it measures
///
/// One agent, two goals, three turns and $1.00 each: **6 turns and $6.00 against a $5.00 cap**.
/// Every goal is comfortably inside both bounds on its own, so the only refusal surface the crate
/// has — `Caps::exceeded` folded over `spend_on`, which is exactly what `trigger::capped` folds —
/// never fires. Attribution was added in the 2026-09-12c wave and nothing reads it to refuse:
/// `spend_by` needs a goal to be given, so there is no per-agent total to bound.
///
/// What reaches it: a swarm with more than one goal. `fire` iterates every row of the AWAITING
/// view, and each is capped alone.
#[tokio::test]
async fn an_agents_spend_is_not_bounded_across_the_goals_it_works() {
    let (swarm, _data) = swarm("across-goals").await;
    let goals = ["77fc1fcc-goal-one", "77fc1fcc-goal-two"];
    let agent = "worker-a";

    for goal in goals {
        for turn in 1..=3 {
            swarm.record_spend(agent, goal, turn, &cost(1.00), Some(false), None);
        }
    }

    // The agent's own figure, which attribution now makes readable — and which nothing reads.
    let (spent, turns) = goals.iter().fold((0.0, 0), |(usd, count), goal| {
        let (spent, turns) = swarm.spend_by(goal, agent);
        (usd + spent.cost_usd.unwrap_or_default(), count + turns)
    });
    assert_eq!(turns, 6, "the agent took six turns");
    assert!((spent - 6.00).abs() < 1e-9, "and spent $6.00: {spent}");

    // The refusal surface: the default caps, folded per goal exactly as `trigger::capped` folds
    // them. Nothing here is over a cap, because no cap is measured on the agent.
    let caps = Caps::default();
    let cap = caps.max_spend_usd.unwrap_or_default();
    let goal_count = goals.len();
    let refused = goals.iter().any(|goal| {
        let (spent, attempts) = swarm.spend_on(goal);
        caps.exceeded(attempts, spent.cost_usd).is_some()
    });
    assert!(
        !refused,
        "THIS ASSERTION IS THE BUG, PINNED, AND IT HAS JUST STOPPED BEING TRUE. An agent that \
         spent ${spent:.2} of a ${cap:.2} cap over {turns} turns, spread across {goal_count} \
         goals, was refused NOWHERE when this case was written — that absence is the unmet fourth \
         acceptance clause of story:the-turn-cap-did-not-hold, and the ceiling that fixes it \
         belongs in state.rs. Something now refuses, so the clause is met: rename this case to \
         an_agents_spend_is_bounded_across_the_goals_it_works and flip `!refused` to `refused`."
    );
}
