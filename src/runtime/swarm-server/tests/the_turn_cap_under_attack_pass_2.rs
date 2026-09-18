//! Adversarial cases against unit A of the 2026-09-12c wave, pass 2 (the correction, `3793fdd`).
//!
//! Pass 1 attacked the documents the unit wrote about itself. The correction answered five of six
//! and added a feature — `coordinator::resolve`, with a default that runs a real model. These are
//! the three places where the correction's own documents and the code it shipped disagree.
//!
//! * `budget.rs` says a cap is lifted by "`0`, `off` or `none` … and to nothing else", and that a
//!   value which is "merely wrong" keeps the default and says so. The correction's own case is
//!   titled `no_cap_is_lifted_by_a_value_that_is_merely_wrong` and says it measures "the class, not
//!   the instance". The class is not closed: `read::<f64>` compares against `T::default()` with
//!   `==` and `<`, and NaN is equal to nothing and less than nothing, so it is neither lifted nor
//!   refused — it is ACCEPTED as the ceiling, and no finite spend is ever `>=` it.
//!   `a_spend_cap_of_nan_is_not_a_cap` drives `Caps::configured` from the environment.
//! * `trigger.rs` has two cap guards. The one in `fire` reports the goal capped, announces it and
//!   logs it; the one added by the correction at `trigger.rs:137` does none of those. The shape the
//!   story is about — a coordinator that never writes a verdict — leaves the goal in `Pursuing`,
//!   which `swarm.goal.GoalsAwaitingATick` does not select (`goal.yaml:259-262`), so `fire` never
//!   runs again and the SILENT guard is the only one that ever trips.
//!   `a_goal_stopped_by_the_silent_guard_is_still_reported_capped` drives the real loop.
//! * `config.yaml` says "One config is Active per swarm at a time" and that `ActivateConfig`
//!   supersedes the previous one "in the same transaction" — and then records, in the same file,
//!   that a binding cannot do it. `from_config` resolved with `.find(state == "Active")`.
//!   `resolve_refuses_rather_than_coin_flipping_between_two_active_configs` asks which config a
//!   swarm runs with after its operator replaced one. The unit answered by refusing, and the case
//!   below now pins BOTH halves: that two rows really are left Active — the specification's claim
//!   is false and stays false — and that eight swarms in that state give the same deterministic
//!   answer instead of a coin flip.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value as Json, json};

use ess_runtime::Spec;
use swarm_server::budget::Caps;
use swarm_server::coordinator::{self, Launch};
use swarm_server::state::Server;
use swarm_server::swarm::Swarm;

/// The process environment is one object and these cases set it. Held for the whole of each.
static ENVIRONMENT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

/// The kernel specification, copied, with only the periodic binding's `every:` shortened.
///
/// `trigger::run` sleeps the binding's own period before its first tick, and the guard under
/// attack is only reached on the fourth of them — two real minutes for one case. `tokio`'s
/// `start_paused` is the usual answer and this crate does not enable `test-util`, so the cadence is
/// shortened in a COPY inside the case's own scratch directory. Nothing else in the specification is
/// touched, and the cadence is not what is being measured: the guard runs the same code at any
/// period. `every: PT30S` lives at `src/core/components.yaml:254`.
fn a_quicker_kernel(into: &std::path::Path) -> PathBuf {
    fn copy(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).expect("a directory");
        for entry in std::fs::read_dir(from).expect("the kernel is readable") {
            let entry = entry.expect("an entry");
            let (source, target) = (entry.path(), to.join(entry.file_name()));
            if source.is_dir() {
                copy(&source, &target);
            } else {
                std::fs::copy(&source, &target).expect("a file copies");
            }
        }
    }
    let root = into.join("kernel");
    copy(&kernel(), &root);
    let components = root.join("components.yaml");
    let text = std::fs::read_to_string(&components).expect("components.yaml is readable");
    assert!(text.contains("every: PT30S"), "the cadence is where it was");
    std::fs::write(&components, text.replace("every: PT30S", "every: PT1S"))
        .expect("the copy is writable");
    root
}

async fn issue(swarm: &Arc<Swarm>, command: &str, input: Json) {
    // A fresh idempotency key per call: these cases issue the same command more than once on
    // purpose, and the store rightly refuses a repeated key carrying a different request.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let key = format!(
        "{command}-{}",
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    swarm
        .issue(
            None,
            command,
            input.as_object().expect("an object").clone(),
            &key,
        )
        .await
        .expect("the command applies");
}

/// The claim: "`0`, `off` or `none` … and to nothing else" (`budget.rs`, module doc), and "a value
/// that does not parse, or that is below zero … keeps the default and says so"
/// (`Caps::configured`).
///
/// `nan` parses, so it is not the first branch. It is not below zero — NaN is below nothing — so it
/// is not the second. It is not equal to zero, so it is not a lift either. It falls through to
/// `Ok(value) => Some(value)` and becomes the ceiling, and `spent >= NaN` is false for every spend
/// there has ever been. The spend cap is gone, silently, with no warning: exactly the failure mode
/// `SWARM_MAX_SPEND_USD=-1` had before the correction, in a value the correction did not add to its
/// list. The turn cap is unaffected, because `u64` cannot parse it — the same asymmetry, again.
///
/// What reaches it: `SWARM_MAX_SPEND_USD`, which `story:the-turn-cap-did-not-hold` §Out of scope
/// names as the only way this cap is set. Any string an operator, a shell expansion or a wrapper
/// script puts there reaches `read`; this case asserts the classification the module claims to have,
/// not that somebody typed `nan`.
#[tokio::test]
async fn a_spend_cap_of_nan_is_not_a_cap() {
    let _guard = ENVIRONMENT.lock().await;
    for merely_wrong in ["nan", "NaN", "inf"] {
        // SAFETY: every case in this file holds `ENVIRONMENT` first.
        unsafe {
            std::env::set_var("SWARM_MAX_SPEND_USD", merely_wrong);
            std::env::remove_var("SWARM_MAX_TURNS");
        }
        let caps = Caps::configured();
        assert_eq!(
            caps.max_turns,
            Caps::default().max_turns,
            "SWARM_MAX_TURNS is untouched here"
        );
        // The contract is not "a value is present"; it is that the cap can still refuse. A ceiling
        // no spend can reach is a lifted cap wearing a number.
        assert!(
            caps.exceeded(0, Some(1_000_000.0)).is_some(),
            "SWARM_MAX_SPEND_USD={merely_wrong} is neither 0, off nor none, so it must not lift the \
             spend cap: a million dollars on one goal was refused by nothing, and max_spend_usd is \
             {:?}",
            caps.max_spend_usd
        );
    }
    // SAFETY: as above.
    unsafe { std::env::remove_var("SWARM_MAX_SPEND_USD") };
}

/// The claim: a reader can see why the loop stopped (`state.rs::CappedGoal`, `What::Capped`, and
/// `Reached`'s Display — "raise SWARM_MAX_TURNS or abandon it", which is written to be read by a
/// person). The unit's own case asserts it, for the goal that ANSWERS:
/// `a_goal_with_a_cap_of_three_stops_at_three_turns` ends on
/// `server.capped_goals().iter().any(...)`.
///
/// This is the other shape, and it is the shape the story is about: a coordinator that never writes
/// a verdict. Such a goal is left in `Pursuing`; `swarm.goal.GoalsAwaitingATick` selects only `Open`
/// and `Evaluating` (`src/core/domains/goal.yaml:259-262`), so `fire` — the guard that reports,
/// announces and warns — never sees the goal again after the first period. Every later refusal
/// comes from the guard the correction added at `trigger.rs:137`, which `continue`s in silence.
///
/// So the loop does stop, and nobody is told. The goal sits in `Pursuing` for ever with no entry in
/// `capped_goals()`, nothing on the watch stream and no warning in the log — which is
/// indistinguishable, to every reader the server has, from the $11.35 run that is the reason this
/// module exists.
///
/// What reaches it: `story:the-turn-cap-did-not-hold`'s own evidence (`dsfsdf`, goal `77fc1fcc`,
/// 44 attempts and one `iterations`), and the unit's own
/// `a_coordinator_that_never_answers_is_stopped_at_three_attempts`, which constructs this state and
/// then does not ask whether anybody was told.
#[tokio::test]
async fn a_goal_stopped_by_the_silent_guard_is_still_reported_capped() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("swarm-silent-cap").expect("a scratch directory");

    // A coordinator that runs, costs money and never answers. `exit 3` is what the unit's own
    // never-answers case uses.
    let program = data.path().join("coordinator.sh");
    std::fs::write(&program, "#!/bin/sh\ncat > /dev/null\nexit 3\n").expect("written");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("runnable");
    // SAFETY: every case in this file holds `ENVIRONMENT` first.
    unsafe {
        std::env::set_var("SWARM_MAX_TURNS", "3");
        std::env::set_var("SWARM_MAX_SPEND_USD", "off");
        std::env::set_var("SWARM_COORDINATOR", &program);
    }

    let spec = Spec::load(a_quicker_kernel(data.path())).expect("the kernel resolves");
    let server = Arc::new(
        Server::start(spec, data.path().to_path_buf())
            .await
            .expect("the server starts"),
    );
    let swarm = server.open("silent").await.expect("a swarm opens");
    issue(
        &swarm,
        "swarm.manager.CreateSwarm",
        json!({"display_name": "silent", "tmux_session": "silent",
               "home": data.path().display().to_string(),
               "created_at": "2026-09-12T10:00:00Z"}),
    )
    .await;
    let swarm_id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();
    issue(
        &swarm,
        "swarm.manager.StartSwarm",
        json!({"swarm_id": swarm_id, "started_at": "2026-09-12T10:01:00Z"}),
    )
    .await;
    issue(
        &swarm,
        "swarm.goal.SetGoal",
        json!({"swarm_id": swarm_id, "text": "2342342"}),
    )
    .await;
    let goal = swarm.instances("swarm.goal.Goal").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();

    // The real loop, on the specification's own cadence, with the clock paused so that thirty
    // seconds of it costs nothing. Nothing here re-implements a guard.
    let driven = tokio::spawn(swarm_server::trigger::run(Arc::clone(&server)));
    for _ in 0..1_500 {
        if swarm.spend_on(&goal).1 >= 3 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    // A few more periods, so that a guard which reports late still reports. The cap is checked at
    // the START of a turn, so the period in which the third attempt lands is not the period that
    // refuses — the refusal comes one period later. At `every: PT1S` the 500ms this first read does
    // not reach it, and the case could not observe the thing it asserts.
    tokio::time::sleep(Duration::from_millis(3_500)).await;
    driven.abort();

    let (_, attempts) = swarm.spend_on(&goal);
    assert_eq!(attempts, 3, "the cap of three held on attempts");
    assert!(
        server.capped_goals().iter().any(|c| c.goal == goal),
        "a goal the loop stopped asking about is reported capped, whether the guard that stopped \
         it was the one in `fire` or the one at trigger.rs:137. This goal took {attempts} of a cap \
         of 3 and then the loop went quiet: capped_goals() holds {:?}, so every reader the server \
         has — the status endpoint, the watch stream, the log — sees a goal in Pursuing and no \
         reason, which is the picture the $11.35 run presented",
        server.capped_goals()
    );
}

/// The claim: "One config is Active per swarm at a time", and `ActivateConfig` "supersedes the
/// previously active row in the same transaction" (`src/core/domains/config.yaml:9-12`, and the
/// command's own `summary`). `from_config` rested on it — it resolved with
/// `.find(|config| state == "Active")`, which is only "the config in force" if there is at most one.
///
/// The same file records, in the same paragraph, that a binding cannot supersede the previous row.
/// So the runtime keeps every activated config Active, and `.find` returned whichever the instance
/// map yielded first — which is keyed by a generated UUID, so WHICH COORDINATOR A SWARM LAUNCHED
/// WAS DECIDED BY A COIN FLIP. This case was first written asserting only `resolve`'s answer and
/// was red alone and green in the suite for that reason; the invariant is asserted first because it
/// is the deterministic half, and the eight swarms below are the visible consequence.
///
/// # Two halves, and what each one pins
///
/// **The specification's claim is false, and this pins it false.** `assert_eq!(active.len(), 2)`
/// is not what anyone wants to be true; it is what IS true, and pinning it means the day somebody
/// implements the supersede this case fails and sends them here. The runtime cannot fix it: no
/// binding can carry it (`ConfigActivated` does not name the previous row) and `activated_at` is
/// never written, so the log does not even record which activation came last. `src/core` is the
/// specification and is not this unit's to change; the correction is filed as a patch.
///
/// **The coin flip is gone, and this pins that.** `resolve` refuses, by name, on all eight swarms.
/// Refusing is the only answer available: picking needs an ordering, and the ordering the field was
/// declared for is empty on every row. A refusal an operator can read and act on beats launching
/// the config they replaced, half the time, silently.
///
/// # When the supersede lands
///
/// Change `2` to `1` in the first assertion, drop the `activated_at` assertion if a timestamp is
/// written, and change the last block to expect `Source::Config` with `the-new-one`.
///
/// What reaches it: `DraftConfig` + `ActivateConfig` twice, which is the documented way to change a
/// swarm's launch line — "a new draft is copied from it instead" (`config.yaml`, on the lifecycle).
#[tokio::test]
async fn resolve_refuses_rather_than_coin_flipping_between_two_active_configs() {
    let _guard = ENVIRONMENT.lock().await;
    // SAFETY: every case in this file holds `ENVIRONMENT` first.
    unsafe { std::env::remove_var("SWARM_COORDINATOR") };
    let data = tempdir::TempDir::new("swarm-two-configs").expect("a scratch directory");
    let spec = Arc::new(Spec::load(kernel()).expect("the kernel resolves"));

    let mut answers = Vec::new();
    for round in 0..8 {
        let slug = format!("replaced-{round}");
        let swarm = Arc::new(
            Swarm::open(Arc::clone(&spec), data.path(), &slug)
                .await
                .expect("a swarm opens"),
        );
        issue(
            &swarm,
            "swarm.manager.CreateSwarm",
            json!({"display_name": slug, "tmux_session": slug,
                   "home": data.path().display().to_string(),
                   "created_at": "2026-09-12T10:00:00Z"}),
        )
        .await;
        let swarm_id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
            .as_str()
            .expect("an identity")
            .to_owned();

        // Two configs, drafted and activated in order. The second is what the operator asked for.
        for binary in ["/usr/local/bin/the-old-one", "/usr/local/bin/the-new-one"] {
            issue(
                &swarm,
                "swarm.config.DraftConfig",
                json!({"swarm_id": swarm_id, "paths": {},
                       "launch": {"harness": "Claude", "binary": binary, "model": "", "args": [],
                                  "skip_permissions": false, "tool_surface": "Owned",
                                  "allow_program": []},
                       "schedules": {}, "budgets": {}, "board": {}}),
            )
            .await;
            let drafted = swarm
                .instances("swarm.config.Config")
                .await
                .into_iter()
                .find(|config| config["state"].as_str() == Some("Draft"))
                .expect("the draft exists");
            let config_id = drafted["id"].as_str().expect("an identity").to_owned();
            issue(
                &swarm,
                "swarm.config.ActivateConfig",
                json!({"config_id": config_id, "swarm_id": swarm_id}),
            )
            .await;
        }

        let active: Vec<Json> = swarm
            .instances("swarm.config.Config")
            .await
            .into_iter()
            .filter(|config| config["state"].as_str() == Some("Active"))
            .map(|config| config["fields"].clone())
            .collect();
        assert_eq!(
            active.len(),
            2,
            "PINNED AT THE WRONG ANSWER. config.yaml:9 says one config is Active per swarm at a \
             time and ActivateConfig's summary says the previous row is superseded in the same \
             transaction. Both are false and this asserts them false: activating a replacement \
             left {} row(s) Active. It has just changed, so somebody implemented the supersede — \
             change this 2 to a 1 and read the rest of this case's doc comment. Rows: {active:?}",
            active.len()
        );
        assert!(
            active.iter().all(|fields| fields["activated_at"].is_null()),
            "and nothing records which of them was activated last: the `activated` outcome writes \
             no fields and ConfigActivated carries no timestamp, so activated_at is null on both. \
             That is why resolve refuses instead of ordering them: {active:?}"
        );

        let active_ids: Vec<String> = swarm
            .instances("swarm.config.Config")
            .await
            .into_iter()
            .filter(|config| config["state"].as_str() == Some("Active"))
            .map(|config| config["id"].as_str().unwrap_or_default().to_owned())
            .collect();

        // The coin flip, eight times. Whatever resolve does here, it must do the same thing every
        // time — that is the half this shape exists to measure.
        let answer = match coordinator::resolve(&swarm).await {
            Err(why) => {
                for id in &active_ids {
                    assert!(
                        why.contains(id.as_str()),
                        "a refusal an operator can act on names the rows to supersede: {why}"
                    );
                }
                // The generated ids differ per swarm and are asserted above; what must be identical
                // across all eight is the answer's SHAPE, so they are cut out of the comparison.
                format!("refused: {}", why.split(" (").next().unwrap_or(&why))
            }
            Ok(resolved) => format!(
                "{} from {}",
                match &resolved.launch {
                    Launch::Program(argv) => argv.join(" "),
                    Launch::Metaharness { .. } => "metaharness".to_owned(),
                },
                resolved.source
            ),
        };
        answers.push(answer);
    }

    assert_eq!(
        answers
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        1,
        "eight swarms, the same two activations in the same order, and resolve() must answer the \
         same thing every time. It answered: {answers:?}"
    );
    assert!(
        answers[0].starts_with("refused: 2 configs are Active at once"),
        "and the one answer is a refusal that names what is wrong and how to fix it, not a config \
         the operator replaced: {}",
        answers[0]
    );
    // Nothing silently ran: the refusal is the whole answer, and Source never reached Config.
    assert!(
        !answers[0].contains("the-old-one"),
        "a replaced config is never what a swarm launches: {}",
        answers[0]
    );
}

/// The correction's off-by-one fix, at the cap values its own case does not use.
///
/// `trigger.rs:137` became `iterations.saturating_sub(1)` because two guards over one bound,
/// disagreeing by one, stopped a cap of three at two. `a_goal_with_a_cap_of_three_stops_at_three_turns`
/// pins three. A subtraction is the classic place for a fence-post to move, so this asks the same
/// question at the boundary (1), just above it (2) and at the default's order of magnitude (5): a
/// cap of N stops at exactly N, never N-1 and never N+1.
///
/// What reaches it: `SWARM_MAX_TURNS`, any value. `1` is the interesting one — it is the only value
/// at which `saturating_sub` saturates on the very first turn.
#[tokio::test]
async fn a_cap_of_n_stops_at_exactly_n() {
    let _guard = ENVIRONMENT.lock().await;
    for cap in [1_u64, 2, 5] {
        let data = tempdir::TempDir::new("swarm-cap-n").expect("a scratch directory");
        let program = data.path().join("coordinator.sh");
        std::fs::write(
            &program,
            "#!/bin/sh\ncat > /dev/null\nprintf '{\"reached\": false, \"note\": \"not yet\"}\\n'\n",
        )
        .expect("written");
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))
            .expect("runnable");
        // SAFETY: every case in this file holds `ENVIRONMENT` first.
        unsafe {
            std::env::set_var("SWARM_MAX_TURNS", cap.to_string());
            std::env::set_var("SWARM_MAX_SPEND_USD", "off");
            std::env::set_var("SWARM_COORDINATOR", &program);
        }

        let spec = Spec::load(a_quicker_kernel(data.path())).expect("the kernel resolves");
        let server = Arc::new(
            Server::start(spec, data.path().to_path_buf())
                .await
                .expect("the server starts"),
        );
        let slug = format!("cap-{cap}");
        let swarm = server.open(&slug).await.expect("a swarm opens");
        issue(
            &swarm,
            "swarm.manager.CreateSwarm",
            json!({"display_name": slug, "tmux_session": slug,
                   "home": data.path().display().to_string(),
                   "created_at": "2026-09-12T10:00:00Z"}),
        )
        .await;
        let swarm_id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
            .as_str()
            .expect("an identity")
            .to_owned();
        issue(
            &swarm,
            "swarm.manager.StartSwarm",
            json!({"swarm_id": swarm_id, "started_at": "2026-09-12T10:01:00Z"}),
        )
        .await;
        issue(
            &swarm,
            "swarm.goal.SetGoal",
            json!({"swarm_id": swarm_id, "text": "2342342"}),
        )
        .await;
        let goal = swarm.instances("swarm.goal.Goal").await[0]["id"]
            .as_str()
            .expect("an identity")
            .to_owned();

        let driven = tokio::spawn(swarm_server::trigger::run(Arc::clone(&server)));
        // Well past the cap in periods, so a bound that does not hold is visible as N+k.
        tokio::time::sleep(Duration::from_millis(1_000 * (cap + 5))).await;
        driven.abort();

        let (_, attempts) = swarm.spend_on(&goal);
        assert_eq!(
            attempts, cap,
            "SWARM_MAX_TURNS={cap} must stop at exactly {cap} turns, not {attempts}"
        );
    }
}
