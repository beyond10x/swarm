//! The second adversary pass of the 2026-09-13a wave, against what correction round 1 changed.
//!
//! Everything the unit changed this round arrived with cases the unit wrote for it, so every one of
//! those cases asserts the behaviour it built. What nobody ran is the unit's own prose read as the
//! specification it claims to be: `config.yaml`'s "the worst a config can do to it is admit less",
//! `frame::scope`'s six numbered rules read through metaharness's own matcher rather than through a
//! JSON shape assertion, `bounded`'s "the goal's is not one — it counts a different subject", and
//! `knobs_for`'s "no fallback from one to the other".
//!
//! No money is spent here: the one case that launches anything launches a shell script on `PATH`
//! under the name `metaharness`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value as Json, json};

use ess_runtime::Spec;
use swarm_server::budget::{Bound, Caps};
use swarm_server::coordinator::{self, Launch, Spent};
use swarm_server::frame;
use swarm_server::swarm::Swarm;
use swarm_server::trigger;

/// The process environment is one object and two cases below set it.
static ENVIRONMENT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

/// A created, started swarm and its own record id.
async fn a_swarm(data: &tempdir::TempDir, slug: &str) -> (Arc<Swarm>, String) {
    let spec = Arc::new(Spec::load(kernel()).expect("the kernel resolves"));
    let swarm = Arc::new(
        Swarm::open(spec, data.path(), slug)
            .await
            .expect("a swarm opens"),
    );
    swarm
        .issue(
            None,
            "swarm.manager.CreateSwarm",
            json!({"display_name": slug, "tmux_session": slug, "home": "/nowhere",
                   "created_at": "2026-09-13T10:00:00Z"})
            .as_object()
            .expect("an object")
            .clone(),
            "create",
        )
        .await
        .expect("the swarm is created");
    let id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();
    (swarm, id)
}

// ---------------------------------------------------------------------------------------------
// metaharness's own subject matcher, ported verbatim so the scope is judged by the thing that
// judges it in production rather than by a JSON shape this repository also wrote.
// ---------------------------------------------------------------------------------------------

/// `metaharness_protocol::frame::glob_matches`, 0.7.0, `crates/metaharness-protocol/src/frame.rs`.
///
/// Copied rather than depended on: `metaharness-protocol` is not a dependency of this crate and
/// adding one would change `Cargo.toml` and `Cargo.lock`, which are not an adversary's files. Its
/// own doc: "`*` matches within a path segment, `**` across them, and everything else is literal."
fn glob_matches(pattern: &str, value: &str) -> bool {
    fn go(pattern: &[u8], value: &[u8]) -> bool {
        match pattern.first() {
            None => value.is_empty(),
            Some(b'*') => {
                let crosses = pattern.get(1) == Some(&b'*');
                let rest = if crosses {
                    &pattern[2..]
                } else {
                    &pattern[1..]
                };
                for taken in 0..=value.len() {
                    if !crosses && value[..taken].contains(&b'/') {
                        break;
                    }
                    if go(rest, &value[taken..]) {
                        return true;
                    }
                }
                false
            }
            Some(&expected) => value.first() == Some(&expected) && go(&pattern[1..], &value[1..]),
        }
    }
    go(pattern.as_bytes(), value.as_bytes())
}

/// `metaharness_protocol::frame::SubjectScope::verdict`, same file, same version.
///
/// Its own doc: "A call carrying several subjects is judged by the **first rule any of them
/// matches**, and a call whose record names none is `Silent`."
fn verdict(rules: &Json, operation: &str, subjects: &[&str]) -> &'static str {
    let rules = rules["rules"].as_array().expect("a scope has rules");
    if rules.is_empty() || subjects.is_empty() {
        return "Silent";
    }
    for rule in rules {
        let patterns = rule["subjects"].as_array().expect("a rule has subjects");
        let matched = subjects.iter().any(|subject| {
            patterns
                .iter()
                .any(|pattern| glob_matches(pattern.as_str().unwrap_or_default(), subject))
        });
        if matched {
            let admits = rule["operations"]
                .as_array()
                .expect("a rule has operations")
                .iter()
                .any(|op| op["op"].as_str() == Some(operation));
            return if admits { "Admitted" } else { "Refused" };
        }
    }
    "Silent"
}

/// The scope one member of a two-member swarm runs under.
fn a_members_scope() -> Json {
    let operations: Vec<String> = frame::ADMITTED.iter().map(|op| (*op).to_owned()).collect();
    frame::scope(
        Path::new("/repo/data/swarms/s/work/builder"),
        Path::new("/repo/data/swarms/s/work"),
        &operations,
    )
}

// ---------------------------------------------------------------------------------------------

/// `frame::scope`'s six numbered rules, each read back through metaharness's own matcher.
///
/// This case is expected GREEN. It is here because the case that follows is not, and a red case
/// built on a matcher nobody checked is a typo dressed as a finding: this is the evidence that the
/// port above answers the way metaharness does on every claim the module's own doc comment makes.
#[test]
fn every_single_subject_claim_the_scope_doc_makes_holds_under_metaharnesss_matcher() {
    let rules = a_members_scope();
    let own = "/repo/data/swarms/s/work/builder";
    let shared = "/repo/data/swarms/s/work";

    let expected: Vec<(&str, &str, Vec<String>, &str)> = vec![
        (
            "rule 2: its own directory, in full",
            "file.write",
            vec![format!("file:{own}/NOTES.md")],
            "Admitted",
        ),
        (
            "rule 3: the shared one, read",
            "file.read",
            vec![format!("file:{shared}/coordinator/NOTES.md")],
            "Admitted",
        ),
        (
            "rule 3: the shared one, not written",
            "file.write",
            vec![format!("file:{shared}/coordinator/NOTES.md")],
            "Refused",
        ),
        (
            "rule 3: a peer's directory, not written",
            "file.edit",
            vec![format!("file:{shared}/reviewer/NOTES.md")],
            "Refused",
        ),
        (
            "rule 3: the shared root itself, not written",
            "file.write",
            vec![format!("file:{shared}/loose.md")],
            "Refused",
        ),
        (
            "rule 1: a climb out through the own directory",
            "file.write",
            vec![format!("file:{own}/../reviewer/NOTES.md")],
            "Refused",
        ),
        (
            "rule 1: a relative climb",
            "file.write",
            vec!["file:../reviewer/NOTES.md".to_owned()],
            "Refused",
        ),
        (
            "rule 4: any other absolute path, written",
            "file.write",
            vec!["file:/etc/passwd".to_owned()],
            "Refused",
        ),
        (
            "rule 4: any other absolute path, read",
            "file.read",
            vec!["file:/etc/passwd".to_owned()],
            "Refused",
        ),
        (
            "rule 4: the swarm's own turns directory",
            "file.read",
            vec!["file:/repo/data/swarms/s/turns/0001-01-builder-abcd.jsonl".to_owned()],
            "Refused",
        ),
        (
            "rule 5: a relative path that does not climb",
            "file.write",
            vec!["file:NOTES.md".to_owned()],
            "Admitted",
        ),
        (
            "rule 6: a subject that is not a file",
            "shell",
            vec!["proc:bash".to_owned()],
            "Refused",
        ),
        (
            "a call that names no subject at all",
            "shell",
            vec![],
            "Silent",
        ),
    ];

    let mut wrong: Vec<String> = Vec::new();
    for (what, operation, subjects, want) in expected {
        let borrowed: Vec<&str> = subjects.iter().map(String::as_str).collect();
        let got = verdict(&rules, operation, &borrowed);
        if got != want {
            wrong.push(format!(
                "{what}: {operation} on {subjects:?} -> {got}, wanted {want}"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "the scope does not do what `frame::scope`'s own doc says it does:\n  {}",
        wrong.join("\n  ")
    );
}

/// Rule 4 is "every other absolute path is refused. This is the rule that makes the scope a
/// confinement rather than a list of verbs" — and one admitted subject beside it turns it off.
///
/// # PINNED TO TODAY'S STATE, AND INVERTED ON PURPOSE
///
/// **This case asserted `Refused` and was correct to. It now asserts `Admitted`, which is the
/// defect, on the coordinator's explicit instruction of 2026-09-13, recorded here because a case
/// that quietly asserts a defect is indistinguishable from a case that never noticed one.**
///
/// The finding stands and is unfixed. What changed is only what the case is for: it no longer
/// claims the scope refuses these calls, it pins that the scope ADMITS them, so it goes red the day
/// that changes in either direction. `#[ignore]` was refused — a case that pre-excuses its own red
/// is one nobody reads again — and leaving it red was refused too, because a red case takes the
/// suite's exit status for everything after it and would blind the gate rather than document one
/// defect. The 2026-09-12c and 2026-09-12d waves both pinned a known gap this way.
///
/// **Invert it back to `Refused`** the day either of these lands, and delete this section:
///
/// * `SubjectScope::verdict` stops judging a call by the first rule ANY of its subjects matches; or
/// * `frame::scope` gains the swarm's roster and emits a read-only rule per PEER ahead of the
///   agent's own rule — which closes the second pair below, though not the first.
///
/// # Why it is unsatisfiable today, measured rather than argued
///
/// The pair is decided by `min(i, j)` — `i` the first rule matching the agent's own path, `j` the
/// first matching the outside one. Refusing it needs `j < i`: a refusing rule matching
/// `/etc/passwd` and NOT any own-directory path, placed before the admitting one. The matcher has
/// `*`, `**` and literals — no negation, no character classes, by its own doc — so such a rule must
/// discriminate on a literal prefix, for every path outside the work tree rather than the one named
/// here. That complement IS a finite union of literal-prefix globs (agree on the first *i* bytes,
/// differ at *i*, for each *i* and each other byte), and for a real work directory it is **14,848
/// patterns**, regenerated per swarm and sealed into every frame — against a type whose stated
/// purpose is "a boundary somebody has to be able to read at a glance". Established by exhaustion
/// over the pattern space, not by argument.
///
/// # The finding itself, unchanged
///
/// metaharness judges a call by **the first rule ANY of its subjects matches**
/// (`SubjectScope::verdict`, quoted above the port). Rule 2 matches the agent's own file, so the
/// whole call is Admitted and the operation is admitted **on every other subject the call carries**,
/// including an absolute path rule 4 exists to refuse. The scope's ordering is written as though a
/// call had one subject; metaharness's subject list is a `Vec`.
///
/// WHAT REACHES IT: nothing found. `metaharness_tools::subjects_of_vendor_call` builds the list
/// from three argument names — `file_path`, `notebook_path`, `path` — so a two-subject call needs a
/// vendor tool input carrying two of them, and this pass did not establish that Claude Code emits
/// or forwards one. The defect is therefore in the document before it is in the deployment: rule 4
/// is stated without the condition it actually holds under, and `frame::scope`'s doc now states it.
#[test]
fn an_admitted_subject_beside_an_outside_one_does_not_admit_the_outside_one() {
    let rules = a_members_scope();
    let own = "/repo/data/swarms/s/work/builder";
    let shared = "/repo/data/swarms/s/work";

    assert_eq!(
        verdict(&rules, "file.write", &["file:/etc/passwd"]),
        "Refused",
        "the control, and it still holds: alone, rule 4 refuses it"
    );
    assert_eq!(
        verdict(
            &rules,
            "file.write",
            &[&format!("file:{own}/a")[..], "file:/etc/passwd"],
        ),
        "Admitted",
        "PINNED DEFECT, not a claim that this is right: beside a subject rule 2 admits, rule 4 \
         does NOT refuse `/etc/passwd` — metaharness judges a call by the first rule ANY of its \
         subjects matches, so rule 2 governs the whole call. `frame::scope`'s rule 4 says it makes \
         the scope \"a confinement rather than a list of verbs\", and that holds only for a call \
         naming no admitted path beside the outside one. If this now reads `Refused`, the gap has \
         closed: invert this assertion back and delete the pin."
    );
    assert_eq!(
        verdict(
            &rules,
            "file.write",
            &[
                &format!("file:{own}/a")[..],
                &format!("file:{shared}/reviewer/NOTES.md")[..],
            ],
        ),
        "Admitted",
        "PINNED DEFECT, same rule: a peer's NOTES.md IS writable when the call also names the \
         member's own file, though rule 3 refuses it alone. This is the half a per-peer scope rule \
         would close without closing the one above — if this now reads `Refused`, that landed, and \
         this assertion inverts back."
    );
}

/// `config.yaml` says the operation set is safe to put in a config because "the worst a config can
/// do to it is admit less". A config admits MORE.
///
/// The unit's own argument for keeping the SUBJECT scope out of the config is that "a coordinator
/// reaches every command through `swarm do`, `swarm.config.DraftConfig` and `ActivateConfig` among
/// them, so a scope stated here is a scope the confined party can rewrite for itself. A confinement
/// its subject can widen confines nothing." Every word of that applies to `admitted_operations`,
/// which is in the config: `from_config` takes the list verbatim and `frame::document` checks it
/// against `VOCABULARY` (ten operations) rather than against `ADMITTED` (seven). So a coordinator
/// drafts and activates a config naming all ten and seals itself a frame admitting `web.read`,
/// `subagent.spawn` and `task.todo` — the three `frame::ADMITTED`'s own doc says are deliberately
/// not there ("the 2026-09-13 probe measured `available_operations` at seven *including* `web.read`
/// and `subagent.spawn` under `--decisions observe`, and neither is here").
///
/// WHAT REACHES IT: `prompt_for`, which tells every coordinator "`swarm do` reaches every command,
/// so you can spawn agents, draw boxes on the canvas and wire connections between them", and the
/// frame admits `shell`, which is how `swarm do` is called.
#[tokio::test]
async fn an_activated_config_cannot_admit_more_than_the_default_frame_does() {
    let data = tempdir::TempDir::new("swarm-widened-frame").expect("a scratch directory");
    let (swarm, swarm_id) = a_swarm(&data, "widened-frame").await;

    let everything: Vec<String> = frame::VOCABULARY
        .iter()
        .map(|op| (*op).to_owned())
        .collect();
    swarm
        .issue(
            None,
            "swarm.config.DraftConfig",
            json!({"swarm_id": swarm_id, "paths": {},
                   "launch": {"harness": "Claude", "binary": "", "model": "", "args": [],
                              "skip_permissions": false, "tool_surface": "Owned",
                              "allow_program": [],
                              "admitted_operations": everything},
                   "schedules": {}, "budgets": {}, "board": {}})
            .as_object()
            .expect("an object")
            .clone(),
            "draft",
        )
        .await
        .expect("the config is drafted");
    let config_id = swarm
        .instances("swarm.config.Config")
        .await
        .into_iter()
        .find(|config| config["state"].as_str() == Some("Draft"))
        .expect("the draft exists")["id"]
        .as_str()
        .expect("an identity")
        .to_owned();
    swarm
        .issue(
            None,
            "swarm.config.ActivateConfig",
            json!({"config_id": config_id, "swarm_id": swarm_id})
                .as_object()
                .expect("an object")
                .clone(),
            "activate",
        )
        .await
        .expect("the config is activated");

    let resolved = coordinator::resolve(&swarm)
        .await
        .expect("one Active config resolves");
    let admitted = match resolved.launch {
        Launch::Metaharness { admitted } => admitted,
        Launch::Program(argv) => {
            panic!("the config names no binary, so this is the frame arm: {argv:?}")
        }
    };

    let widened: Vec<&String> = admitted
        .iter()
        .filter(|operation| !frame::ADMITTED.contains(&operation.as_str()))
        .collect();
    assert!(
        widened.is_empty(),
        "a config the coordinator can draft and activate for itself widened its own frame by \
         {widened:?}. `config.yaml` says of this field that \"Narrowing is different and is safe, \
         which is why the operation set IS here — the worst a config can do to it is admit less\", \
         and `from_config` does not intersect with `frame::ADMITTED`; only `frame::document` \
         checks, and it checks against the ten-name VOCABULARY. The whole resolved set: {admitted:?}"
    );
}

/// `bounded`'s unit bound is fed the AGENT's figure by the member arm, so it fires on a unit that
/// has taken no turns and is reported as that unit's own cap.
///
/// `bounded`'s own doc draws the distinction this breaks: "The agent bound below reads the agent's
/// recorded attempts alone, and has no equivalent defence: there is no `iterations` figure kept per
/// agent to compare it against, and the goal's is not one — it counts a different subject."
/// `work_the_assignments` passes `swarm.spend_by_agent(agent).1` as `iterations`, and the unit bound
/// is `iterations.max(spend_on(unit).1)` — so a member retasked to a second assignment
/// ("Retasking a member is a new post, not an edit", `agent.yaml`) is refused at its first turn at
/// that assignment under `Bound::Goal`, and `record_capped` writes `bound: "unit"` for a unit with
/// nothing on its record. That is the shape `Capped`'s own doc records as the 2026-09-12 defect:
/// figures published against a unit that "used none of" the cap.
///
/// WHAT REACHES IT: `trigger::work_the_assignments` (`trigger.rs:246-254`), on every period, for
/// every member with an open assignment and any earlier work on its record.
#[tokio::test]
async fn a_members_first_turn_at_a_new_assignment_is_not_capped_as_that_assignments_own() {
    let data = tempdir::TempDir::new("swarm-unit-bound").expect("a scratch directory");
    let (swarm, _) = a_swarm(&data, "unit-bound").await;

    // The member worked its first assignment three times and finished it.
    for turn in 1..=3 {
        swarm.record_spend(
            "builder",
            "assignment-the-first",
            turn,
            &Spent::default(),
            Some(turn == 3),
            None,
        );
    }
    let (_, agent_wide) = swarm.spend_by_agent("builder");
    assert_eq!(agent_wide, 3, "three turns on the member's whole record");
    assert_eq!(
        swarm.spend_on("assignment-the-second").1,
        0,
        "and none at all on the assignment it has just been retasked to"
    );

    // Exactly what `work_the_assignments` passes: the agent's whole record, as `iterations`.
    let bound = trigger::bounded(
        Caps {
            max_turns: Some(3),
            max_spend_usd: None,
            ..Caps::default()
        },
        &swarm,
        "builder",
        "assignment-the-second",
        agent_wide,
    )
    .expect("three turns against a cap of three is refused, one way or the other");

    assert!(
        matches!(bound.bound, Bound::Agent(_)),
        "the refusal is right and the account of it is wrong: it fired as {:?} with turns={} \
         against an assignment whose own record holds 0 turns, so `record_capped` writes \
         bound=\"unit\" and a reader of turns/capped.jsonl is told to raise a cap this assignment \
         has used none of. The agent bound is the one that actually holds here.",
        bound.bound,
        bound.turns
    );
}

/// A member's launch is reported to the operator with the COORDINATOR's model on it.
///
/// `knobs_for` says "`SWARM_COORDINATOR_*` for the coordinator and `SWARM_MEMBER_*` for a member,
/// and **no fallback from one to the other**", and the argv honours that. `Launch::describe` does
/// not: it reads `SWARM_COORDINATOR_MODEL` unconditionally (`coordinator.rs:259`), and `run_turn`
/// logs `launch = %resolved.launch.describe()` for EVERY turn, a member's included
/// (`coordinator.rs:954`). The line `describe` produces is also what `/status` publishes as
/// `coordinator.program` (`state.rs:787`).
///
/// So an operator who sets `SWARM_COORDINATOR_MODEL` — which is what that knob is for — is told
/// each member runs `--model <that model>`, while `metaharness_argv` passes it no `--model` at all
/// and the member runs on the vendor default. The knob does not leak into the launch; it leaks into
/// the account of the launch, which is the only thing the operator ever sees.
///
/// WHAT REACHES IT: `run_turn`'s own tracing line, once per member turn, unconditionally.
#[tokio::test]
async fn a_members_launch_is_not_reported_with_the_coordinators_model() {
    let _guard = ENVIRONMENT.lock().await;
    // SAFETY: every case in this file that touches the environment holds `ENVIRONMENT` first.
    unsafe {
        std::env::set_var(
            "SWARM_COORDINATOR_MODEL",
            "a-model-only-the-coordinator-gets",
        );
        std::env::remove_var("SWARM_MEMBER_MODEL");
    }

    let described = Launch::metaharness().describe();

    // SAFETY: as above.
    unsafe {
        std::env::remove_var("SWARM_COORDINATOR_MODEL");
    }

    assert!(
        !described.contains("a-model-only-the-coordinator-gets"),
        "`Launch::describe` is the sentence `run_turn` logs for a member's turn and the one \
         `/status` publishes, and it names the coordinator's model on a launch that will not use \
         it: {described:?}. `knobs_for` is the function that knows which prefix a turn reads and \
         `describe` takes no turn, so it cannot ask."
    );
}

/// A stand-in for `metaharness run claude`, on `PATH` under that name.
fn a_stand_in_for_metaharness(bin: &Path, argv_at: &Path) {
    std::fs::create_dir_all(bin).expect("a directory for the stand-in");
    std::fs::write(
        bin.join("metaharness"),
        format!(
            "#!/bin/sh\n\
             : > '{argv}'\n\
             for arg in \"$@\"; do printf '%s\\n' \"$arg\" >> '{argv}'; done\n\
             printf '%s\\n' '{{\"event\":\"text\",\"text\":\"VERDICT \
             {{\\\"reached\\\": false, \\\"note\\\": \\\"the third attempt ran\\\"}}\"}}'\n",
            argv = argv_at.display()
        ),
    )
    .expect("the stand-in is written");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(
        bin.join("metaharness"),
        std::fs::Permissions::from_mode(0o755),
    )
    .expect("the stand-in is runnable");
}

/// The attempt the frame states on a THIRD attempt, which the second-attempt case cannot see.
///
/// Expected GREEN. `turn_file_attempt` counts transcripts already on disk from 1, so a case that
/// only ever checks 2 cannot tell "the attempt" from "1 plus whether anything is there"; this
/// checks that the counter is a counter. Asked for by the brief of this pass.
#[tokio::test]
async fn the_frame_of_a_third_attempt_says_three() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("swarm-third-attempt").expect("a scratch directory");
    let bin = data.path().join("bin");
    let argv_at = data.path().join("argv.txt");
    a_stand_in_for_metaharness(&bin, &argv_at);
    // SAFETY: every case in this file that touches the environment holds `ENVIRONMENT` first.
    unsafe {
        std::env::set_var(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        );
        std::env::set_var("SWARM_COORDINATOR", "metaharness");
    }

    let (swarm, swarm_id) = a_swarm(&data, "third-attempt").await;
    let issue = async |actor: Option<&str>, command: &str, input: Json| {
        swarm
            .issue(
                actor,
                command,
                input.as_object().expect("an object").clone(),
                command,
            )
            .await
            .expect("the command applies")
    };
    issue(
        None,
        "swarm.manager.StartSwarm",
        json!({"swarm_id": swarm_id, "started_at": "2026-09-13T10:01:00Z"}),
    )
    .await;
    issue(
        None,
        "swarm.goal.SetGoal",
        json!({"swarm_id": swarm_id, "text": "reach something"}),
    )
    .await;
    let goal = swarm.instances("swarm.goal.Goal").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();
    issue(
        Some("swarm.goal.Loop"),
        "swarm.goal.Pursue",
        json!({"goal_id": goal, "iterations": 1}),
    )
    .await;

    // Two attempts already on disk, neither of which wrote a verdict.
    for _ in 0..2 {
        let at = coordinator::turn_file(&swarm, "coordinator", &goal, 1);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("the turns directory");
        std::fs::write(&at, "{\"event\":\"text\",\"text\":\"no verdict\"}\n")
            .expect("an earlier attempt left a transcript");
    }

    coordinator::take_a_turn(&swarm, "coordinator", &goal, "reach something", 1)
        .await
        .expect("the turn is answered");

    // SAFETY: as above.
    unsafe {
        std::env::remove_var("SWARM_COORDINATOR");
    }

    let argv: Vec<String> = std::fs::read_to_string(&argv_at)
        .expect("the stand-in recorded the argv it was launched with")
        .lines()
        .map(ToOwned::to_owned)
        .collect();
    let frame_at = argv
        .iter()
        .position(|part| part == "--frame")
        .and_then(|at| argv.get(at + 1))
        .expect("a metaharness turn is launched with a frame");
    let frame: Json = serde_json::from_str(
        &std::fs::read_to_string(frame_at).expect("the frame the turn ran under reads back"),
    )
    .expect("the frame is JSON");

    assert_eq!(
        frame["step"]["attempt"], 3,
        "two transcripts are already on disk for this turn, so this is the third attempt and \
         metaharness renders the number into the instruction the model is shown: {}",
        frame["step"]
    );
    assert!(
        frame_at.contains("0001-03-coordinator"),
        "and the frame is kept beside the transcript of the attempt it frames: {frame_at}"
    );
}
