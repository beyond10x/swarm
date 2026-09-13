//! Process-host lifecycle, separate from the interpreted domain. A claim spans dispatch through
//! evidence persistence and result publication. Closing admission invalidates all existing claims.
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::{Notify, watch};

const CLEANUP_BOUND: Duration = Duration::from_secs(2);

tokio::task_local! { static CURRENT: Arc<Registration>; }

pub(crate) struct Control {
    state: Mutex<State>,
    changed: Notify,
    pub lifecycle: tokio::sync::Mutex<()>,
    #[cfg(test)]
    fail_termination: std::sync::atomic::AtomicBool,
    #[cfg(test)]
    result_barriers: Mutex<Option<(Arc<tokio::sync::Barrier>, Arc<tokio::sync::Barrier>)>>,
}
struct State {
    running: bool,
    generation: u64,
    turns: BTreeMap<String, Arc<Turn>>,
}
pub(crate) struct Turn {
    generation: u64,
    cancel: watch::Sender<u64>,
    failure: Mutex<Option<String>>,
}
pub(crate) struct Claim {
    registration: Arc<Registration>,
}
struct Registration {
    control: Arc<Control>,
    key: String,
    turn: Arc<Turn>,
}
impl Drop for Registration {
    fn drop(&mut self) {
        self.control
            .state
            .lock()
            .expect("not poisoned")
            .turns
            .remove(&self.key);
        self.control.changed.notify_waiters();
    }
}
impl Drop for Claim {
    fn drop(&mut self) {
        self.registration.turn.cancel.send_modify(|n| *n += 1);
    }
}
impl Claim {
    pub async fn scope<F: Future>(self, future: F) -> F::Output {
        CURRENT.scope(Arc::clone(&self.registration), future).await
    }
}
impl Control {
    pub fn new(running: bool) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                running,
                generation: 0,
                turns: BTreeMap::new(),
            }),
            changed: Notify::new(),
            lifecycle: tokio::sync::Mutex::new(()),
            #[cfg(test)]
            fail_termination: std::sync::atomic::AtomicBool::new(false),
            #[cfg(test)]
            result_barriers: Mutex::new(None),
        })
    }
    #[cfg(test)]
    pub async fn before_result(&self) {
        let barriers = self.result_barriers.lock().expect("not poisoned").clone();
        if let Some((ready, release)) = barriers {
            ready.wait().await;
            release.wait().await;
        }
    }
    pub fn running(&self) -> bool {
        self.state.lock().expect("not poisoned").running
    }
    pub fn admit(self: &Arc<Self>, key: String) -> Option<Claim> {
        let mut state = self.state.lock().expect("not poisoned");
        if !state.running || state.turns.contains_key(&key) {
            return None;
        }
        let turn = Arc::new(Turn {
            generation: state.generation,
            cancel: watch::channel(0).0,
            failure: Mutex::new(None),
        });
        state.turns.insert(key.clone(), Arc::clone(&turn));
        Some(Claim {
            registration: Arc::new(Registration {
                control: Arc::clone(self),
                key,
                turn,
            }),
        })
    }
    pub fn check(&self) -> Result<(), String> {
        CURRENT
            .try_with(|turn| {
                let state = self.state.lock().expect("not poisoned");
                if state.running
                    && state.generation == turn.turn.generation
                    && std::ptr::eq(self, turn.control.as_ref())
                {
                    Ok(())
                } else {
                    Err("the turn was cancelled by the swarm lifecycle".to_owned())
                }
            })
            .unwrap_or(Ok(()))
    }
    pub fn in_turn() -> bool {
        CURRENT.try_with(|_| ()).is_ok()
    }
    // Called under the world's command lock, after the domain transition commits.
    pub fn close(&self) {
        let mut state = self.state.lock().expect("not poisoned");
        state.running = false;
        state.generation += 1;
        for turn in state.turns.values() {
            turn.cancel.send_modify(|n| *n += 1);
        }
    }
    pub fn open(&self) {
        self.state.lock().expect("not poisoned").running = true;
    }
    pub async fn quiesce(&self) -> Result<(), String> {
        {
            let state = self.state.lock().expect("not poisoned");
            if state.running {
                return Err("quiescence requires a non-running swarm".to_owned());
            }
            for turn in state.turns.values() {
                *turn.failure.lock().expect("not poisoned") = None;
                turn.cancel.send_modify(|n| *n += 1);
            }
        }
        let waiting = async {
            loop {
                let changed = self.changed.notified();
                {
                    let state = self.state.lock().expect("not poisoned");
                    if state.turns.is_empty() {
                        return Ok(());
                    }
                    for turn in state.turns.values() {
                        if let Some(why) = turn.failure.lock().expect("not poisoned").clone() {
                            return Err(why);
                        }
                    }
                }
                changed.await;
            }
        };
        tokio::time::timeout(CLEANUP_BOUND + Duration::from_secs(1), waiting).await
            .map_err(|_| "failed quiescence: turns did not finish within 3 seconds; retry POST /swarms/{slug}/quiesce".to_owned())?
    }
    // Admission and OS spawn share this lock: a cancellation cannot miss a just-created child.
    pub fn spawn(&self, command: &mut Command) -> Result<Child, String> {
        let state = self.state.lock().expect("not poisoned");
        let turn = CURRENT
            .try_with(Arc::clone)
            .map_err(|_| "turn has no lifecycle claim")?;
        if !state.running
            || state.generation != turn.turn.generation
            || !std::ptr::eq(self, turn.control.as_ref())
        {
            return Err("turn cancelled before launch".to_owned());
        }
        #[cfg(target_os = "linux")]
        // SAFETY: this only changes orphan adoption for this server; waitpid below selects our group.
        if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        command
            .process_group(0)
            .kill_on_drop(true)
            .spawn()
            .map_err(|why| why.to_string())
    }
    pub fn supervise(
        self: &Arc<Self>,
        mut child: Child,
    ) -> tokio::task::JoinHandle<Result<std::process::ExitStatus, String>> {
        let control = Arc::clone(self);
        let registration = CURRENT.with(Arc::clone);
        let mut cancel = registration.turn.cancel.subscribe();
        let pid = child.id().expect("fresh child") as i32;
        tokio::spawn(async move {
            let turn = &registration.turn;
            loop {
                let cancelled = *cancel.borrow_and_update() != 0;
                if !cancelled {
                    tokio::select! {
                        _ = child.wait() => {},
                        _ = cancel.changed() => {}
                    }
                }
                match terminate(&control, &mut child, pid).await {
                    Ok(status) => return Ok(status),
                    Err(why) => {
                        *turn.failure.lock().expect("not poisoned") = Some(format!(
                            "failed quiescence: {why}; retry POST /swarms/{{slug}}/quiesce"
                        ));
                        control.changed.notify_waiters();
                        // Keep the child and cancellation handle alive for explicit cleanup retry.
                        if cancel.changed().await.is_err() {
                            return Err(why);
                        }
                    }
                }
            }
        })
    }
}

async fn terminate(
    _control: &Control,
    child: &mut Child,
    group: i32,
) -> Result<std::process::ExitStatus, String> {
    #[cfg(test)]
    if _control
        .fail_termination
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        return Err("injected process-group signal failure".to_owned());
    }
    // SIGKILL is deliberate: no grace interval in which a cancelled turn can keep doing work.
    // Ordinary group descendants are included; an intentionally escaped daemon is not containment.
    // SAFETY: negative PID addresses only the fresh child's process group.
    if unsafe { libc::kill(-group, libc::SIGKILL) } != 0 {
        let why = std::io::Error::last_os_error();
        if why.raw_os_error() != Some(libc::ESRCH) {
            return Err(why.to_string());
        }
    }
    tokio::time::timeout(CLEANUP_BOUND, async {
        let status = child.wait().await.map_err(|why| why.to_string())?;
        loop {
            #[cfg(target_os = "linux")]
            // SAFETY: reap only adopted ordinary descendants in this owned process group.
            while unsafe { libc::waitpid(-group, std::ptr::null_mut(), libc::WNOHANG) } > 0 {}
            // SAFETY: signal zero probes existence; it cannot signal unrelated groups.
            if unsafe { libc::kill(-group, 0) } != 0 {
                let why = std::io::Error::last_os_error();
                if why.raw_os_error() == Some(libc::ESRCH) {
                    return Ok(status);
                }
                return Err(why.to_string());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| "process group did not exit within 2 seconds".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        state::Server,
        swarm::{Swarm, What},
    };
    use serde_json::{Value, json};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    async fn fixture() -> (tempdir::TempDir, Arc<Server>, Arc<Swarm>, Value, Value) {
        let data = tempdir::TempDir::new("control-unit").unwrap();
        let spec = ess_runtime::Spec::load(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../core"),
        )
        .unwrap();
        let server = Arc::new(
            Server::start(spec, data.path().to_path_buf())
                .await
                .unwrap(),
        );
        let swarm = server.open("test").await.unwrap();
        issue(&swarm,"swarm.manager.CreateSwarm",json!({"display_name":"test","tmux_session":"test","home":"/unused","created_at":"2026-09-13T10:00:00Z"})).await;
        let id = swarm.instances("swarm.manager.Swarm").await[0]["id"].clone();
        issue(
            &swarm,
            "swarm.manager.StartSwarm",
            json!({"swarm_id":id,"started_at":"2026-09-13T10:00:00Z"}),
        )
        .await;
        issue(
            &swarm,
            "swarm.goal.SetGoal",
            json!({"swarm_id":id,"text":"a goal"}),
        )
        .await;
        let goal = swarm.instances("swarm.goal.Goal").await[0]["id"].clone();
        issue(
            &swarm,
            "swarm.goal.Pursue",
            json!({"goal_id":goal,"iterations":1}),
        )
        .await;
        (data, server, swarm, id, goal)
    }
    async fn issue(swarm: &Swarm, command: &str, input: Value) {
        swarm
            .issue(
                None,
                command,
                input.as_object().unwrap().clone(),
                &uuid::Uuid::new_v4().to_string(),
            )
            .await
            .unwrap();
    }
    async fn serving(server: Arc<Server>) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        (
            addr,
            tokio::spawn(async move {
                axum::serve(listener, crate::http::routes(server))
                    .await
                    .unwrap();
            }),
        )
    }
    async fn post(addr: std::net::SocketAddr, path: &str, input: Value) -> (u16, Value) {
        let body = json!({"input":input}).to_string();
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream.write_all(format!("POST /swarms/test/{path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        let mut response = String::new();
        tokio::time::timeout(Duration::from_secs(8), stream.read_to_string(&mut response))
            .await
            .unwrap()
            .unwrap();
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        (
            head.split_whitespace().nth(1).unwrap().parse().unwrap(),
            serde_json::from_str(body).unwrap(),
        )
    }
    #[tokio::test]
    async fn failed_termination_is_an_http_host_error_and_cleanup_is_retryable() {
        let (_data, server, swarm, id, _) = fixture().await;
        let (addr, serving) = serving(server).await;
        let claim = swarm.control.admit("goal:controlled".to_owned()).unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let control = Arc::clone(&swarm.control);
        let task = tokio::spawn(claim.scope(async move {
            let mut command = Command::new("sh");
            command
                .args(["-c", "read line"])
                .stdin(std::process::Stdio::piped());
            let mut child = control.spawn(&mut command).unwrap();
            let _stdin = child.stdin.take();
            let pid = child.id().unwrap();
            let reaped = control.supervise(child);
            started.send(pid).unwrap();
            reaped.await.unwrap().unwrap();
        }));
        let pid = ready.await.unwrap();
        swarm
            .control
            .fail_termination
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let (status, body) = post(
            addr,
            "commands/swarm.manager.PauseSwarm",
            json!({"swarm_id":id}),
        )
        .await;
        assert_eq!(status, 500);
        assert_eq!(body["kind"], "host");
        assert!(
            body["error"]
                .as_str()
                .unwrap()
                .contains("failed quiescence")
        );
        assert_eq!(
            swarm.instances("swarm.manager.Swarm").await[0]["state"],
            "Paused"
        );
        assert!(!swarm.control.running());
        assert!(swarm.control.admit("new".into()).is_none());
        assert_eq!(
            unsafe { libc::kill(pid as i32, 0) },
            0,
            "forced failure retains the live handle"
        );
        let (status, body) = post(
            addr,
            "commands/swarm.manager.PauseSwarm",
            json!({"swarm_id":id}),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(body["outcome"], "wrong-state");
        let (status, body) = post(
            addr,
            "commands/swarm.manager.StartSwarm",
            json!({"swarm_id":id,"started_at":"2026-09-13T10:00:00Z"}),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(body["outcome"], "wrong-state");
        let (status, _) = post(
            addr,
            "commands/swarm.manager.ResumeSwarm",
            json!({"swarm_id":id}),
        )
        .await;
        assert_eq!(status, 500);
        assert_eq!(
            swarm.instances("swarm.manager.Swarm").await[0]["state"],
            "Paused"
        );
        let (status, _) = post(addr, "quiesce", json!({})).await;
        assert_eq!(status, 500);
        swarm
            .control
            .fail_termination
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let (status, body) = post(addr, "quiesce", json!({})).await;
        assert_eq!(status, 200);
        assert_eq!(body, json!({"quiescent":true}));
        task.await.unwrap();
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
        assert_eq!(
            post(
                addr,
                "commands/swarm.manager.ResumeSwarm",
                json!({"swarm_id":id})
            )
            .await
            .0,
            200
        );
        assert!(swarm.control.running());
        serving.abort();
    }
    #[tokio::test]
    async fn claim_before_pause_cannot_spawn_or_publish_and_ack_waits_for_release() {
        let (_data, server, swarm, id, goal) = fixture().await;
        let claim = swarm.control.admit("goal:waiting".into()).unwrap();
        let mut changes = swarm.watch();
        let (addr, serving) = serving(server).await;
        let pause = tokio::spawn(post(
            addr,
            "commands/swarm.manager.PauseSwarm",
            json!({"swarm_id":id}),
        ));
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                if let What::Applied { outcome, .. } = changes.recv().await.unwrap().what
                    && outcome == "paused"
                {
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert!(
            !pause.is_finished(),
            "a registered pre-launch turn still belongs to cancellation"
        );
        assert!(swarm.control.admit("other".into()).is_none());
        claim
            .scope(async {
                let mut command = Command::new("sh");
                command.args(["-c", "exit 0"]);
                assert!(
                    swarm
                        .control
                        .spawn(&mut command)
                        .unwrap_err()
                        .contains("cancelled before launch")
                );
                let result = swarm
                    .issue(
                        None,
                        "swarm.goal.Evaluate",
                        json!({"goal_id":goal,"reached":true})
                            .as_object()
                            .unwrap()
                            .clone(),
                        "stale",
                    )
                    .await;
                assert!(result.unwrap_err().to_string().contains("cancelled"));
            })
            .await;
        assert_eq!(pause.await.unwrap().0, 200);
        assert_eq!(
            post(
                addr,
                "commands/swarm.manager.ResumeSwarm",
                json!({"swarm_id":id})
            )
            .await
            .0,
            200
        );
        assert!(
            swarm
                .history(100)
                .await
                .unwrap()
                .iter()
                .all(|e| e.name != "swarm.goal.GoalReached")
        );
        serving.abort();
    }
    #[tokio::test]
    async fn aborting_the_turn_does_not_unregister_its_live_process() {
        let (_data, _server, swarm, _id, _goal) = fixture().await;
        let claim = swarm.control.admit("abort".into()).unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let control = Arc::clone(&swarm.control);
        let task = tokio::spawn(claim.scope(async move {
            let mut command = Command::new("sh");
            command
                .args(["-c", "read line"])
                .stdin(std::process::Stdio::piped());
            let mut child = control.spawn(&mut command).unwrap();
            let _stdin = child.stdin.take();
            let pid = child.id().unwrap();
            let _supervisor = control.supervise(child);
            started.send(pid).unwrap();
            std::future::pending::<()>().await;
        }));
        let pid = ready.await.unwrap();
        swarm
            .control
            .fail_termination
            .store(true, std::sync::atomic::Ordering::SeqCst);
        task.abort();
        let _ = task.await;
        swarm.control.close();
        assert!(swarm.control.quiesce().await.is_err());
        assert!(
            !swarm.control.state.lock().unwrap().turns.is_empty(),
            "the supervisor still owns the registration"
        );
        swarm
            .control
            .fail_termination
            .store(false, std::sync::atomic::Ordering::SeqCst);
        swarm.control.quiesce().await.unwrap();
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
    }
    #[tokio::test]
    async fn a_finished_process_cannot_publish_across_pause_and_rapid_resume() {
        let _environment = crate::ENVIRONMENT.lock().await;
        let (data, server, swarm, id, _) = fixture().await;
        let program = data.path().join("verdict.sh");
        std::fs::write(&program,"#!/bin/sh\ncat >/dev/null\nprintf '{\"event\":\"usage\",\"usage\":{\"input_tokens\":9}}\n{\"reached\":true}\n'\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        unsafe {
            std::env::set_var("SWARM_COORDINATOR", &program);
        }
        let ready = Arc::new(tokio::sync::Barrier::new(2));
        let release = Arc::new(tokio::sync::Barrier::new(2));
        *swarm.control.result_barriers.lock().unwrap() =
            Some((Arc::clone(&ready), Arc::clone(&release)));
        let mut observed = swarm.watch();
        let (addr, serving) = serving(Arc::clone(&server)).await;
        crate::trigger::ask_the_coordinator(&server, &swarm).await;
        tokio::time::timeout(Duration::from_secs(8), ready.wait())
            .await
            .unwrap();
        let pause = tokio::spawn(post(
            addr,
            "commands/swarm.manager.PauseSwarm",
            json!({"swarm_id":id}),
        ));
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                if let What::Applied { outcome, .. } = observed.recv().await.unwrap().what
                    && outcome == "paused"
                {
                    break;
                }
            }
        })
        .await
        .unwrap();
        let resume = tokio::spawn(post(
            addr,
            "commands/swarm.manager.ResumeSwarm",
            json!({"swarm_id":id}),
        ));
        release.wait().await;
        assert_eq!(pause.await.unwrap().0, 200);
        assert_eq!(resume.await.unwrap().0, 200);
        assert_eq!(
            swarm.spend_by_agent("coordinator").1,
            1,
            "already-recorded usage must not be counted twice when its verdict is cancelled"
        );
        assert_eq!(swarm.spend_by_agent("coordinator").0.input_tokens, 9);
        assert!(
            swarm
                .history(100)
                .await
                .unwrap()
                .iter()
                .all(|e| e.name != "swarm.goal.GoalReached")
        );
        *swarm.control.result_barriers.lock().unwrap() = None;
        crate::trigger::ask_the_coordinator(&server, &swarm).await;
        tokio::time::timeout(Duration::from_secs(8), async {
            while server.turns_in_flight() != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            swarm
                .history(100)
                .await
                .unwrap()
                .iter()
                .filter(|e| e.name == "swarm.goal.GoalReached")
                .count(),
            1
        );
        serving.abort();
    }
}
