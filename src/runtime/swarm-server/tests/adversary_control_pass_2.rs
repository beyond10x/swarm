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

/// Child mode of this executable, matching the original stdin/usage/socket/verdict fixture.
#[test]
fn publication_fixture_process() {
    use std::io::{Read, Write};
    if !std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|args| args == ["--exact", "publication_fixture_process"])
    {
        return;
    }
    let asked: Json = serde_json::from_reader(std::io::stdin().lock()).unwrap();
    println!("{}", json!({"event":"usage", "usage":{"input_tokens":7}}));
    std::io::stdout().flush().unwrap();
    let mut barrier = std::os::unix::net::UnixStream::connect(
        std::env::var_os("SWARM_PUBLICATION_SOCKET").unwrap(),
    )
    .unwrap();
    writeln!(barrier, "{}", asked["agent"].as_str().unwrap()).unwrap();
    let mut release = [0];
    barrier.read_exact(&mut release).unwrap();
    assert_eq!(release, [b'x']);
    println!("{}", json!({"reached":true, "note":"released"}));
    std::io::stdout().flush().unwrap();
    // Keep the verdict last: the parent must not receive the test harness's trailing summary.
    std::process::exit(0);
}

async fn publication_contention(
    closing: &'static str,
    opening: &'static str,
    expected: &'static str,
) {
    use std::{
        future::{Future, poll_fn},
        task::Poll,
        time::Duration,
    };
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let _environment = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("pub").unwrap();
    let socket = data.path().join("p.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    unsafe {
        std::env::set_var("SWARM_PUBLICATION_SOCKET", &socket);
        std::env::set_var(
            "SWARM_COORDINATOR",
            format!(
                "{} --exact publication_fixture_process --nocapture",
                std::env::current_exe().unwrap().display()
            ),
        );
    }
    let (server, swarm, _, _) = a_swarm_with_a_member(&data, "race").await;
    let id = swarm.instances("swarm.manager.Swarm").await[0]["id"].clone();
    swarm
        .ensure_agent("second", "Worker", "second worker")
        .await
        .unwrap();
    swarm.issue(Some("swarm.agent.Coordinator"), "swarm.agent.Assign",
        json!({"agent_id":"second", "outcome":"second assignment", "signoff":"approved", "forbidden":"no model calls", "artifact_ref":null}).as_object().unwrap().clone(), "second-assignment").await.unwrap();
    for actor in ["builder", "second"] {
        swarm
            .issue(
                Some("swarm.agent.Worker"),
                "swarm.agent.TakeAssignment",
                json!({"agent_id":actor, "ref":null, "msg":"working"})
                    .as_object()
                    .unwrap()
                    .clone(),
                actor,
            )
            .await
            .unwrap();
    }
    let rows = swarm.view("swarm.agent.OpenAssignments").await.unwrap();
    let assignments: Vec<_> = rows
        .iter()
        .map(|row| swarm_server::coordinator::Assignment::from_row(row).unwrap())
        .collect();
    let first = assignments
        .iter()
        .find(|a| a.agent_id == "builder")
        .unwrap();
    let second = assignments.iter().find(|a| a.agent_id == "second").unwrap();
    let mut first_work = Box::pin(swarm_server::coordinator::work_an_assignment(
        &swarm, first, 1,
    ));
    let mut second_work = Box::pin(swarm_server::coordinator::work_an_assignment(
        &swarm, second, 1,
    ));
    let mut events = swarm.watch();
    // Hold each real turn after the host observes usage but before its fake process returns.
    for (work, actor) in [(&mut first_work, "builder"), (&mut second_work, "second")] {
        tokio::time::timeout(
            Duration::from_secs(8),
            poll_fn(|cx| {
                assert!(
                    work.as_mut().poll(cx).is_pending(),
                    "process must still be blocked on its socket"
                );
                while let Ok(event) = events.try_recv() {
                    if let swarm_server::swarm::What::Agent { agent, event, .. } = event.what
                        && agent == actor
                        && event["event"] == "usage"
                    {
                        return Poll::Ready(());
                    }
                }
                Poll::Pending
            }),
        )
        .await
        .unwrap();
    }
    let mut barriers = std::collections::BTreeMap::new();
    for _ in 0..2 {
        let (stream, _) = tokio::time::timeout(Duration::from_secs(8), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut reader = tokio::io::BufReader::new(stream);
        let mut actor = String::new();
        reader.read_line(&mut actor).await.unwrap();
        barriers.insert(actor.trim().to_owned(), reader.into_inner());
    }
    barriers
        .get_mut("builder")
        .unwrap()
        .write_all(b"x")
        .await
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(8),
        poll_fn(|cx| {
            assert!(
                first_work.as_mut().poll(cx).is_pending(),
                "first result must yield at persistence"
            );
            if swarm.spend_by_agent("builder").1 == 1 {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
    )
    .await
    .unwrap();
    // The first worker now holds publication while FinishAssignment awaits persistence.
    let (entered, reached) = tokio::sync::oneshot::channel();
    let entered = Arc::new(std::sync::Mutex::new(Some(entered)));
    let routes = swarm_server::http::routes(Arc::clone(&server)).layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let entered = Arc::clone(&entered);
            async move {
                if let Some(entered) = entered.lock().unwrap().take() {
                    entered.send(()).unwrap();
                }
                // On this single-thread executor, the real handler reaches its lock wait before
                // the signal receiver resumes. Middleware only observes handler entry.
                next.run(request).await
            }
        },
    ));
    let http_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = http_listener.local_addr().unwrap();
    let serving = tokio::spawn(async move {
        axum::serve(http_listener, routes).await.unwrap();
    });
    let closing_path = format!("/swarms/race/commands/swarm.manager.{closing}");
    let closing_input = json!({"swarm_id":id, "stopped_at":"2026-09-13T20:00:00Z"});
    let closed = tokio::spawn(async move { post(address, &closing_path, closing_input).await });
    tokio::time::timeout(Duration::from_secs(8), reached)
        .await
        .unwrap()
        .unwrap();
    // Queue a second finished worker behind the lifecycle handler's publication acquisition.
    barriers
        .get_mut("second")
        .unwrap()
        .write_all(b"x")
        .await
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(8),
        poll_fn(|cx| {
            assert!(
                second_work.as_mut().poll(cx).is_pending(),
                "second publication must be queued"
            );
            if swarm.spend_by_agent("second").1 == 1 {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
    )
    .await
    .unwrap();
    let (first_answer, second_answer, response) =
        tokio::time::timeout(Duration::from_secs(8), async {
            tokio::join!(first_work, second_work, closed)
        })
        .await
        .expect("publication waiters cannot deadlock closing quiescence");
    assert!(
        first_answer.is_ok(),
        "first result holds publication: {first_answer:?}"
    );
    assert!(
        matches!(
            second_answer,
            Err(swarm_server::coordinator::Unfinished::Unreportable { .. })
        ),
        "second result must lose to lifecycle: {second_answer:?}"
    );
    let (status, body) = response.unwrap();
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["outcome"], expected);
    for (actor, wanted) in [("builder", "Idle"), ("second", "Working")] {
        let agent = swarm
            .instances("swarm.agent.Agent")
            .await
            .into_iter()
            .find(|a| a["id"] == actor)
            .unwrap();
        assert_eq!(agent["state"], wanted);
        assert_eq!(swarm.spend_by_agent(actor).1, 1);
        assert_eq!(swarm.spend_by_agent(actor).0.input_tokens, 7);
    }
    assert_eq!(
        swarm
            .view("swarm.agent.OpenAssignments")
            .await
            .unwrap()
            .len(),
        1
    );
    let (status, body) = post(
        address,
        &format!("/swarms/race/commands/swarm.manager.{opening}"),
        json!({"swarm_id":id,"started_at":"2026-09-13T20:01:00Z"}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    for _ in 0..2 {
        trigger::work_the_assignments(&server, &swarm).await;
    }
    let (stream, _) = tokio::time::timeout(Duration::from_secs(8), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut reader = tokio::io::BufReader::new(stream);
    let mut actor = String::new();
    reader.read_line(&mut actor).await.unwrap();
    assert_eq!(
        actor.trim(),
        "second",
        "only the interrupted assignment continues"
    );
    reader.get_mut().write_all(b"x").await.unwrap();
    tokio::time::timeout(Duration::from_secs(8), async {
        while server.turns_in_flight() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        swarm
            .view("swarm.agent.OpenAssignments")
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(swarm.spend_by_agent("builder").1, 1);
    assert_eq!(swarm.spend_by_agent("second").1, 2);
    assert_eq!(swarm.spend_by_agent("second").0.input_tokens, 14);
    for agent in swarm.instances("swarm.agent.Agent").await {
        if agent["id"] == "builder" || agent["id"] == "second" {
            assert_eq!(agent["state"], "Idle");
        }
    }
    let history = swarm.history(100).await.unwrap();
    assert_eq!(
        history
            .iter()
            .filter(|e| e.name == "swarm.agent.AssignmentDone")
            .count(),
        2
    );
    serving.abort();
}

#[tokio::test]
async fn pause_between_two_worker_publications_retires_waiters_and_resumes_only_the_loser() {
    publication_contention("PauseSwarm", "ResumeSwarm", "paused").await;
}

#[tokio::test]
async fn stop_between_two_worker_publications_retires_waiters_and_restarts_only_the_loser() {
    publication_contention("StopSwarm", "StartSwarm", "stopped").await;
}
