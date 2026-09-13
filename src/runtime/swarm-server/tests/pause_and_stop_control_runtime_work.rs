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

/// Waits for every turn the trigger spawned.
async fn settle(server: &Arc<Server>) {
    for _ in 0..600 {
        if server.turns_in_flight() == 0 {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("a turn never finished");
}

#[tokio::test]
async fn paused_assignments_do_not_launch() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("lifecycle").unwrap();
    let program = a_session(&data.path().join("session.sh"), true);
    unsafe { std::env::set_var("SWARM_COORDINATOR", &program) };
    let (server, swarm, _, _) = a_swarm_with_a_member(&data, "paused").await;
    let id = swarm.instances("swarm.manager.Swarm").await[0]["id"].clone();
    swarm
        .issue(
            None,
            "swarm.manager.PauseSwarm",
            json!({"swarm_id": id}).as_object().unwrap().clone(),
            "pause",
        )
        .await
        .unwrap();
    trigger::work_the_assignments(&server, &swarm).await;
    settle(&server).await;
    assert_eq!(
        swarm.spend_by_agent("builder").1,
        0,
        "Paused work must never launch"
    );
}

// Child mode of this test executable. No shell, model, polling marker files or paid launches.
#[test]
fn fake_process() {
    if !std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|args| args == ["--exact", "fake_process"])
    {
        return;
    }
    use std::io::{Read, Write};
    let Ok(socket) = std::env::var("SWARM_TEST_SOCKET") else {
        return;
    };
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let asked: Json = serde_json::from_str(&input).unwrap();
    let mut descendant = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "fake_descendant", "--nocapture"])
        .env("SWARM_TEST_ASKED", &input)
        .spawn()
        .unwrap();
    println!(
        "{}",
        json!({"event":"usage", "request_id":"one", "usage":{"input_tokens":17}})
    );
    std::io::stdout().flush().unwrap();
    let mut barrier = std::os::unix::net::UnixStream::connect(socket).unwrap();
    writeln!(barrier, "{}", json!({"pid":std::process::id(), "agent":asked["agent"], "swarm":asked["swarm"], "role":"parent"})).unwrap();
    let mut byte = [0];
    barrier.read_exact(&mut byte).unwrap();
    // Natural completion must also collect its ordinary child.
    if byte[0] != b'e' {
        descendant.kill().unwrap();
        descendant.wait().unwrap();
    }
    println!("{{\"reached\":true,\"note\":\"barrier released\"}}");
    std::io::stdout().flush().unwrap();
    std::process::exit(0);
}

#[test]
fn fake_descendant() {
    if !std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|args| args == ["--exact", "fake_descendant"])
    {
        return;
    }
    use std::io::{Read, Write};
    let Ok(socket) = std::env::var("SWARM_TEST_SOCKET") else {
        return;
    };
    let asked: Json = serde_json::from_str(&std::env::var("SWARM_TEST_ASKED").unwrap()).unwrap();
    let mut barrier = std::os::unix::net::UnixStream::connect(socket).unwrap();
    writeln!(barrier, "{}", json!({"pid":std::process::id(), "agent":asked["agent"], "swarm":asked["swarm"], "role":"descendant"})).unwrap();
    let mut byte = [0];
    let _ = barrier.read_exact(&mut byte);
    std::process::exit(0);
}

async fn http(server: &Arc<Server>) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let routes = swarm_server::http::routes(Arc::clone(server));
    (
        address,
        tokio::spawn(async move {
            axum::serve(listener, routes).await.unwrap();
        }),
    )
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
async fn handshake(listener: &tokio::net::UnixListener) -> (Json, tokio::net::UnixStream) {
    use tokio::io::AsyncBufReadExt;
    let (stream, _) = tokio::time::timeout(std::time::Duration::from_secs(8), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut reader = tokio::io::BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    (serde_json::from_str(&line).unwrap(), reader.into_inner())
}
fn dead(pid: &Json) {
    // Signal 0 also sees zombies: success therefore means both termination and reaping.
    assert_eq!(
        unsafe { libc::kill(pid.as_i64().unwrap() as i32, 0) },
        -1,
        "process {pid} survived acknowledgment"
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}
async fn cancelling(command: &str, restart: &str) {
    use tokio::io::AsyncWriteExt;
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("lifecycle-http").unwrap();
    let socket = data.path().join("barrier.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    unsafe {
        std::env::set_var(
            "SWARM_COORDINATOR",
            format!(
                "{} --exact fake_process --nocapture",
                std::env::current_exe().unwrap().display()
            ),
        );
        std::env::set_var("SWARM_TEST_SOCKET", &socket);
    }
    let (server, swarm, goal, _) = a_swarm_with_a_member(&data, "active").await;
    let id = swarm.instances("swarm.manager.Swarm").await[0]["id"].clone();
    swarm
        .issue(
            None,
            "swarm.goal.Pursue",
            json!({"goal_id":goal,"iterations":1})
                .as_object()
                .unwrap()
                .clone(),
            "pursue",
        )
        .await
        .unwrap();
    let (address, serving) = http(&server).await;
    let (status, refusal) = post(address, "/swarms/active/quiesce", json!({})).await;
    assert_eq!(status, 400);
    assert_eq!(refusal["kind"], "command");
    let mut observed = swarm.watch();
    trigger::ask_the_coordinator(&server, &swarm).await;
    trigger::work_the_assignments(&server, &swarm).await;
    let mut processes = Vec::new();
    for _ in 0..4 {
        processes.push(handshake(&listener).await);
    }
    // Wait for actual runtime observation of both usage records, not just child writes.
    let observing = async {
        let mut agents = std::collections::BTreeSet::new();
        while agents.len() < 2 {
            if let swarm_server::swarm::What::Agent { agent, event, .. } =
                observed.recv().await.unwrap().what
                && event["event"] == "usage"
            {
                agents.insert(agent);
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(8), observing)
        .await
        .unwrap();
    let input = json!({"swarm_id":id,"stopped_at":"2026-09-13T12:00:00Z"});
    let path = format!("/swarms/active/commands/swarm.manager.{command}");
    let (status, response) = post(address, &path, input.clone()).await;
    assert_eq!(status, 200, "{response}");
    assert_eq!(
        response["outcome"],
        if command == "PauseSwarm" {
            "paused"
        } else {
            "stopped"
        }
    );
    assert_eq!(server.turns_in_flight(), 0);
    for (process, _) in &processes {
        dead(&process["pid"]);
    }
    for agent in ["coordinator", "builder"] {
        let (spent, attempts) = swarm.spend_by_agent(agent);
        assert_eq!(attempts, 1);
        assert_eq!(spent.input_tokens, 17);
    }
    assert!(swarm.history(100).await.unwrap().iter().all(|e| !matches!(
        e.name.as_str(),
        "swarm.goal.GoalReached" | "swarm.agent.AssignmentDone"
    )));
    let (status, repeated) = post(address, &path, input).await;
    assert_eq!(status, 200);
    assert_eq!(repeated["outcome"], "wrong-state");
    let (status, cleanup) = post(address, "/swarms/active/quiesce", json!({})).await;
    assert_eq!(status, 200);
    assert_eq!(cleanup, json!({"quiescent":true}));
    // A new generation continues each unit once, even when dispatch is asked twice.
    let (status, resumed) = post(
        address,
        &format!("/swarms/active/commands/swarm.manager.{restart}"),
        json!({"swarm_id":id,"started_at":"2026-09-13T12:01:00Z"}),
    )
    .await;
    assert_eq!(status, 200, "{resumed}");
    for _ in 0..2 {
        trigger::ask_the_coordinator(&server, &swarm).await;
        trigger::work_the_assignments(&server, &swarm).await;
    }
    let mut resumed_processes = Vec::new();
    for _ in 0..4 {
        resumed_processes.push(handshake(&listener).await);
    }
    for (process, stream) in &mut resumed_processes {
        if process["role"] == "parent" {
            stream.write_all(b"x").await.unwrap();
        }
    }
    settle(&server).await;
    for agent in ["coordinator", "builder"] {
        assert_eq!(swarm.spend_by_agent(agent).1, 2);
    }
    let log = swarm.history(100).await.unwrap();
    for event in ["swarm.goal.GoalReached", "swarm.agent.AssignmentDone"] {
        assert_eq!(log.iter().filter(|e| e.name == event).count(), 1);
    }
    assert!(
        std::fs::read_dir(swarm.dir().join("turns"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|s| s == "jsonl"))
            .count()
            >= 5
    );
    serving.abort();
    unsafe {
        std::env::remove_var("SWARM_TEST_SOCKET");
    }
}
#[tokio::test]
async fn pause_reaps_coordinator_worker_and_descendants_then_resume_continues() {
    cancelling("PauseSwarm", "ResumeSwarm").await;
}
#[tokio::test]
async fn stop_reaps_coordinator_worker_and_descendants_then_start_continues() {
    cancelling("StopSwarm", "StartSwarm").await;
}

#[tokio::test]
async fn every_nonrunning_state_refuses_goals_and_assignments_before_and_after_replay() {
    let _guard = ENVIRONMENT.lock().await;
    for state in ["Created", "Paused", "Stopped", "Deleted"] {
        let data = tempdir::TempDir::new("nonrunning").unwrap();
        let program = a_session(&data.path().join("session.sh"), true);
        unsafe {
            std::env::set_var("SWARM_COORDINATOR", program);
        }
        let (server, swarm, goal, _) =
            a_swarm_with_a_member_state(&data, "nonrunning", state != "Created").await;
        let id = swarm.instances("swarm.manager.Swarm").await[0]["id"].clone();
        let (addr, serving) = http(&server).await;
        post(
            addr,
            "/swarms/nonrunning/commands/swarm.goal.Pursue",
            json!({"goal_id":goal,"iterations":1}),
        )
        .await;
        let command = match state {
            "Paused" => Some("PauseSwarm"),
            "Stopped" | "Deleted" => Some("StopSwarm"),
            _ => None,
        };
        if let Some(command) = command {
            assert_eq!(
                post(
                    addr,
                    &format!("/swarms/nonrunning/commands/swarm.manager.{command}"),
                    json!({"swarm_id":id,"stopped_at":"2026-09-13T10:00:00Z"})
                )
                .await
                .0,
                200
            );
        }
        if state == "Deleted" {
            assert_eq!(
                post(
                    addr,
                    "/swarms/nonrunning/commands/swarm.manager.DeleteSwarm",
                    json!({"swarm_id":id})
                )
                .await
                .0,
                200
            );
        }
        for _ in 0..2 {
            trigger::ask_the_coordinator(&server, &swarm).await;
            trigger::work_the_assignments(&server, &swarm).await;
        }
        settle(&server).await;
        assert_eq!(server.turns_in_flight(), 0);
        assert_eq!(swarm.spend_by_agent("builder").1, 0, "{state}");
        assert_eq!(swarm.spend_by_agent("coordinator").1, 0, "{state}");
        serving.abort();
        let _ = serving.await;
        drop(swarm);
        drop(server);
        let replayed = Arc::new(
            Server::start(Spec::load(kernel()).unwrap(), data.path().to_path_buf())
                .await
                .unwrap(),
        );
        let swarm = replayed.get("nonrunning").await.unwrap();
        assert_eq!(
            swarm.instances("swarm.manager.Swarm").await[0]["state"],
            state
        );
        trigger::ask_the_coordinator(&replayed, &swarm).await;
        trigger::work_the_assignments(&replayed, &swarm).await;
        settle(&replayed).await;
        assert_eq!(swarm.spend_by_agent("builder").1, 0, "replayed {state}");
        assert_eq!(swarm.spend_by_agent("coordinator").1, 0, "replayed {state}");
    }
}

#[tokio::test]
async fn pause_isolates_the_other_swarm_and_reaps_children_of_exited_launchers() {
    use tokio::io::AsyncWriteExt;
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("isolation").unwrap();
    let socket = data.path().join("barrier.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    unsafe {
        std::env::set_var(
            "SWARM_COORDINATOR",
            format!(
                "{} --exact fake_process --nocapture",
                std::env::current_exe().unwrap().display()
            ),
        );
        std::env::set_var("SWARM_TEST_SOCKET", &socket);
    }
    let (server, active, _, _) = a_swarm_with_a_member(&data, "active").await;
    let (_, other, _, _) = populate(Arc::clone(&server), "other", true).await;
    let (addr, serving) = http(&server).await;
    trigger::work_the_assignments(&server, &active).await;
    trigger::work_the_assignments(&server, &other).await;
    let mut processes = Vec::new();
    for _ in 0..4 {
        processes.push(handshake(&listener).await);
    }
    let id = active.instances("swarm.manager.Swarm").await[0]["id"].clone();
    let (status, body) = post(
        addr,
        "/swarms/active/commands/swarm.manager.PauseSwarm",
        json!({"swarm_id":id}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    for (process, stream) in &mut processes {
        if process["swarm"] == "active" {
            dead(&process["pid"]);
        } else {
            assert_eq!(
                unsafe { libc::kill(process["pid"].as_i64().unwrap() as i32, 0) },
                0
            );
            if process["role"] == "parent" {
                stream.write_all(b"e").await.unwrap();
            }
        }
    }
    settle(&server).await;
    for (process, _) in &processes {
        dead(&process["pid"]);
    }
    assert_eq!(
        active.instances("swarm.agent.Assignment").await[0]["state"],
        "Assigned"
    );
    assert_eq!(
        other.instances("swarm.agent.Assignment").await[0]["state"],
        "Done"
    );
    serving.abort();
    unsafe {
        std::env::remove_var("SWARM_TEST_SOCKET");
    }
}

#[tokio::test]
async fn aborted_turn_preserves_observed_usage_until_pause_reaps() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("aborted-evidence").unwrap();
    let socket = data.path().join("barrier.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    unsafe {
        std::env::set_var(
            "SWARM_COORDINATOR",
            format!(
                "{} --exact fake_process --nocapture",
                std::env::current_exe().unwrap().display()
            ),
        );
        std::env::set_var("SWARM_TEST_SOCKET", &socket);
    }
    let (server, swarm, goal, _) = a_swarm_with_a_member(&data, "active").await;
    let id = swarm.instances("swarm.manager.Swarm").await[0]["id"].clone();
    let mut observed = swarm.watch();
    let (addr, serving) = http(&server).await;
    let working = Arc::clone(&swarm);
    let task = tokio::spawn(async move {
        swarm_server::coordinator::take_a_turn(&working, "coordinator", &goal, "goal", 1).await
    });
    let parent = handshake(&listener).await;
    let descendant = handshake(&listener).await;
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        loop {
            if let swarm_server::swarm::What::Agent { event, .. } =
                observed.recv().await.unwrap().what
                && event["event"] == "usage"
            {
                break;
            }
        }
    })
    .await
    .unwrap();
    task.abort();
    let _ = task.await;
    let (status, body) = post(
        addr,
        "/swarms/active/commands/swarm.manager.PauseSwarm",
        json!({"swarm_id":id}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    dead(&parent.0["pid"]);
    dead(&descendant.0["pid"]);
    let (spent, attempts) = swarm.spend_by_agent("coordinator");
    assert_eq!(attempts, 1);
    assert_eq!(spent.input_tokens, 17);
    serving.abort();
    unsafe {
        std::env::remove_var("SWARM_TEST_SOCKET");
    }
}
