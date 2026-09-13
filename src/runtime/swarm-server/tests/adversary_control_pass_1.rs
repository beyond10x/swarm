//! Pause/stop through HTTP govern dispatch, process ownership, evidence and continuation.
//! Fake subprocesses use socket barriers; timeouts are failure bounds, never coordination.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value as Json, json};

use ess_runtime::Spec;
use swarm_server::state::Server;
use swarm_server::swarm::Swarm;
use swarm_server::trigger;

/// The process environment is one object and one case here sets it. Held for the whole of that
/// case, on the convention `the_turn_cap_under_attack_pass_2.rs` sets.
static ENVIRONMENT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

/// A member's session, satisfying the stdin→stdout contract and costing nothing.
///
/// It streams one refused tool call before its verdict, so the transcript is not empty and the
/// refusal has something to reach.
fn a_session(at: &std::path::Path, reached: bool) -> String {
    std::fs::write(
        at,
        format!(
            "#!/bin/sh\ncat > /dev/null\n\
             printf '{{\"event\":\"tool.requested\",\"call_id\":\"t1\",\"tool_name\":\"Bash\"}}\\n'\n\
             printf '{{\"event\":\"tool.decided\",\"call_id\":\"t1\",\"decided_by\":\"frame\",\
             \"seam\":\"hook\",\"decision\":{{\"decision\":\"deny\",\"reason\":\
             \"this step admits [file.read] and Bash is [shell], which it does not\"}}}}\\n'\n\
             printf '{{\"reached\": {reached}, \"note\": \"the member reported\"}}\\n'\n"
        ),
    )
    .expect("the session program is written");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(0o755))
        .expect("the session program is runnable");
    at.display().to_string()
}

/// A started swarm, served by a real [`Server`], with a coordinator and one member holding an
/// assignment.
async fn a_swarm_with_a_member(
    data: &tempdir::TempDir,
    slug: &str,
) -> (Arc<Server>, Arc<Swarm>, String, String) {
    a_swarm_with_a_member_state(data, slug, true).await
}
async fn a_swarm_with_a_member_state(
    data: &tempdir::TempDir,
    slug: &str,
    started: bool,
) -> (Arc<Server>, Arc<Swarm>, String, String) {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Arc::new(
        Server::start(spec, data.path().to_path_buf())
            .await
            .expect("the server starts"),
    );
    populate(server, slug, started).await
}
async fn populate(
    server: Arc<Server>,
    slug: &str,
    started: bool,
) -> (Arc<Server>, Arc<Swarm>, String, String) {
    let swarm = server.open(slug).await.expect("a swarm opens");

    let issue = async |actor: Option<&str>, command: &str, input: Json| {
        swarm
            .issue(
                actor,
                command,
                input.as_object().expect("an object").clone(),
                &format!("{command}:{}", input),
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
    if started {
        issue(
            None,
            "swarm.manager.StartSwarm",
            json!({"swarm_id": swarm_id, "started_at": "2026-09-13T10:01:00Z"}),
        )
        .await;
    }

    issue(
        None,
        "swarm.goal.SetGoal",
        json!({"swarm_id": swarm_id, "text": "two agents, working"}),
    )
    .await;
    let goal = swarm.instances("swarm.goal.Goal").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();

    swarm
        .ensure_agent(
            trigger::COORDINATOR,
            trigger::COORDINATOR_ROLE,
            "The coordinator",
        )
        .await
        .expect("the coordinator is a record");
    swarm
        .ensure_agent("builder", trigger::WORKER_ROLE, "A member that builds")
        .await
        .expect("the member is a record");

    // What a coordinator does with `swarm do`: one outcome, the sign-off it runs under, and what it
    // may not do (`swarm/AGENTS.md` §4 step 4). The binding on `AssignmentPosted` records it.
    issue(
        Some("swarm.agent.Coordinator"),
        "swarm.agent.Assign",
        json!({"agent_id": "builder", "outcome": "make the suite green",
               "signoff": "Timo, verbatim", "forbidden": "do not touch the planning store",
               "artifact_ref": "story:spawn-a-second-agent"}),
    )
    .await;

    let assignment = swarm.instances("swarm.agent.Assignment").await[0]["id"]
        .as_str()
        .expect("the binding recorded the assignment")
        .to_owned();
    (server, swarm, goal, assignment)
}

async fn post(address: std::net::SocketAddr, path: &str, input: Json) -> (u16, Json) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let body = json!({"input":input}).to_string();
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    stream.write_all(format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
    let mut reply = String::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(8),
        stream.read_to_string(&mut reply),
    )
    .await
    .unwrap()
    .unwrap();
    let (headers, body) = reply.split_once("\r\n\r\n").unwrap();
    (
        headers.split_whitespace().nth(1).unwrap().parse().unwrap(),
        serde_json::from_str(body).unwrap(),
    )
}

/// FinishAssignment and the subsequent GoIdle are separate world-lock acquisitions. Poll the
/// actual public worker entrypoint up to the first pending persistence after its observed spend,
/// then let the real HTTP pause handler queue before the worker's next acquisition.
#[tokio::test]
async fn pause_after_finish_does_not_strand_a_worker_in_working() {
    use std::{
        future::{Future, poll_fn},
        task::Poll,
        time::Duration,
    };
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("control-adversary-finish").unwrap();
    let program = a_session(&data.path().join("session.sh"), true);
    unsafe {
        std::env::set_var("SWARM_COORDINATOR", program);
    }
    let (server, swarm, _, _) = a_swarm_with_a_member(&data, "race").await;
    let id = swarm.instances("swarm.manager.Swarm").await[0]["id"].clone();
    swarm
        .issue(
            Some("swarm.agent.Worker"),
            "swarm.agent.TakeAssignment",
            json!({"agent_id":"builder", "ref":"story:spawn-a-second-agent", "msg":"working"})
                .as_object()
                .unwrap()
                .clone(),
            "take",
        )
        .await
        .unwrap();
    let assignment = swarm_server::coordinator::Assignment::from_row(
        &swarm.view("swarm.agent.OpenAssignments").await.unwrap()[0],
    )
    .unwrap();
    let mut work = Box::pin(swarm_server::coordinator::work_an_assignment(
        &swarm,
        &assignment,
        1,
    ));
    tokio::time::timeout(
        Duration::from_secs(8),
        poll_fn(|cx| match work.as_mut().poll(cx) {
            Poll::Ready(result) => {
                panic!("worker completed before persistence barrier: {result:?}")
            }
            Poll::Pending if swarm.spend_by_agent("builder").1 == 1 => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }),
    )
    .await
    .expect("worker reaches finish persistence");

    let (entered, reached) = tokio::sync::oneshot::channel();
    let entered = Arc::new(std::sync::Mutex::new(Some(entered)));
    let routes = swarm_server::http::routes(Arc::clone(&server)).layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let entered = Arc::clone(&entered);
            async move {
                if let Some(entered) = entered.lock().unwrap().take() {
                    entered.send(()).unwrap();
                }
                // No extra yield: this single-thread runtime polls the handler until it queues
                // on the world lock before the receiver can continue the held worker future.
                next.run(request).await
            }
        },
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let serving = tokio::spawn(async move { axum::serve(listener, routes).await.unwrap() });
    let pause = tokio::spawn(post(
        address,
        "/swarms/race/commands/swarm.manager.PauseSwarm",
        json!({"swarm_id":id}),
    ));
    tokio::time::timeout(Duration::from_secs(8), reached)
        .await
        .unwrap()
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(8), work)
        .await
        .unwrap();
    assert!(
        result.is_ok(),
        "the committed assignment answer is retained: {result:?}"
    );
    let (status, body) = pause.await.unwrap();
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["outcome"], "paused");
    assert_eq!(
        swarm.instances("swarm.agent.Assignment").await[0]["state"],
        "Done"
    );
    let (status, body) = post(
        address,
        "/swarms/race/commands/swarm.manager.ResumeSwarm",
        json!({"swarm_id":id}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    trigger::work_the_assignments(&server, &swarm).await;
    assert!(
        swarm
            .view("swarm.agent.OpenAssignments")
            .await
            .unwrap()
            .is_empty()
    );
    let builder = swarm
        .instances("swarm.agent.Agent")
        .await
        .into_iter()
        .find(|agent| agent["id"] == "builder")
        .unwrap();
    serving.abort();
    assert_eq!(
        builder["state"], "Idle",
        "a finished assignment must not leave its worker permanently Working across pause/resume"
    );
}
