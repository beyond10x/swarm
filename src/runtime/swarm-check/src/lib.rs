//! Evidence verdicts for recorded two-agent runs. Seven clauses decide success; unattended is separate.
mod clauses;
pub mod evidence;
mod language;
pub mod records;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct Clause {
    pub number: usize,
    pub title: &'static str,
    pub met: bool,
    pub looked_for: &'static str,
    pub found: String,
}
impl Clause {
    fn new(number: usize, met: bool, found: String) -> Self {
        let (title, looked_for) = language::LANGUAGE[number];
        Self {
            number,
            title,
            met,
            looked_for,
            found,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub swarm: String,
    pub clauses: Vec<Clause>,
    pub unattended: Clause,
    pub met: usize,
    pub not_met: usize,
    pub ok: bool,
}
pub fn check(root: &Path) -> Report {
    let r = records::read(root);
    let clauses = vec![
        clauses::one(&r),
        clauses::two(&r),
        clauses::three(&r),
        clauses::four(&r),
        clauses::five(&r),
        clauses::six(&r),
        clauses::seven(&r),
    ];
    let met = clauses.iter().filter(|c| c.met).count();
    Report {
        swarm: root.to_string_lossy().into_owned(),
        clauses,
        unattended: clauses::unattended(&r),
        met,
        not_met: 7 - met,
        ok: met == 7,
    }
}
impl Report {
    pub fn render(&self) -> String {
        let state = |met| if met { "met    " } else { "NOT MET" };
        let mut lines = vec![format!("swarm: {}", self.swarm), String::new()];
        for c in &self.clauses {
            lines.push(format!(
                "clause {}  {}  {}",
                c.number,
                state(c.met),
                c.title
            ));
            if !c.met {
                lines.push(format!("          wanted: {}", c.looked_for));
            }
            lines.push(format!("          found:  {}", c.found));
        }
        lines.extend([
            String::new(),
            format!(
                "unattended  {}  no operator input in the window",
                state(self.unattended.met)
            ),
            format!("          wanted: {}", self.unattended.looked_for),
            format!("          found:  {}", self.unattended.found),
            String::new(),
        ]);
        let mut summary = format!("7 clauses: {} met, {} not met", self.met, self.not_met);
        let missed: Vec<_> = self
            .clauses
            .iter()
            .filter(|c| !c.met)
            .map(|c| format!("clause {}", c.number))
            .collect();
        if !missed.is_empty() {
            summary.push_str(&format!(" — {}", missed.join(", ")));
        }
        summary.push_str(&format!(
            "; unattended {}, which is reported beside them and decides nothing",
            if self.unattended.met {
                "met"
            } else {
                "NOT MET"
            }
        ));
        lines.push(summary);
        lines.join("\n")
    }
}
