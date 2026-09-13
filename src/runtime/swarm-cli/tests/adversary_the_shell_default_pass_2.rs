//! Adversary, wave 2026-09-13b unit A, pass 2 — the door the unit did not drive end to end.
//!
//! The story's acceptance clause 3: "An operator command reaching the HTTP surface is
//! distinguishable in the record from a command the runtime issued to itself — so 'no operator
//! touched this' becomes a checkable claim."
//!
//! There are two doors that reach that surface. The unit drove the HTTP one directly, in
//! `swarm-server/tests/an_event_says_which_agent_acted.rs`, and it is right: a body with no
//! `agent` records `operator`. The other door is this binary, and it is the one a person has in
//! their hands — `AGENTS.md`'s "Build, run, test" block is `cargo run -p swarm-cli -- --help`,
//! and `--url`, `--swarm` and `--agent` are all `global = true` flags on it.
//!
//! `main.rs`'s new `issuing` says in its own doc comment:
//!
//! > Empty means no settings file was found — a shell a person is typing into, which the record
//! > calls an operator rather than inventing a name for.
//!
//! `settings.agent` is never empty when no settings file is found. `run` builds it as
//!
//! ```ignore
//! agent: cli.agent.clone().or(found.agent).unwrap_or_else(|| "coordinator".to_owned()),
//! ```
//!
//! so the name IS invented, and the name invented is the name of a real member of every swarm this
//! runtime runs. The unit's own case for the claim —
//! `a_shell_with_no_settings_claims_no_agent_rather_than_an_empty_one` — builds
//! `Settings { agent: String::new() }` by hand, and `run` has no branch that produces it.
//!
//! This drives the real binary instead. No server is needed: what is being asked is what goes on
//! the wire, so the wire is a one-shot `TcpListener` that reads the request and answers it.
//!
//! **Nothing here edits an implementation file.** The default is read where it is written and the
//! binary is run as a subprocess.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use serde_json::Value as Json;

/// Where the binary is run from: a fresh empty directory, so `read_config` finds nothing of ours.
///
/// `CARGO_TARGET_TMPDIR` is inside this worktree's own `target/`, which is the build directory the
/// unit brief names and the one place a test may write.
fn a_directory_with_no_settings(name: &str) -> PathBuf {
    let at = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&at);
    std::fs::create_dir_all(&at).expect("a directory to run the binary from");
    at
}

/// `read_config` walks UP from the working directory, so the claim only holds if no ancestor of
/// the run directory carries one either. Asserted rather than assumed: a `.swarm/config.json`
/// somewhere above would make this case pass or fail for a reason that is not the one it names.
fn no_ancestor_carries_settings(from: &Path) {
    let mut at = Some(from);
    while let Some(directory) = at {
        let candidate = directory.join(".swarm/config.json");
        assert!(
            !candidate.is_file(),
            "this case is about a shell with NO settings file, and {} is one",
            candidate.display()
        );
        at = directory.parent();
    }
}

/// Reads one HTTP request off the socket and answers it, returning the body it carried.
fn one_request(listener: &TcpListener, answer: &str) -> Json {
    let (stream, _) = listener.accept().expect("the binary connects");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout");
    let mut reader = BufReader::new(stream);

    let mut length = 0usize;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).expect("a header line");
        if read == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        let lowered = line.to_ascii_lowercase();
        if let Some(value) = lowered.strip_prefix("content-length:") {
            length = value.trim().parse().expect("a content length");
        }
    }

    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).expect("the whole body");

    let mut stream: TcpStream = reader.into_inner();
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: \
         close\r\n\r\n{answer}",
        answer.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();

    serde_json::from_slice(&body).expect("the binary sends JSON")
}

/// Runs `swarm` with these arguments, from a directory with no settings, and hands back the one
/// request body it put on the wire.
fn what_the_binary_sends(name: &str, answer: &'static str, arguments: &[&str]) -> Json {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let at: SocketAddr = listener.local_addr().expect("its address");
    let (sending, sent) = mpsc::channel();
    let listening = std::thread::spawn(move || {
        let body = one_request(&listener, answer);
        sending.send(body).expect("the case is still waiting");
    });

    let run_in = a_directory_with_no_settings(name);
    no_ancestor_carries_settings(&run_in);

    let mut command = Command::new(env!("CARGO_BIN_EXE_swarm"));
    command
        .current_dir(&run_in)
        .arg("--url")
        .arg(format!("http://{at}"))
        .arg("--swarm")
        .arg("a-swarm")
        .args(arguments);
    let finished = command.output().expect("the binary runs");

    let body = sent
        .recv_timeout(Duration::from_secs(30))
        .unwrap_or_else(|why| {
            panic!(
                "the binary sent no request: {why}. stdout: {}; stderr: {}",
                String::from_utf8_lossy(&finished.stdout),
                String::from_utf8_lossy(&finished.stderr)
            )
        });
    listening.join().expect("the listener finishes");
    body
}

/// The command door, reached by a person at a shell.
///
/// `swarm --swarm <slug> do <command>` is the verb the CLI's own module doc calls the one that
/// "reaches everything else by name" — fifty-three commands, including every
/// `swarm.manager.*` lifecycle command an operator drives a swarm with.
#[test]
fn a_shell_with_no_settings_file_does_not_issue_as_the_coordinator() {
    let body = what_the_binary_sends(
        "no-settings-do",
        r#"{"instance":null,"events":[]}"#,
        &["do", "swarm.manager.PauseSwarm", "--input", "{}"],
    );

    assert_eq!(
        body.get("agent"),
        None,
        "a shell with no settings file named no agent, so the request must name none and the \
         runtime must record `operator`. It carries {} instead, which is a real member of every \
         swarm this runtime runs — so an operator's hand at the CLI is recorded as the \
         coordinator's own command, and `swarm-check`'s unattended condition reports `no \
         operator input in the window` over it. Whole body: {body}",
        body.get("agent")
            .map_or("nothing".to_owned(), Json::to_string)
    );
}

/// The mail door, which this unit newly made name its issuer, from the same shell.
#[test]
fn a_shell_with_no_settings_file_does_not_post_mail_as_the_coordinator() {
    let body = what_the_binary_sends(
        "no-settings-mail",
        r#"{"broadcast_id":null,"messages":[]}"#,
        &["send", "--to", "worker", "--subject", "s", "--body", "b"],
    );

    assert_eq!(
        body.get("agent"),
        None,
        "`Mail::agent`'s doc says \"Absent is an operator, exactly as at the command door\". From \
         a shell with no settings file it is not absent. Whole body: {body}"
    );
}
