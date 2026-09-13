use crate::{Clause, records::*};
use regex::Regex;
use serde_json::Value;
use std::{collections::BTreeSet, sync::LazyLock};
fn listed(items: impl IntoIterator<Item = impl ToString>) -> String {
    listed_limit(items, 6)
}
fn listed_limit(items: impl IntoIterator<Item = impl ToString>, limit: usize) -> String {
    let all: Vec<_> = items.into_iter().map(|i| i.to_string()).collect();
    let mut shown = all
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if all.len() > limit {
        shown.push_str(&format!(", and {} more", all.len() - limit));
    }
    shown
}
fn or_none(s: String) -> String {
    if s.is_empty() { "none".into() } else { s }
}
pub fn one(r: &Records) -> Clause {
    let spawned = r.spawned();
    let workers = r.workers();
    let found = if let Some(e) = &r.log_error {
        e.clone()
    } else if spawned.is_empty() {
        format!("no `{SPAWNED}` at all, in {} event(s)", r.events.len())
    } else if workers.is_empty() {
        format!(
            "{} agent(s) spawned, every one of them a Coordinator: {}",
            spawned.len(),
            listed(
                spawned
                    .iter()
                    .map(|(agent, role)| format!("`{agent}` ({})", py(role)))
            )
        )
    } else {
        format!(
            "{} of {} spawned agent(s): {}",
            workers.len(),
            spawned.len(),
            listed(
                spawned
                    .iter()
                    .filter(|(id, _)| workers.contains(id))
                    .map(|(agent, role)| format!("`{agent}` ({})", py(role)))
            )
        )
    };
    Clause::new(1, !workers.is_empty(), found)
}
struct Handover<'a> {
    post: &'a Event,
    take: &'a Event,
    agent: &'a str,
    assignment: Option<&'a str>,
}
impl Handover<'_> {
    fn describe(&self) -> String {
        format!(
            "posted to `{}` at seq {} and taken by it at seq {}, {}",
            self.agent,
            self.post.seq,
            self.take.seq,
            self.assignment
                .map(|a| format!("assignment {a}"))
                .unwrap_or_else(|| "an assignment the take does not name (`ref` absent)".into())
        )
    }
}
fn handovers(r: &Records) -> Vec<Handover<'_>> {
    let workers = r.workers();
    let mut out = vec![];
    for take in r.named(&[TAKEN]) {
        let Some(agent) = take.data["agent_id"]
            .as_str()
            .filter(|a| workers.contains(a))
        else {
            continue;
        };
        let assignment = nonempty(&take.data["ref"]);
        let recorded = assignment.and_then(|a| {
            r.named(&[RECORDED]).find(|e| {
                e.data["assignment_id"].as_str() == Some(a)
                    && e.data["agent_id"].as_str() == Some(agent)
                    && e.seq < take.seq
            })
        });
        let before: Vec<_> = r
            .named(&[POSTED])
            .filter(|e| e.data["agent_id"].as_str() == Some(agent) && e.seq < take.seq)
            .collect();
        let post = recorded
            .and_then(|rec| before.iter().rev().find(|p| p.seq < rec.seq).copied())
            .or_else(|| before.last().copied());
        if let Some(post) = post {
            out.push(Handover {
                post,
                take,
                agent,
                assignment,
            });
        }
    }
    out
}
fn who_took_it(h: &Handover<'_>, r: &Records) -> (bool, String) {
    let Some(issuer) = h.take.issued_by() else {
        return (
            true,
            "and this log does not say what issued that taking".into(),
        );
    };
    if issuer == OPERATOR {
        return (
            false,
            format!("and an `{OPERATOR}` issued that taking — a hand, not the member"),
        );
    }
    if issuer == RUNTIME {
        return (
            true,
            format!("and the `{RUNTIME}` issued that taking, on the member's behalf"),
        );
    }
    match r.agent_named_by(Some(issuer)) {
        Some(named) if named == h.agent => {
            (true, format!("and `{}` issued that taking itself", h.agent))
        }
        None => (
            false,
            format!(
                "and the taking was issued by `{issuer}`, which this log spawned no agent for — a mark for a name the envelope cannot hold, or a claim with nothing behind it. Either way it cannot be shown to be the member"
            ),
        ),
        Some(named) => (
            false,
            format!(
                "and `{named}` issued that taking, not `{}` — one agent taking another's work in its name is exactly what this clause could not see before",
                h.agent
            ),
        ),
    }
}
fn coping(r: &Records) -> String {
    format!(
        "{} of {} event(s) in this log name no issuer at all, so it was written before commands recorded who issued them (`store.rs` wrote the actor into both `subject` and `actor`) — this part is not checked here, and is reported rather than passed over",
        r.events.iter().filter(|e| e.issued_by().is_none()).count(),
        r.events.len()
    )
}
pub fn two(r: &Records) -> Clause {
    let posts: Vec<_> = r.named(&[POSTED]).collect();
    let takes: Vec<_> = r.named(&[TAKEN]).collect();
    let pairs = handovers(r);
    let mut met = !pairs.is_empty();
    let found = if let Some(e) = &r.log_error {
        e.clone()
    } else {
        let mut parts = vec![
            format!(
                "{} `{POSTED}`{}",
                posts.len(),
                if posts.is_empty() {
                    String::new()
                } else {
                    format!(
                        " (to {})",
                        listed(
                            posts
                                .iter()
                                .map(|p| format!("`{}`", py(&p.data["agent_id"])))
                        )
                    )
                }
            ),
            format!(
                "{} `{TAKEN}`{}",
                takes.len(),
                if takes.is_empty() {
                    " — it has fired zero times in this repository's history".into()
                } else {
                    format!(
                        " (by {})",
                        listed(
                            takes
                                .iter()
                                .map(|t| format!("`{}`", py(&t.data["agent_id"])))
                        )
                    )
                }
            ),
        ];
        if let Some(last) = pairs.last() {
            let (issued, why) = who_took_it(last, r);
            met &= issued;
            parts.push(format!("{}, {why}", last.describe()));
            if pairs.len() > 1 {
                parts.push(format!(
                    "{} handover(s) in all; this is the one in flight last",
                    pairs.len()
                ));
            }
            for h in pairs.iter().take(pairs.len() - 1) {
                let (ok, reason) = who_took_it(h, r);
                if !ok {
                    met = false;
                    parts.push(format!(
                        "an EARLIER handover is not clean: taken at seq {}, {reason}",
                        h.take.seq
                    ));
                }
            }
            if pairs.iter().any(|h| h.take.issued_by().is_none()) {
                parts.push(coping(r));
            }
        } else if !posts.is_empty() && !takes.is_empty() {
            parts.push(
                "no posting is answered by a later taking from the same non-Coordinator agent"
                    .into(),
            );
        }
        parts.join("; ")
    };
    Clause::new(2, met, found)
}
fn agent_of<'a>(t: &'a Turn, r: &'a Records) -> (Option<&'a str>, &'static str) {
    for row in &t.records {
        if let Some(agent) = row["agent"].as_str().filter(|a| r.knows(a)) {
            return (Some(agent), "named in the record");
        }
    }
    if let Some(rest) = t.rest.as_deref().filter(|a| r.knows(a)) {
        return (Some(rest), "named in the file name");
    }
    for row in &t.records {
        if let Some(cwd) = row["cwd"].as_str() {
            for part in cwd.split('/') {
                if r.knows(part) {
                    return (Some(part), "the work directory it ran in");
                }
            }
        }
    }
    if let (Some(turn), Some(goal)) = (t.turn, t.goal.as_deref()) {
        for row in &r.spend {
            if let (Some(g), Some(agent)) = (row["goal"].as_str(), nonempty(&row["agent"]))
                && r.knows(agent)
                && row["iterations"].as_i64() == Some(turn)
                && g.starts_with(goal)
            {
                return (Some(agent), "spend.jsonl");
            }
        }
    }
    (
        None,
        "nothing in the file, its name or the spend record names a spawned agent",
    )
}
fn prefix(s: &str) -> String {
    s.chars().take(8).collect()
}
pub fn three(r: &Records) -> Clause {
    let attributed: Vec<_> = r.turns.iter().map(|t| (t, agent_of(t, r))).collect();
    let agents: BTreeSet<_> = attributed.iter().filter_map(|(_, (a, _))| *a).collect();
    let mut rows_per_key = vec![];
    for row in &r.spend {
        if let (Some(g), Some(t)) = (row["goal"].as_str(), row["iterations"].as_i64()) {
            increment(&mut rows_per_key, (t, prefix(g)));
        }
    }
    rows_per_key.sort_by(|a, b| a.0.cmp(&b.0));
    let mut missing = vec![];
    for ((turn, goal), rows) in rows_per_key {
        let files = r
            .turns
            .iter()
            .filter(|t| {
                t.turn == Some(turn)
                    && t.goal
                        .as_ref()
                        .is_some_and(|g| goal.starts_with(&prefix(g)))
            })
            .count();
        if rows > files {
            missing.push(format!("turn {turn} of goal {goal} has {rows} attempt(s) in spend.jsonl and {files} file(s): {}",if files==0{"no transcript at all"}else{"a record was overwritten"}));
        }
    }
    let met = r.turns.len() >= 2
        && agents.len() >= 2
        && missing.is_empty()
        && r.turns.iter().all(|t| t.read_error.is_none());
    let mut parts = vec![];
    if r.turns.is_empty() {
        parts.push(
            r.spend_error.clone().unwrap_or_else(|| {
                format!("no turn files under {}", r.root.join("turns").display())
            }),
        );
    } else {
        let mut per_agent = vec![];
        let mut nameless = vec![];
        for (turn, (agent, how)) in &attributed {
            if let Some(agent) = agent {
                increment(&mut per_agent, (*agent, *how));
            } else {
                nameless.push(turn.name());
            }
        }
        parts.push(format!(
            "{} turn file(s), {} attributed: {}",
            r.turns.len(),
            r.turns.len() - nameless.len(),
            or_none(listed(per_agent.iter().map(
                |((agent, how), count)| format!("`{agent}` ×{count} ({how})")
            )))
        ));
        parts.push(format!(
            "{} distinct agent(s): {}",
            agents.len(),
            or_none(listed(agents))
        ));
        if !nameless.is_empty() {
            parts.push(format!(
                "{} file(s) no agent is named on: {}",
                nameless.len(),
                listed_limit(nameless, 3)
            ));
        }
        let unreadable: Vec<_> = r
            .turns
            .iter()
            .filter(|t| t.records.is_empty())
            .map(Turn::name)
            .collect();
        if !unreadable.is_empty() {
            parts.push(format!(
                "{} file(s) hold no readable record: {}",
                unreadable.len(),
                listed(unreadable)
            ));
        }
    }
    parts.extend(r.turns.iter().filter_map(|t| t.read_error.clone()));
    parts.extend(missing);
    Clause::new(3, met, parts.join("; "))
}
fn finisher_of<'a>(e: &'a Event, r: &'a Records) -> Option<&'a str> {
    if let Some(agent) = e.data["agent_id"].as_str() {
        return Some(agent);
    }
    let assignment = &e.data["assignment_id"];
    if assignment.is_null() {
        return None;
    }
    r.named(&[RECORDED])
        .find(|rec| rec.data["assignment_id"] == *assignment)
        .and_then(|rec| rec.data["agent_id"].as_str())
}
fn assignment_named_by(e: &Event) -> Option<&str> {
    nonempty(&e.data["assignment_id"]).or_else(|| nonempty(&e.data["ref"]))
}
fn why_not(e: &Event, agent: Option<&str>, latest: Option<&Handover<'_>>) -> Option<String> {
    let Some(latest) = latest else {
        return Some("nothing was taken, so there is nothing for it to be the finish of".into());
    };
    if e.seq <= latest.take.seq {
        return Some(format!(
            "recorded at seq {}, at or before the take at seq {} — a finish cannot precede the work it finished",
            e.seq, latest.take.seq
        ));
    }
    if agent != Some(latest.agent) {
        return Some(format!(
            "{}, and the work was taken by `{}`",
            agent
                .filter(|a| !a.is_empty())
                .map(|a| format!("by `{a}`"))
                .unwrap_or_else(|| "by nobody the record names".into()),
            latest.agent
        ));
    }
    if let (Some(taken), Some(named)) = (latest.assignment, assignment_named_by(e))
        && taken != named
    {
        return Some(format!(
            "for assignment {named}, and the work taken was {taken}"
        ));
    }
    None
}
pub fn four(r: &Records) -> Clause {
    let pairs = handovers(r);
    let latest = pairs.last();
    let mut hits = vec![];
    let mut others = vec![];
    for e in r.named(FINISHED) {
        let agent = finisher_of(e, r);
        match why_not(e, agent, latest) {
            None => hits.push((e, agent)),
            Some(why) => others.push((e, why)),
        }
    }
    let found = if let Some(e) = &r.log_error {
        e.clone()
    } else if !hits.is_empty() {
        let latest = latest.unwrap();
        listed(hits.iter().map(|(e,agent)|{
            let named=assignment_named_by(e);let suffix=if latest.assignment.is_some()&&named==latest.assignment{format!(", both naming {}",named.unwrap())}else{format!(" — but {}, so the agent and the ordering are checked and the assignment is not",if latest.assignment.is_none(){"the take names no assignment (`ref` absent)"}else{"this event names no assignment"})};
            format!("`{}` at seq {} by `{}`, which took the assignment at seq {}{suffix}",e.name,e.seq,agent.unwrap_or("None"),latest.take.seq)
        }))
    } else if !others.is_empty() {
        format!(
            "{} end-of-work event(s), none of them this finish: {}{}",
            others.len(),
            listed(
                others
                    .iter()
                    .map(|(e, why)| format!("`{}` at seq {} {why}", e.name, e.seq))
            ),
            latest
                .map(|h| format!("; the handover of record is {}", h.describe()))
                .unwrap_or_else(|| "; clause 2 matched no handover".into())
        )
    } else {
        format!(
            "neither, in {} event(s); the agent events present are {}",
            r.events.len(),
            or_none(listed(
                r.events
                    .iter()
                    .filter(|e| e.name.starts_with("swarm.agent."))
                    .map(|e| &e.name)
                    .collect::<BTreeSet<_>>()
            ))
        )
    };
    Clause::new(4, !hits.is_empty(), found)
}
struct Refusal {
    file: String,
    decider: Value,
    tool: Value,
    operations: Value,
    reason: Value,
}
fn refusals(r: &Records) -> Vec<Refusal> {
    let mut out = vec![];
    for t in &r.turns {
        for row in &t.records {
            if row["event"] != "tool.decided" {
                continue;
            }
            let decision = &row["decision"];
            let (word, reason) = if decision.is_object() {
                (&decision["decision"], decision["reason"].clone())
            } else {
                (decision, Value::Null)
            };
            if !word.as_str().is_some_and(|w| {
                [
                    "deny", "denied", "refuse", "refused", "block", "blocked", "reject", "rejected",
                ]
                .contains(&w.to_lowercase().as_str())
            }) {
                continue;
            }
            let asked = t
                .records
                .iter()
                .rev()
                .find(|r| r["event"] == "tool.requested" && r["call_id"] == row["call_id"]);
            out.push(Refusal {
                file: t.name(),
                decider: row["decided_by"].clone(),
                tool: asked.map(|r| r["name"].clone()).unwrap_or_default(),
                operations: asked.map(|r| r["operations"].clone()).unwrap_or_default(),
                reason,
            });
        }
    }
    out
}
pub fn five(r: &Records) -> Clause {
    let denied = refusals(r);
    let by_frame: Vec<_> = denied.iter().filter(|d| d.decider == "frame").collect();
    let decided = r
        .turns
        .iter()
        .flat_map(|t| &t.records)
        .filter(|r| r["event"] == "tool.decided")
        .count();
    let claimed: i64 = r
        .turns
        .iter()
        .filter_map(|t| {
            t.records.iter().find(|r| {
                ["session.ended", "session.summary"].contains(&r["event"].as_str().unwrap_or(""))
                    && r["census"].is_object()
            })
        })
        .filter_map(|r| r["census"]["denied"].as_i64())
        .sum();
    let unaccounted = if claimed != denied.len() as i64 {
        format!(
            "; the session censuses claim {claimed} denied call(s) and {} `tool.decided` record(s) name a denier",
            denied.len()
        )
    } else {
        String::new()
    };
    let found = if r.turns.is_empty() {
        r.spend_error
            .clone()
            .unwrap_or_else(|| format!("no turn records under {}", r.root.join("turns").display()))
    } else if let Some(entry) = by_frame.first() {
        format!(
            "{} of {decided} decided call(s) refused by the frame: `{}` ({}) in {}{}",
            by_frame.len(),
            py(&entry.tool),
            listed(entry.operations.as_array().into_iter().flatten().map(py)),
            entry.file,
            if entry.reason.is_null() || entry.reason == "" {
                String::new()
            } else {
                format!(" — {}", py(&entry.reason))
            }
        )
    } else if !denied.is_empty() {
        format!(
            "{} of {decided} decided call(s) denied, none of them by the frame: {}{unaccounted}",
            denied.len(),
            listed(
                denied
                    .iter()
                    .map(|e| format!("`{}` denied `{}`", py(&e.decider), py(&e.tool)))
            )
        )
    } else {
        format!(
            "{decided} tool call(s) decided in {} turn record(s), none denied{unaccounted}",
            r.turns.len()
        )
    };
    Clause::new(5, !by_frame.is_empty(), found)
}
pub fn six(r: &Records) -> Clause {
    let mut named = r.spend_agents();
    let spawned = named.iter().filter(|(a, _)| r.knows(a)).count();
    let met = named.len() >= 2 && spawned >= 2;
    let found = if r.spend.is_empty() {
        r.spend_error
            .clone()
            .unwrap_or_else(|| "no rows in the spend record".into())
    } else {
        named.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        let mut parts = vec![format!(
            "{} row(s), {} agent(s) named: {}",
            r.spend.len(),
            named.len(),
            or_none(listed(named.iter().map(|(a, n)| format!("`{a}` ×{n}"))))
        )];
        let unattributed = r.spend.len() - named.iter().map(|(_, n)| n).sum::<usize>();
        if unattributed > 0 {
            parts.push(format!("{unattributed} row(s) naming no agent"));
        }
        let strangers: BTreeSet<_> = named
            .iter()
            .filter(|(a, _)| !r.knows(a))
            .map(|(a, _)| a)
            .collect();
        if !strangers.is_empty() {
            parts.push(format!(
                "named by no `{SPAWNED}`: {}",
                listed(strangers.into_iter().map(|a| format!("`{a}`")))
            ));
        }
        parts.join("; ")
    };
    Clause::new(6, met, found)
}
static AGENT_CEILING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"by the agent [`'"]?(?P<agent>[^`'"\s]+)[`'"]? across every goal it works"#)
        .unwrap()
});
fn ceiling_refusals(r: &Records) -> Vec<(String, String)> {
    let mut out = vec![];
    let sentence = |text: Option<&str>, where_: String, out: &mut Vec<(String, String)>| {
        if let Some(c) = text.and_then(|t| AGENT_CEILING.captures(t)) {
            out.push((c["agent"].into(), where_));
        }
    };
    for row in &r.capped {
        let agent = if row["bound"].is_object() {
            nonempty(&row["bound"]["agent"])
        } else if row["bound"] == "agent" {
            nonempty(&row["agent"])
        } else {
            None
        };
        if let Some(agent) = agent {
            out.push((agent.into(), "turns/capped.jsonl".into()));
        } else {
            sentence(row["why"].as_str(), "turns/capped.jsonl".into(), &mut out);
        }
    }
    for row in &r.spend {
        for key in ["note", "why", "error"] {
            sentence(row[key].as_str(), "turns/spend.jsonl".into(), &mut out);
        }
    }
    for e in &r.events {
        sentence(
            Some(&e.data.to_string()),
            format!("`{}` at seq {}", e.name, e.seq),
            &mut out,
        );
    }
    out
}
pub fn seven(r: &Records) -> Clause {
    if let Some(error) = &r.capped_error {
        return Clause::new(7, false, error.clone());
    }
    let known: BTreeSet<_> = r.spawned().into_iter().map(|(a, _)| a).collect();
    let ceilings = ceiling_refusals(r);
    let (refused, unknown): (Vec<_>, Vec<_>) = ceilings
        .iter()
        .partition(|(a, _)| known.contains(a.as_str()));
    let met = !refused.is_empty() && known.len() >= 2;
    let mut parts = vec![format!(
        "{} spawned agent(s) to bound: {}",
        known.len(),
        or_none(listed(known))
    )];
    if !refused.is_empty() {
        parts.push(format!(
            "refused: {}",
            listed(refused.iter().map(|(a, w)| format!("`{a}` ({w})")))
        ));
    } else if !unknown.is_empty() {
        parts.push(format!(
            "a ceiling refusal names an agent no `swarm.agent.AgentSpawned` named: {}",
            listed(unknown.iter().map(|(a, w)| format!("`{a}` ({w})")))
        ));
    } else {
        parts.push(
            "no refusal by the agent ceiling in the event log, in spend.jsonl or in capped.jsonl"
                .into(),
        );
    }
    Clause::new(7, met, parts.join("; "))
}
pub fn unattended(r: &Records) -> Clause {
    let no = |found| Clause::new(0, false, found);
    if let Some(e) = &r.log_error {
        return no(e.clone());
    }
    let Some(opener) = r.named(&[STARTED]).next() else {
        return no(format!(
            "no `{STARTED}` in {} event(s), so there is no window to read",
            r.events.len()
        ));
    };
    let verdict = r.named(VERDICT).filter(|e| e.seq >= opener.seq).last();
    let closes = verdict.unwrap_or_else(|| r.events.last().unwrap()).seq;
    let window: Vec<_> = r
        .events
        .iter()
        .filter(|e| opener.seq <= e.seq && e.seq <= closes)
        .collect();
    let silent = window.iter().filter(|e| e.issued_by().is_none()).count();
    if silent > 0 {
        let operators: BTreeSet<_> = window
            .iter()
            .filter(|e| e.actor.rsplit('.').next() == Some("Operator"))
            .map(|e| &e.actor)
            .collect();
        let corroboration = if operators.is_empty() {
            String::new()
        } else {
            format!(
                "; what it does show is {} in the `actor` column, an actor type the specification declares a person",
                listed(operators)
            )
        };
        return no(format!(
            "{silent} of {} event(s) in the window name no issuer at all: this log was written before commands recorded who issued them, so the absence of a hand cannot be read off it{corroboration}",
            window.len()
        ));
    }
    let after: Vec<_> = window.iter().filter(|e| e.seq != opener.seq).collect();
    if after.is_empty() {
        return no(format!(
            "the window holds one event, the `{STARTED}` at seq {} that opens it and that a person issues by construction. A swarm that was started and then did nothing is not evidence that nobody touched it — there is nothing here either way",
            opener.seq
        ));
    }
    let hands: Vec<_> = after
        .iter()
        .filter(|e| e.issued_by() != Some(RUNTIME) && r.agent_named_by(e.issued_by()).is_none())
        .collect();
    if !hands.is_empty() {
        return no(format!(
            "{} of {} event(s) in the window were issued by neither the `{RUNTIME}` nor an agent this log spawned: {}",
            hands.len(),
            window.len(),
            listed(hands.iter().map(|e| format!(
                "`{}` at seq {} by `{}`",
                e.name,
                e.seq,
                e.issued_by().unwrap()
            )))
        ));
    }
    Clause::new(
        0,
        true,
        format!(
            "{} event(s) from seq {} to seq {closes}{}, every one naming its issuer and every one of them the runtime or an agent this log spawned; issuers: {}",
            window.len(),
            opener.seq,
            verdict
                .map(|e| format!(", the `{}` that closes it", e.name))
                .unwrap_or_else(|| ", the end of the log".into()),
            listed(
                window
                    .iter()
                    .filter_map(|e| e.issued_by())
                    .collect::<BTreeSet<_>>()
            )
        ),
    )
}
