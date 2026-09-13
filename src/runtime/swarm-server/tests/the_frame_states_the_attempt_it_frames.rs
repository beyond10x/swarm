//! The sealed frame against the turn it is sealed for, driven through the runtime's own
//! metaharness arm.
//!
//! Added by the adversary pass of the 2026-09-13a wave. Every other case in this crate reaches the
//! frame either by calling `frame::write` itself or by reading the argv `metaharness_argv` returns;
//! nothing drives `run_turn`'s `Launch::Metaharness` arm, because that arm spawns `metaharness` and
//! a real one costs money. It does not have to be a real one: the arm resolves `metaharness` on
//! `PATH`, so a stand-in on `PATH` exercises the whole wiring — seal the frame, name it to
//! `--frame`, keep it beside the transcript — for nothing.
//!
//! What that buys is the one figure in the document nobody else can see: `step.attempt`.
//! `turn_file` computes the attempt (its own doc records a real retry on 2026-09-12, where a first
//! attempt read its mail, ran out of vendor turns before writing a verdict, and was retried) and
//! `coordinator.rs` seals `attempt: 1` regardless. metaharness renders that field into the
//! instruction the model is shown — `Frame::render_instruction`: "step {index} attempt {attempt}"
//! — so on every retry the model is told it is on its first.
//!
//! No money is spent here: the stand-in is a shell script and no vendor is required.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value as Json, json};

use ess_runtime::Spec;
use swarm_server::coordinator;
use swarm_server::swarm::Swarm;

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

/// A stand-in for `metaharness run claude`, on `PATH` under that name.
///
/// It writes the argv it was given, one argument per line, where the case can read it, and streams
/// one `text` event carrying a verdict — which is what the metaharness arm reads a verdict from.
fn a_stand_in_for_metaharness(bin: &Path, argv_at: &Path) {
    std::fs::create_dir_all(bin).expect("a directory for the stand-in");
    std::fs::write(
        bin.join("metaharness"),
        format!(
            "#!/bin/sh\n\
             : > '{argv}'\n\
             for arg in \"$@\"; do printf '%s\\n' \"$arg\" >> '{argv}'; done\n\
             printf '%s\\n' '{{\"event\":\"text\",\"text\":\"VERDICT \
             {{\\\"reached\\\": false, \\\"note\\\": \\\"the retry ran\\\"}}\"}}'\n",
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

/// The frame a retry runs under says which attempt it is, because the runtime already knows.
///
/// `turn_file` picks the attempt by counting the transcripts already on disk for this turn, so the
/// runtime holds the number at the moment it seals the frame — and seals `attempt: 1` anyway. A
/// frame that misstates the step it frames is the one field of the document the sealer invents
/// rather than reports, and metaharness puts it in front of the model verbatim.
#[tokio::test]
async fn the_frame_a_retry_runs_under_states_the_attempt_it_is() {
    let data = tempdir::TempDir::new("swarm-frame-attempt").expect("a scratch directory");
    let bin = data.path().join("bin");
    let argv_at = data.path().join("argv.txt");
    a_stand_in_for_metaharness(&bin, &argv_at);
    // SAFETY: this case is the only one in this test binary, and a test binary is its own process.
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

    let (swarm, goal) = a_swarm_mid_turn(&data, "frame-attempt").await;

    // A first attempt that streamed and never wrote a verdict — the retry `turn_file`'s own doc
    // comment records happening on 2026-09-12 — leaves its transcript on disk, so the turn the
    // runtime is about to take is attempt 2.
    let first = coordinator::turn_file(&swarm, "coordinator", &goal, 1);
    std::fs::create_dir_all(first.parent().expect("a parent")).expect("the turns directory");
    std::fs::write(&first, "{\"event\":\"text\",\"text\":\"no verdict\"}\n")
        .expect("the first attempt left a transcript");

    coordinator::take_a_turn(&swarm, "coordinator", &goal, "reach something", 1)
        .await
        .expect("the turn is answered");

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
        frame["step"]["index"], 1,
        "the frame names the turn it is sealed for: {}",
        frame["step"]
    );
    assert!(
        frame_at.contains("0001-02-coordinator"),
        "and it is beside the transcript of the attempt it frames: {frame_at}"
    );
    assert_eq!(
        frame["step"]["attempt"], 2,
        "a retry's frame says it is a retry. metaharness renders this field into the instruction \
         the model is shown (`Frame::render_instruction`: \"step {{index}} attempt {{attempt}}\"), \
         so a frame that always says 1 tells every retry it is a first attempt: {}",
        frame["step"]
    );
}
