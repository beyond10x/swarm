use rusqlite::{Connection, params};
use serde_json::Value;
use std::{fs, path::Path};

pub fn corpus() -> Value {
    serde_json::from_str(include_str!("../fixtures/corpus.json")).unwrap()
}

pub fn materialize(input: &Value, root: &Path) {
    if !input["exists"].as_bool().unwrap() {
        return;
    }
    fs::create_dir_all(root).unwrap();
    for dir in input["directories"].as_array().unwrap() {
        fs::create_dir_all(root.join(dir.as_str().unwrap())).unwrap();
    }
    if let Some(events) = input["events"].as_array() {
        let db = Connection::open(root.join("eventlog.sqlite3")).unwrap();
        db.execute_batch("CREATE TABLE swarm_events(global_seq INTEGER, event_name TEXT, actor TEXT, subject TEXT, occurred_at TEXT, data TEXT);").unwrap();
        for event in events {
            db.execute(
                "INSERT INTO swarm_events VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    event["seq"].as_i64(),
                    event["name"].as_str(),
                    event["actor"].as_str(),
                    event["issuer"].as_str(),
                    event["at"].as_str(),
                    event["data"]
                        .to_string()
                        .replace("$ROOT", &root.to_string_lossy())
                ],
            )
            .unwrap();
        }
    }
    for (name, contents) in input["files"].as_object().unwrap() {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            contents
                .as_str()
                .unwrap()
                .replace("$ROOT", &root.to_string_lossy()),
        )
        .unwrap();
    }
    if let Some(rows) = input["raw_events"].as_array() {
        let db = Connection::open(root.join("eventlog.sqlite3")).unwrap();
        db.execute_batch("CREATE TABLE swarm_events(global_seq INTEGER, event_name TEXT, actor TEXT, subject TEXT, occurred_at TEXT, data TEXT);").unwrap();
        for row in rows {
            db.execute(
                "INSERT INTO swarm_events VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    row[0].as_i64(),
                    row[1].as_str(),
                    row[2].as_str(),
                    row[3].as_str(),
                    row[4].as_str(),
                    row[5]
                        .as_str()
                        .map(|s| s.replace("$ROOT", &root.to_string_lossy()))
                ],
            )
            .unwrap();
        }
    }
    if input["corrupt_database"] == true {
        fs::write(root.join("eventlog.sqlite3"), "not a sqlite database").unwrap();
    }
}
