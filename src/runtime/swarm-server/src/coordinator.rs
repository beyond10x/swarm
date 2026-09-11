//! Running the coordinator, and turning what it says into a verdict.
//!
//! The loop is `[loop] -> [coordinator] -> [goal]`. The trigger drives the first arrow and the goal
//! lifecycle closes the third; this is the middle one, and without it a goal reaches `Pursuing` and
//! stays there, because nothing else in the system is allowed to say whether a goal is met.
//!
//! The coordinator is a PROGRAM, named by the swarm's config. It is handed the goal on stdin and
//! answers on stdout:
//!
//! ```text
//!   in   {"goal": "...", "iterations": 3, "swarm": "demo"}
//!   out  {"reached": false, "note": "still three tests short"}
//! ```
//!
//! That shape is the whole contract, and it is deliberately small enough that a shell script
//! satisfies it. A `metaharness`-launched agent is one such program — per the 2026-09-11 spike it
//! reaches this system through `tool_surface: Owned` and `allow_program`, because a plugin-supplied
//! MCP server is never discovered under `--strict-mcp-config`.
//!
//! What this must never do is decide the verdict itself. A runtime that answered "reached" on a
//! coordinator's behalf — on a timeout, on a crash, on a budget — would be reporting work nobody
//! did. Every failure here leaves the goal in `Pursuing`, where a person can see it waiting.

use std::process::Stdio;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::swarm::Swarm;

/// What the coordinator is asked.
#[derive(Debug, Serialize)]
struct Asked<'a> {
    /// The goal, in the words it was set in.
    goal: &'a str,
    /// Which turn of the loop this is.
    iterations: u64,
    /// Which swarm it belongs to.
    swarm: &'a str,
}

/// What the coordinator answers.
#[derive(Debug, Deserialize)]
struct Verdict {
    /// Whether the goal is met. The one thing only the coordinator may say.
    reached: bool,
    /// Why, for a person reading the log. Not acted on.
    #[serde(default)]
    note: Option<String>,
}

/// Why a turn could not be finished.
#[derive(Debug)]
pub enum Unfinished {
    /// No coordinator is configured. The goal waits, which is the honest state.
    NoCoordinator,
    /// The program could not be started or did not finish.
    Unrunnable(String),
    /// It ran, and what it said was not a verdict.
    Unreadable { output: String, why: String },
    /// It gave a verdict, and reporting it was refused.
    Unreportable(String),
}

impl std::fmt::Display for Unfinished {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCoordinator => f.write_str(
                "no coordinator is configured, so nothing can say whether the goal is met",
            ),
            Self::Unrunnable(why) => write!(f, "the coordinator could not be run: {why}"),
            Self::Unreadable { output, why } => {
                write!(
                    f,
                    "the coordinator did not answer with a verdict ({why}): {output}"
                )
            }
            Self::Unreportable(why) => write!(f, "the verdict could not be reported: {why}"),
        }
    }
}

/// The program a swarm runs as its coordinator.
///
/// Read from the environment for now rather than from the swarm's `Config` record. That is a
/// shortcut and it is worth naming: `swarm.config.HarnessLaunch` already declares `binary`, `args`,
/// `tool_surface` and `allow_program`, and this should read them once a swarm reliably has an
/// activated config. Until then one setting keeps a demonstration honest rather than inventing a
/// config nobody drafted.
pub fn configured() -> Option<Vec<String>> {
    let raw = std::env::var("SWARM_COORDINATOR").ok()?;
    let parts: Vec<String> = raw.split_whitespace().map(ToOwned::to_owned).collect();
    (!parts.is_empty()).then_some(parts)
}

/// Asks the coordinator about one goal and reports what it says.
///
/// The goal must already be in `Pursuing`: this answers a turn that the loop started, and a verdict
/// on a goal nobody is pursuing is refused by the specification's own `wrong-state` branch.
pub async fn take_a_turn(
    swarm: &Arc<Swarm>,
    goal_id: &str,
    goal: &str,
    iterations: u64,
) -> Result<bool, Unfinished> {
    let program = configured().ok_or(Unfinished::NoCoordinator)?;

    let asked = serde_json::to_string(&Asked {
        goal,
        iterations,
        swarm: swarm.slug(),
    })
    .map_err(|why| Unfinished::Unrunnable(why.to_string()))?;

    let mut child = Command::new(&program[0])
        .args(&program[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| Unfinished::Unrunnable(format!("{}: {why}", program[0])))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(asked.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }

    let done = child
        .wait_with_output()
        .await
        .map_err(|why| Unfinished::Unrunnable(why.to_string()))?;

    let output = String::from_utf8_lossy(&done.stdout).trim().to_owned();
    if !done.status.success() {
        // A coordinator that failed has not reported anything, and must not be read as "not yet":
        // that would be this runtime answering on its behalf.
        return Err(Unfinished::Unrunnable(format!(
            "exited {}: {}",
            done.status,
            String::from_utf8_lossy(&done.stderr).trim()
        )));
    }

    let verdict: Verdict = serde_json::from_str(&output).map_err(|why| Unfinished::Unreadable {
        output: output.clone(),
        why: why.to_string(),
    })?;

    if let Some(note) = &verdict.note {
        tracing::info!(swarm = %swarm.slug(), goal = %goal_id, reached = verdict.reached, note = %note,
                       "the coordinator reported");
    }

    let input = json!({ "goal_id": goal_id, "reached": verdict.reached })
        .as_object()
        .cloned()
        .expect("an object");

    swarm
        .issue(
            Some("swarm.goal.Coordinator"),
            "swarm.goal.Evaluate",
            input,
            &format!("verdict:{goal_id}:{iterations}"),
        )
        .await
        .map_err(|why| Unfinished::Unreportable(why.to_string()))?;

    Ok(verdict.reached)
}
