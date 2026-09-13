//! Read historical evidence once. Readers never create a database or retry with write access.
use crate::evidence::Value;
use regex::Regex;
use rusqlite::{Connection, OpenFlags};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub const SPAWNED: &str = "swarm.agent.AgentSpawned";
pub const POSTED: &str = "swarm.agent.AssignmentPosted";
pub const RECORDED: &str = "swarm.agent.AssignmentRecorded";
pub const TAKEN: &str = "swarm.agent.AssignmentTaken";
pub const STARTED: &str = "swarm.manager.SwarmStarted";
pub const RUNTIME: &str = "runtime";
pub const OPERATOR: &str = "operator";
pub const AGENT: &str = "agent:";
pub const UNNAMEABLE_AGENT: &str = "agent:?";
pub const FINISHED: &[&str] = &[
    "swarm.agent.AssignmentDone",
    "swarm.agent.GateGreen",
    "swarm.agent.FinishAssignment",
];
pub const VERDICT: &[&str] = &["swarm.goal.GoalReached", "swarm.goal.GoalNotReached"];

#[derive(Debug, Clone)]
pub struct Event {
    pub seq: i64,
    pub name: String,
    pub actor: String,
    pub issuer: String,
    pub data: Value,
}
impl Event {
    pub fn issued_by(&self) -> Option<&str> {
        let said = self.issuer.as_str();
        (said == RUNTIME || said == OPERATOR || said.starts_with(AGENT)).then_some(said)
    }
}
#[derive(Debug)]
pub struct Turn {
    pub path: PathBuf,
    pub turn: Option<i64>,
    pub goal: Option<String>,
    pub rest: Option<String>,
    pub records: Vec<Value>,
    pub read_error: Option<String>,
}
impl Turn {
    pub fn name(&self) -> String {
        crate::text::Text::scalar(&self.path.file_name().unwrap_or_default().to_string_lossy())
            .encoded()
            .to_owned()
    }
}
#[derive(Default, Debug)]
pub struct Records {
    pub root: PathBuf,
    pub events: Vec<Event>,
    pub log_error: Option<String>,
    pub turns: Vec<Turn>,
    pub spend: Vec<Value>,
    pub spend_error: Option<String>,
    pub capped: Vec<Value>,
    pub capped_error: Option<String>,
}
impl Records {
    pub fn named<'a>(&'a self, names: &'a [&str]) -> impl Iterator<Item = &'a Event> {
        self.events
            .iter()
            .filter(move |e| names.contains(&e.name.as_str()))
    }
    // Insertion order is meaningful in the Python reports; a later spawn replaces its role.
    pub fn spawned(&self) -> Vec<(&str, &Value)> {
        let mut out: Vec<(&str, &Value)> = vec![];
        for e in self.named(&[SPAWNED]) {
            if let Some(agent) = nonempty(&e.data["agent_id"]) {
                if let Some(entry) = out.iter_mut().find(|(id, _)| *id == agent) {
                    entry.1 = &e.data["role"];
                } else {
                    out.push((agent, &e.data["role"]));
                }
            }
        }
        out
    }
    pub fn knows(&self, agent: &str) -> bool {
        self.spawned().iter().any(|(id, _)| *id == agent)
    }
    pub fn agent_named_by<'a>(&self, issuer: Option<&'a str>) -> Option<&'a str> {
        issuer?.strip_prefix(AGENT).filter(|id| self.knows(id))
    }
    pub fn workers(&self) -> Vec<&str> {
        self.spawned()
            .into_iter()
            .filter(|(_, role)| role.as_str() != Some("Coordinator"))
            .map(|(id, _)| id)
            .collect()
    }
    pub fn spend_agents(&self) -> Vec<(&str, usize)> {
        let mut out = vec![];
        for row in &self.spend {
            if let Some(agent) = nonempty(&row["agent"]) {
                increment(&mut out, agent);
            }
        }
        out
    }
}
pub fn increment<T: PartialEq>(items: &mut Vec<(T, usize)>, key: T) {
    if let Some((_, n)) = items.iter_mut().find(|(k, _)| *k == key) {
        *n += 1;
    } else {
        items.push((key, 1));
    }
}
pub fn nonempty(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| !s.is_empty())
}
pub fn py(v: &Value) -> String {
    v.python()
}

pub fn read(root: &Path) -> Records {
    let mut r = read_raw(root);
    // Errors originate in filesystem/SQLite libraries rather than the evidence parser.
    // Escape them once at this boundary, including all early returns from read_raw.
    for error in [&mut r.log_error, &mut r.spend_error, &mut r.capped_error]
        .into_iter()
        .chain(r.turns.iter_mut().map(|turn| &mut turn.read_error))
        .flatten()
    {
        *error = crate::text::Text::scalar(error).encoded().to_owned();
    }
    r
}
fn read_raw(root: &Path) -> Records {
    let mut r = Records {
        root: root.into(),
        ..Records::default()
    };
    if !root.is_dir() {
        let why = format!("there is no directory at {}", root.display());
        r.log_error = Some(why.clone());
        r.spend_error = Some(why);
        return r;
    }
    let log = root.join("eventlog.sqlite3");
    if !log.exists() {
        r.log_error = Some(format!("there is no event log at {}", log.display()));
    } else {
        match read_events(&log) {
            Ok(events) => r.events = events,
            Err(e) => r.log_error = Some(format!("{} could not be read: {e}", log.display())),
        }
    }
    let turns = root.join("turns");
    if !turns.is_dir() {
        r.spend_error = Some(format!(
            "there is no turns directory at {}",
            turns.display()
        ));
    } else {
        match fs::read_dir(&turns) {
            Ok(entries) => {
                let mut paths = vec![];
                for entry in entries {
                    match entry {
                        Ok(entry) => paths.push(entry.path()),
                        Err(e) => {
                            r.spend_error =
                                Some(format!("{} could not be read: {e}", turns.display()));
                            return r;
                        }
                    }
                }
                paths.sort();
                for path in paths {
                    if path.extension().is_some_and(|ext| ext == "jsonl")
                        && !["spend.jsonl", "capped.jsonl"].contains(
                            &path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .as_ref(),
                        )
                    {
                        r.turns.push(read_turn(&path));
                    }
                }
            }
            Err(e) => {
                r.spend_error = Some(format!("{} could not be read: {e}", turns.display()));
                return r;
            }
        }
        (r.spend, r.spend_error, _) = read_jsonl(&turns.join("spend.jsonl"));
        let (capped, error, inaccessible) = read_jsonl(&turns.join("capped.jsonl"));
        r.capped = capped;
        r.capped_error = if inaccessible { error } else { None };
    }
    r
}
fn read_events(path: &Path) -> rusqlite::Result<Vec<Event>> {
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let mut stmt=db.prepare("select global_seq, event_name, actor, subject, occurred_at, data from swarm_events order by global_seq")?;
    stmt.query_map([], |row| {
        let raw: Option<String> = row.get(5)?;
        let data = raw
            .and_then(|raw| Value::parse(&raw).ok())
            .filter(Value::is_object)
            .unwrap_or_else(|| Value::Object(vec![]));
        Ok(Event {
            seq: row.get(0)?,
            name: crate::text::Text::scalar(&row.get::<_, String>(1)?)
                .encoded()
                .to_owned(),
            actor: crate::text::Text::scalar(&row.get::<_, Option<String>>(2)?.unwrap_or_default())
                .encoded()
                .to_owned(),
            issuer: crate::text::Text::scalar(
                &row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            )
            .encoded()
            .to_owned(),
            data,
        })
    })?
    .collect()
}
pub fn read_jsonl(path: &Path) -> (Vec<Value>, Option<String>, bool) {
    if !path.exists() {
        return (
            vec![],
            Some(format!(
                "there is no {} at {}",
                path.file_name().unwrap_or_default().to_string_lossy(),
                path.display()
            )),
            false,
        );
    }
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                vec![],
                Some(format!("{} could not be read: {e}", path.display())),
                true,
            );
        }
    };
    let rows: Vec<_> = String::from_utf8_lossy(&bytes)
        .lines()
        .filter_map(|line| Value::parse(line.trim()).ok())
        .filter(Value::is_object)
        .collect();
    let error = rows
        .is_empty()
        .then(|| format!("{} holds no readable rows", path.display()));
    (rows, error, false)
}
static TURN_FILE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<turn>\d{4})-(?P<attempt>\d{2})-(?:(?P<agent>[A-Za-z0-9_-]+?)-)?(?P<goal>[0-9a-fA-F]{6,})(?:-(?P<rest>.+))?\.jsonl$").unwrap()
});
static LEGACY_TURN_FILE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<turn>\d{4})-(?P<goal>[0-9a-fA-F]{6,})\.jsonl$").unwrap());
fn read_turn(path: &Path) -> Turn {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let caps = TURN_FILE
        .captures(&name)
        .or_else(|| LEGACY_TURN_FILE.captures(&name));
    let field = |name| {
        caps.as_ref()
            .and_then(|c| c.name(name))
            .map(|m| crate::text::Text::scalar(m.as_str()).encoded().to_owned())
    };
    let (records, error, inaccessible) = read_jsonl(path);
    Turn {
        path: path.into(),
        turn: field("turn").and_then(|s| s.parse().ok()),
        goal: field("goal"),
        rest: field("rest"),
        records,
        read_error: if inaccessible { error } else { None },
    }
}
