//! `story:a-turn-is-confined-by-a-frame`, the clauses that are the runtime's to answer.
//!
//! What metaharness does with a frame was measured, twice, for $0.224, and is not re-measured here:
//! a frame admitting only `file.read` was sealed, passed with `--decisions frame`, and the run's own
//! census reported `denied: 1`, `by_decider: {"frame": 1}`, `by_seam: {"hook": 1}` against a `Bash`
//! call that never ran. A frame with a wrong digest was refused with exit 2 before any session
//! started and nothing was billed.
//!
//! What is left, and what these cases drive, is this runtime's half of clause 2: the refusal has to
//! reach the turn's own record under `data/swarms/<slug>/turns/`, where a person can read which
//! operation and which rule. A decision the vendor makes and the runtime drops on the floor is a
//! confinement nobody can audit.
//!
//! The session here is a `Launch::Program` streaming the same `metaharness.event/1` records a real
//! run emits, so no money is spent and no vendor is required.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value as Json, json};

use ess_runtime::Spec;
use swarm_server::coordinator;
use swarm_server::swarm::Swarm;

/// The process environment is one object and these cases set it.
static ENVIRONMENT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

/// A session that reaches for a tool the frame does not admit and is refused at the seam.
///
/// The two lines are the shape metaharness emits, `tool.requested` then `tool.decided` — "three
/// events rather than two because a denial has no result and a decision is not a result"
/// (`metaharness-protocol::Event`). The `reason` is metaharness's own wording, which names the tool,
/// what it resolved to, and what the step admits instead.
fn a_refused_session(at: &std::path::Path) -> String {
    std::fs::write(
        at,
        "#!/bin/sh\ncat > /dev/null\n\
         printf '{\"event\":\"tool.requested\",\"call_id\":\"t1\",\"tool_name\":\"Bash\",\
\"operations\":[\"shell\"]}\\n'\n\
         printf '{\"event\":\"tool.decided\",\"call_id\":\"t1\",\"decided_by\":\"frame\",\
\"seam\":\"hook\",\"decision\":{\"decision\":\"deny\",\"reason\":\
\"this step admits [file.read] and Bash is [shell], which it does not; a refusal is the answer, \
not an obstacle\"}}\\n'\n\
         printf '{\"reached\": false, \"note\": \"refused, and I did not route around it\"}\\n'\n",
    )
    .expect("the session program is written");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(0o755))
        .expect("the session program is runnable");
    at.display().to_string()
}

/// A stand-in for `metaharness run claude`, on `PATH` under that name.
///
/// It records the argv it was given, one argument per line, and streams one `text` event carrying a
/// verdict — which is what the metaharness arm reads a verdict from. No vendor, no money.
fn a_stand_in_for_metaharness(bin: &std::path::Path, argv_at: &std::path::Path) {
    std::fs::create_dir_all(bin).expect("a directory for the stand-in");
    std::fs::write(
        bin.join("metaharness"),
        format!(
            "#!/bin/sh\n\
             : > '{argv}'\n\
             for arg in \"$@\"; do printf '%s\\n' \"$arg\" >> '{argv}'; done\n\
             printf '%s\\n' '{{\"event\":\"text\",\"text\":\"VERDICT \
             {{\\\"reached\\\": false, \\\"note\\\": \\\"it ran\\\"}}\"}}'\n",
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

/// A started swarm with one goal already in `Pursuing`, ready for a turn.
async fn a_swarm_mid_turn(data: &tempdir::TempDir, slug: &str) -> (Arc<Swarm>, String) {
    let spec = Arc::new(Spec::load(kernel()).expect("the kernel resolves"));
    let swarm = Arc::new(
        Swarm::open(spec, data.path(), slug)
            .await
            .expect("a swarm opens"),
    );
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
        "swarm.manager.CreateSwarm",
        json!({"display_name": slug, "tmux_session": slug, "home": "/nowhere",
               "created_at": "2026-09-13T10:00:00Z"}),
    )
    .await;
    let swarm_id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();
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
    (swarm, goal)
}

/// Clause 2: the refusal reaches the turn's event record, where a person can read which operation
/// and which rule.
///
/// The runtime writes every `metaharness.event/1` record it is handed, so the raw decision is in
/// the file — and a case that only asserted that would be asserting the absence of a filter. What
/// it also has to be is *findable*: a reader asking "was anything refused in this turn" should not
/// have to read a transcript to find out, which is what `Spent::refused` on the spend row answers.
#[tokio::test]
async fn a_refusal_reaches_the_turns_own_record_and_its_figures() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("swarm-frame-refusal").expect("a scratch directory");
    let program = a_refused_session(&data.path().join("session.sh"));
    // SAFETY: every case in this file that touches the environment holds `ENVIRONMENT` first.
    unsafe { std::env::set_var("SWARM_COORDINATOR", &program) };
    let (swarm, goal) = a_swarm_mid_turn(&data, "frame-refusal").await;

    let answer = coordinator::take_a_turn(&swarm, "coordinator", &goal, "reach something", 1)
        .await
        .expect("the turn is answered");
    assert!(!answer.verdict.reached);
    assert_eq!(
        answer.spent.refused, 1,
        "the turn's own figures say a call was refused"
    );

    let turns: Vec<PathBuf> = std::fs::read_dir(swarm.dir().join("turns"))
        .expect("the turns directory exists")
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|kind| kind == "jsonl"))
        .filter(|path| path.file_name().is_some_and(|name| name != "spend.jsonl"))
        .collect();
    assert_eq!(turns.len(), 1, "one turn, one transcript: {turns:?}");

    let transcript = std::fs::read_to_string(&turns[0]).expect("the transcript reads back");
    let decided: Vec<Json> = transcript
        .lines()
        .filter_map(|line| serde_json::from_str::<Json>(line).ok())
        .filter(|event| event["event"] == "tool.decided")
        .collect();
    assert_eq!(
        decided.len(),
        1,
        "the decision is in the turn's record, not only on the watch stream: {transcript}"
    );
    assert_eq!(decided[0]["decision"]["decision"], "deny");
    assert_eq!(
        decided[0]["decided_by"], "frame",
        "and the record says the frame decided it, not a timeout and not the adapter"
    );

    let reason = decided[0]["decision"]["reason"]
        .as_str()
        .expect("a refusal carries its reason");
    assert!(
        reason.contains("Bash") && reason.contains("shell"),
        "a person can read WHICH operation was refused: {reason}"
    );
    assert!(
        reason.contains("file.read"),
        "and WHICH rule refused it: {reason}"
    );

    // And the same fact where a reader looks for figures rather than for a transcript.
    let rows: Vec<Json> = std::fs::read_to_string(swarm.dir().join("turns").join("spend.jsonl"))
        .expect("the spend record exists")
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert_eq!(rows.len(), 1, "one attempt, one row");
    assert_eq!(
        rows[0]["spent"]["refused"], 1,
        "the refusal is in the row a cap is read from, too: {}",
        rows[0]
    );
    assert_eq!(rows[0]["agent"], "coordinator");
}

/// Clause 1, driven: the frame a metaharness turn ran under is sealed, named by the runtime's own
/// `frame_file`, and kept beside the transcript.
///
/// The first version built a frame itself and asserted the name it had just computed, which is a
/// case agreeing with itself: `frame_file` had no caller in the suite at all, so it could have been
/// pointed anywhere and every test stayed green.
///
/// The second version called `frame_file` and compared it to the `--frame` the runtime used, which
/// is the SAME tautology one step out: both sides move together. Measured — pointing `frame_file`
/// at `.somewhere-else.json` left it green.
///
/// So this states the property instead: the frame is in the transcript's own directory, under the
/// transcript's own name, with an extension that says what it is. That is what "kept beside its
/// transcript" means to the person who has to find it, and it is true or false independently of
/// what any function in the crate returns.
///
/// The metaharness arm is driven with a stand-in on `PATH`, so no vendor is required and nothing is
/// billed.
#[tokio::test]
async fn the_frame_a_turn_ran_under_is_named_by_the_runtime_and_kept_beside_its_transcript() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("swarm-frame-kept").expect("a scratch directory");
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
    let (swarm, goal) = a_swarm_mid_turn(&data, "frame-kept").await;

    coordinator::take_a_turn(&swarm, "coordinator", &goal, "reach something", 1)
        .await
        .expect("the turn is answered");

    let argv: Vec<String> = std::fs::read_to_string(&argv_at)
        .expect("the stand-in recorded the argv it was launched with")
        .lines()
        .map(ToOwned::to_owned)
        .collect();
    let used = argv
        .iter()
        .position(|part| part == "--frame")
        .and_then(|at| argv.get(at + 1))
        .map(PathBuf::from)
        .expect("a metaharness turn is launched with a frame");

    let transcript = std::fs::read_dir(swarm.dir().join("turns"))
        .expect("the turns directory exists")
        .filter_map(|entry| Some(entry.ok()?.path()))
        .find(|path| {
            path.extension().is_some_and(|kind| kind == "jsonl")
                && path.file_name().is_some_and(|name| name != "spend.jsonl")
        })
        .expect("the turn wrote a transcript");

    assert_eq!(
        used.parent(),
        transcript.parent(),
        "the frame is in the transcript's own directory: {used:?} beside {transcript:?}"
    );
    let stem = transcript
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".jsonl"))
        .expect("a transcript is a .jsonl");
    assert_eq!(
        used.file_name().and_then(|name| name.to_str()),
        Some(format!("{stem}.frame.json").as_str()),
        "under the transcript's own name, with an extension that says what it is — which is how a \
         reader asking `why was that call refused` finds the document that refused it"
    );
    assert!(used.is_file(), "and it is on disk where the turn read it");

    let document: Json =
        serde_json::from_str(&std::fs::read_to_string(&used).expect("the frame reads back"))
            .expect("it is JSON");
    assert_eq!(document["format"], "metaharness.frame/1");
    assert_eq!(
        swarm_server::frame::digest_of(&document),
        document["digest"].as_str().expect("a digest"),
        "sealed where it lies: a frame edited after the turn would not describe itself"
    );
    assert!(
        used.starts_with(swarm.dir().join("turns")),
        "under the swarm's own directory, beside the transcript: {used:?}"
    );
}
