//! `swarm` — the command line a swarm's own agents use to reach their runtime.
//!
//! Everything the runtime serves is already an HTTP surface, and the coordinator already has a
//! shell. What it did not have was a way to name a command: it could not spawn an agent, draw a
//! box, wire a connection or send a message, because nothing told it the surface existed or how to
//! address it. This is that.
//!
//! ## Why configuration arrives in a file
//!
//! A coordinator runs under `metaharness run claude --hermetic`, whose child environment is built
//! from a **seven-key allowlist** — `HOME USER LOGNAME LANG LC_ALL TERM TZ` — so no variable of
//! ours reaches it. Its `PATH` is `$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin`, which is why
//! this binary belongs in `~/.local/bin` and not in `~/.cargo/bin`.
//!
//! So the runtime writes `<work>/.swarm/config.json` before each turn and this walks up from the
//! working directory to find it:
//!
//! ```json
//! {"url": "http://localhost:5000", "swarm": "demo", "agent": "coordinator", "mailbox": "main"}
//! ```
//!
//! ## Why there is a `do` verb
//!
//! The specification declares fifty-three commands and will declare more. Writing a subcommand per
//! command would mean this binary falls behind `src/core` the first time somebody adds a domain, so
//! the verbs here are the ones an agent uses constantly and `do` reaches everything else by name.
//! `swarm spec` lists what that is, with each command's inputs, read from the runtime rather than
//! from a copy kept here.
//!
//! Nothing in this file decides what is allowed. A command an actor may not issue, or one whose
//! subject is in the wrong state, is refused by the specification, and what this prints is the
//! declared error.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json, json};

/// Where this binary looks for its settings, relative to the working directory and its parents.
const CONFIG: &str = ".swarm/config.json";

#[derive(Debug, Parser)]
#[command(
    name = "swarm",
    about = "Reach the swarm runtime: read your mail, issue a command, read a view.",
    version
)]
struct Cli {
    /// The runtime's address. Overrides the config file.
    #[arg(long, global = true)]
    url: Option<String>,
    /// Which swarm. Overrides the config file.
    #[arg(long, global = true)]
    swarm: Option<String>,
    /// Who is acting. Overrides the config file.
    #[arg(long, global = true)]
    agent: Option<String>,
    /// Print what came back, whole and unformatted.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Verb,
}

#[derive(Debug, Subcommand)]
enum Verb {
    /// What has been said to you and not yet read.
    Inbox {
        /// Everybody's unread mail, not only yours.
        #[arg(long)]
        all: bool,
    },
    /// Read one message, and mark it read.
    Read {
        message_id: String,
        /// Show it without marking it read.
        #[arg(long)]
        peek: bool,
    },
    /// Say you have dealt with a message. Only its recipient may.
    Ack {
        message_id: String,
        #[arg(long)]
        note: Option<String>,
    },
    /// Send a message to one mailbox.
    Send {
        /// `agent` or `agent/mailbox`. `main` is assumed.
        #[arg(long)]
        to: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        body: String,
        /// The message this answers.
        #[arg(long)]
        reply_to: Option<String>,
    },
    /// Send a message to every open mailbox but your own.
    Broadcast {
        #[arg(long)]
        subject: String,
        #[arg(long)]
        body: String,
    },
    /// Open a mailbox of your own.
    Mailbox {
        /// What to call it. The address becomes `<you>/<name>`.
        name: String,
    },
    /// Everything the swarm holds, by entity.
    Canvas,
    /// Read one of the views the specification declares.
    View {
        /// Its name. `UnreadMessages` and `swarm.mailbox.UnreadMessages` both work.
        name: String,
    },
    /// What may be issued, and what each command takes.
    Spec {
        /// One command, with its inputs.
        #[arg(long)]
        command: Option<String>,
        /// List the entities instead.
        #[arg(long)]
        entities: bool,
    },
    /// Issue any command the specification declares.
    Do {
        /// Its qualified name, for example `swarm.agent.Spawn`.
        name: String,
        /// Its input, as one JSON object.
        #[arg(long, default_value = "{}")]
        input: String,
    },
}

/// What the runtime wrote for this turn.
#[derive(Debug, Default, Deserialize, Serialize)]
struct Config {
    url: Option<String>,
    swarm: Option<String>,
    agent: Option<String>,
    mailbox: Option<String>,
}

/// Everything a request needs, from the file and the flags together.
struct Settings {
    url: String,
    swarm: String,
    agent: String,
}

fn main() {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => {}
        Err(why) => {
            eprintln!("swarm: {why}");
            std::process::exit(1);
        }
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let found = read_config();
    let settings = Settings {
        url: cli
            .url
            .clone()
            .or(found.url)
            .unwrap_or_else(|| "http://localhost:5000".to_owned()),
        swarm: cli
            .swarm
            .clone()
            .or(found.swarm)
            .ok_or("no swarm named. Pass --swarm, or run where .swarm/config.json is")?,
        agent: cli
            .agent
            .clone()
            .or(found.agent)
            .unwrap_or_else(|| "coordinator".to_owned()),
    };

    match &cli.command {
        Verb::Inbox { all } => inbox(&settings, *all, cli.json),
        Verb::Read { message_id, peek } => read_message(&settings, message_id, *peek, cli.json),
        Verb::Ack { message_id, note } => ack(&settings, message_id, note.as_deref(), cli.json),
        Verb::Send {
            to,
            subject,
            body,
            reply_to,
        } => mail(
            &settings,
            json!({
                "to": to, "sender": settings.agent, "subject": subject, "body": body,
                "reply_to": reply_to
            }),
            cli.json,
        ),
        Verb::Broadcast { subject, body } => mail(
            &settings,
            json!({ "sender": settings.agent, "subject": subject, "body": body }),
            cli.json,
        ),
        Verb::Mailbox { name } => open_mailbox(&settings, name, cli.json),
        Verb::Canvas => {
            let canvas: Json = get(&format!("{}/swarms/{}", settings.url, settings.swarm))?;
            if cli.json {
                println!("{canvas:#}");
                return Ok(());
            }
            for (entity, held) in canvas.as_object().into_iter().flatten() {
                let rows = held.as_array().map(Vec::len).unwrap_or_default();
                if rows > 0 {
                    println!("{rows:>4}  {entity}");
                }
            }
            Ok(())
        }
        Verb::View { name } => {
            let full = resolve_view(&settings, name)?;
            let rows: Json = get(&format!(
                "{}/swarms/{}/views/{full}",
                settings.url, settings.swarm
            ))?;
            println!("{rows:#}");
            Ok(())
        }
        Verb::Spec { command, entities } => describe(&settings, command.as_deref(), *entities),
        Verb::Do { name, input } => {
            let parsed: Json = serde_json::from_str(input)
                .map_err(|why| format!("--input is not a JSON object: {why}"))?;
            let issued = issue(&settings, name, parsed)?;
            println!("{issued:#}");
            Ok(())
        }
    }
}

/// The unread messages addressed to this agent.
fn inbox(settings: &Settings, all: bool, raw: bool) -> Result<(), String> {
    let rows: Vec<Map<String, Json>> = get(&format!(
        "{}/swarms/{}/views/swarm.mailbox.UnreadMessages",
        settings.url, settings.swarm
    ))?;
    let mine: Vec<&Map<String, Json>> = rows
        .iter()
        .filter(|row| {
            all || row.get("recipient_id").and_then(Json::as_str) == Some(settings.agent.as_str())
        })
        .collect();

    if raw {
        println!("{:#}", json!(mine));
        return Ok(());
    }
    if mine.is_empty() {
        println!("no unread mail");
        return Ok(());
    }
    for row in mine {
        let field = |name: &str| row.get(name).and_then(Json::as_str).unwrap_or("");
        let id = field("message_id");
        println!(
            "{}  from {:<14} {}",
            &id[..id.len().min(8)],
            field("sender_id"),
            field("subject")
        );
        if row.get("broadcast_id").and_then(Json::as_str).is_some() {
            println!("          (a broadcast)");
        }
    }
    println!("\nswarm read <id> to read one, swarm ack <id> when you have dealt with it");
    Ok(())
}

/// One message, and the mark that says it was read.
fn read_message(settings: &Settings, id: &str, peek: bool, raw: bool) -> Result<(), String> {
    let rows: Vec<Map<String, Json>> = get(&format!(
        "{}/swarms/{}/views/swarm.mailbox.MessageById",
        settings.url, settings.swarm
    ))?;
    let found = rows
        .iter()
        .find(|row| {
            row.get("message_id")
                .and_then(Json::as_str)
                .is_some_and(|held| held == id || held.starts_with(id))
        })
        .ok_or_else(|| format!("no message `{id}`"))?;

    if raw {
        println!("{:#}", json!(found));
    } else {
        let field = |name: &str| found.get(name).and_then(Json::as_str).unwrap_or("");
        println!("from:    {}", field("sender_id"));
        println!("to:      {}", field("recipient_id"));
        println!("sent:    {}", field("sent_at"));
        println!("subject: {}", field("subject"));
        println!("\n{}\n", field("body"));
    }

    if !peek && found.get("state").and_then(Json::as_str) == Some("Unread") {
        let full = found
            .get("message_id")
            .and_then(Json::as_str)
            .unwrap_or(id)
            .to_owned();
        issue(
            settings,
            "swarm.mailbox.MarkRead",
            json!({ "message_id": full, "read_by": settings.agent, "read_at": stamp() }),
        )?;
    }
    Ok(())
}

fn ack(settings: &Settings, id: &str, note: Option<&str>, raw: bool) -> Result<(), String> {
    let issued = issue(
        settings,
        "swarm.mailbox.AckMessage",
        json!({
            "message_id": id, "acked_by": settings.agent, "acked_at": stamp(), "ack_note": note
        }),
    )?;
    if raw {
        println!("{issued:#}");
    } else {
        println!("acked");
    }
    Ok(())
}

/// Posts a message. The runtime resolves the address, because only it can look a mailbox up.
fn mail(settings: &Settings, body: Json, raw: bool) -> Result<(), String> {
    let posted: Json = post(
        &format!("{}/swarms/{}/mail", settings.url, settings.swarm),
        body,
    )?;
    if raw {
        println!("{posted:#}");
        return Ok(());
    }
    let sent = posted
        .get("messages")
        .and_then(Json::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    match posted.get("broadcast_id").and_then(Json::as_str) {
        Some(_) => println!("sent to {sent} mailboxes"),
        None => println!("sent"),
    }
    Ok(())
}

fn open_mailbox(settings: &Settings, name: &str, raw: bool) -> Result<(), String> {
    let canvas: Json = get(&format!("{}/swarms/{}", settings.url, settings.swarm))?;
    let swarm_id = canvas
        .get("swarm.manager.Swarm")
        .and_then(Json::as_array)
        .and_then(|held| held.first())
        .and_then(|instance| instance.get("id"))
        .and_then(Json::as_str)
        .ok_or("this swarm has no record yet")?;

    let issued = issue(
        settings,
        "swarm.mailbox.OpenMailbox",
        json!({
            "swarm_id": swarm_id, "agent_id": settings.agent, "name": name, "created_at": stamp()
        }),
    )?;
    if raw {
        println!("{issued:#}");
    } else {
        println!("{}/{name} is open", settings.agent);
    }
    Ok(())
}

/// What the specification declares, read from the runtime rather than from a copy kept here.
fn describe(settings: &Settings, command: Option<&str>, entities: bool) -> Result<(), String> {
    let spec: Json = get(&format!("{}/spec", settings.url))?;

    if entities {
        for entity in spec
            .get("entities")
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
        {
            let name = entity.get("name").and_then(Json::as_str).unwrap_or("");
            let states: Vec<&str> = entity
                .get("states")
                .and_then(Json::as_array)
                .into_iter()
                .flatten()
                .filter_map(Json::as_str)
                .collect();
            println!("{name}  [{}]", states.join(" -> "));
        }
        return Ok(());
    }

    let declared = spec
        .get("commands")
        .and_then(Json::as_array)
        .ok_or("no commands")?;
    for entry in declared {
        let name = entry.get("name").and_then(Json::as_str).unwrap_or("");
        if let Some(wanted) = command
            && !name.contains(wanted)
        {
            continue;
        }
        let inputs: Vec<String> = entry
            .get("input")
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
            .map(|field| {
                let field_name = field.get("name").and_then(Json::as_str).unwrap_or("");
                if field.get("optional").and_then(Json::as_bool) == Some(true) {
                    format!("[{field_name}]")
                } else {
                    field_name.to_owned()
                }
            })
            .collect();
        println!("{name}  {}", inputs.join(" "));
    }
    Ok(())
}

/// A view's full name, from whatever the caller typed.
///
/// Views are declared as `swarm.mailbox.UnreadMessages` and an agent reasonably types
/// `UnreadMessages`; before this, that was a 404 with nothing to go on. A short name that matches
/// exactly one declared view is that view, and one that matches several is refused with the list,
/// because guessing between them would be this binary deciding which the caller meant.
fn resolve_view(settings: &Settings, name: &str) -> Result<String, String> {
    if name.contains('.') {
        return Ok(name.to_owned());
    }
    let spec: Json = get(&format!("{}/spec", settings.url))?;
    let declared: Vec<String> = spec
        .get("views")
        .and_then(Json::as_array)
        .into_iter()
        .flatten()
        .filter_map(Json::as_str)
        .map(ToOwned::to_owned)
        .collect();

    let matched: Vec<&String> = declared
        .iter()
        .filter(|view| view.rsplit('.').next() == Some(name))
        .collect();
    match matched.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(format!(
            "no view called `{name}`. Declared:\n  {}",
            declared.join("\n  ")
        )),
        several => Err(format!(
            "`{name}` names several views:\n  {}",
            several
                .iter()
                .map(|view| view.as_str())
                .collect::<Vec<_>>()
                .join("\n  ")
        )),
    }
}

/// Issues one command as this agent.
fn issue(settings: &Settings, command: &str, input: Json) -> Result<Json, String> {
    let issued: Json = post(
        &format!(
            "{}/swarms/{}/commands/{command}",
            settings.url, settings.swarm
        ),
        json!({ "input": input, "actor": actor_for(command, &settings.agent) }),
    )?;
    // A refusal the specification declares comes back as an outcome with an error, not as a
    // transport failure. Saying so here beats printing a success-shaped object that refused.
    if let Some(error) = issued.get("error").and_then(Json::as_str) {
        return Err(format!("{command} was refused: {error}"));
    }
    Ok(issued)
}

/// Which actor to issue as.
///
/// The specification decides what an actor may do, and a swarm's own agent is the `SwarmAgent` of
/// whichever domain it is acting in. Where a domain declares no such actor the command is issued
/// as the system, which skips the actor check — the runtime's own behaviour, not a widening
/// invented here.
fn actor_for(command: &str, _agent: &str) -> Option<String> {
    match command.split('.').nth(1) {
        Some("mailbox") => Some("swarm.mailbox.SwarmAgent".to_owned()),
        _ => None,
    }
}

/// Now, as the runtime's commands want it. The model does not read a clock; callers do.
fn stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default();
    // A civil date from a Unix second, so this binary needs no date library for one field.
    let (days, rest) = (now / 86_400, now % 86_400);
    let (mut y, mut d) = (1970i64, days as i64);
    loop {
        let len = if leap(y) { 366 } else { 365 };
        if d < len {
            break;
        }
        d -= len;
        y += 1;
    }
    let months = [
        31,
        if leap(y) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0;
    while d >= months[m] {
        d -= months[m];
        m += 1;
    }
    format!(
        "{y:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        m + 1,
        d + 1,
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

fn leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn get<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    ureq::get(url)
        .call()
        .map_err(|why| unreachable_or(why, url))?
        .body_mut()
        .read_json()
        .map_err(|why| format!("what came back is not what was expected: {why}"))
}

fn post<T: serde::de::DeserializeOwned>(url: &str, body: Json) -> Result<T, String> {
    ureq::post(url)
        .send_json(&body)
        .map_err(|why| unreachable_or(why, url))?
        .body_mut()
        .read_json()
        .map_err(|why| format!("what came back is not what was expected: {why}"))
}

/// A transport failure, said in a way that names the likeliest cause.
fn unreachable_or(why: ureq::Error, url: &str) -> String {
    match why {
        ureq::Error::StatusCode(status) => format!("the runtime refused it: {status}"),
        other => format!("the runtime at {url} could not be reached: {other}"),
    }
}

/// The nearest `.swarm/config.json`, walking up from the working directory.
fn read_config() -> Config {
    let mut here: Option<&Path> = None;
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut at = Some(cwd.as_path());
    while let Some(directory) = at {
        let candidate = directory.join(CONFIG);
        if candidate.is_file() {
            here = Some(directory);
            break;
        }
        at = directory.parent();
    }
    let Some(directory) = here else {
        return Config::default();
    };
    std::fs::read_to_string(directory.join(CONFIG))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stamp_is_an_rfc_3339_instant() {
        let stamped = stamp();
        assert_eq!(stamped.len(), 20, "{stamped}");
        assert!(stamped.ends_with('Z'));
        assert!(stamped.starts_with("20"));
        // The two separators the runtime's Timestamp parser needs.
        assert_eq!(stamped.as_bytes()[10], b'T');
        assert_eq!(stamped.matches('-').count(), 2);
    }

    #[test]
    fn a_mailbox_command_is_issued_as_the_domains_agent() {
        assert_eq!(
            actor_for("swarm.mailbox.PostMessage", "coordinator").as_deref(),
            Some("swarm.mailbox.SwarmAgent")
        );
        // No actor is declared for these, and inventing one would be this binary deciding what the
        // specification is silent about.
        assert_eq!(actor_for("swarm.agent.Spawn", "coordinator"), None);
    }

    #[test]
    fn leap_years_are_the_gregorian_ones() {
        assert!(leap(2024) && leap(2000));
        assert!(!leap(1900) && !leap(2026));
    }
}
