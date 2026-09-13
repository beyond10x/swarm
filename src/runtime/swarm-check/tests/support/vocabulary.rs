use std::collections::BTreeSet;
use swarm_check::records::{AGENT, OPERATOR, RUNTIME, UNNAMEABLE_AGENT};
pub fn expected() -> BTreeSet<String> {
    [RUNTIME, OPERATOR, AGENT, UNNAMEABLE_AGENT]
        .map(String::from)
        .into_iter()
        .collect()
}
pub fn labels(source: &str) -> Result<BTreeSet<String>, String> {
    use regex::Regex;
    let enum_re = Regex::new(r"(?s)pub enum Issuer \{(.*?)\n\}").unwrap();
    let enum_body = enum_re.captures(source).ok_or("missing enum Issuer")?;
    let variants_re = Regex::new(r"(?m)^    (\w+)\s*[,({]").unwrap();
    let variants: Vec<_> = variants_re
        .captures_iter(&enum_body[1])
        .map(|c| c[1].to_string())
        .collect();
    if variants.is_empty() {
        return Err("enum Issuer has no readable variants".into());
    }
    let const_re = Regex::new(r#"(?m)^const (\w+): &str = "([^"]*)";"#).unwrap();
    let consts: std::collections::BTreeMap<_, _> = const_re
        .captures_iter(source)
        .map(|c| (c[1].to_string(), c[2].to_string()))
        .collect();
    let body_re = Regex::new(r"(?s)pub fn label\(&self\) -> String \{(.*?)\n    \}").unwrap();
    let body = body_re.captures(source).ok_or("missing Issuer::label")?;
    let literal_re = Regex::new(r#"^"([^"]*)"\.to_owned\(\)$"#).unwrap();
    let named_re = Regex::new(r"^(\w+)\.to_owned\(\)$").unwrap();
    let formatted_re = Regex::new(r#"^format!\("\{(\w+)\}"#).unwrap();
    let mut labels = BTreeSet::new();
    for variant in variants {
        let arm_re = Regex::new(&format!(
            r"Self::{}\b(?:\([^)]*\)|\s*\{{[^}}]*\}})?\s*=>\s*(.+)",
            regex::escape(&variant)
        ))
        .unwrap();
        let arm = arm_re
            .captures(&body[1])
            .ok_or_else(|| format!("Issuer::{variant} has no readable label arm"))?;
        let arm = arm[1].trim().trim_end_matches(',');
        let literal = literal_re.captures(arm).map(|c| c[1].to_owned());
        let constant = named_re
            .captures(arm)
            .or_else(|| formatted_re.captures(arm));
        let label = literal
            .or_else(|| constant.and_then(|c| consts.get(&c[1]).cloned()))
            .ok_or_else(|| {
                format!("Issuer::{variant} has a label this checker cannot read: {arm}")
            })?;
        labels.insert(label);
    }
    Ok(labels)
}

pub fn guard(source: &str) -> Result<(), String> {
    let actual = labels(source)?;
    if actual == expected() {
        Ok(())
    } else {
        Err(format!("issuer vocabulary drift: {actual:?}"))
    }
}
pub const FOUR: &str = r#"pub enum Issuer {
    Runtime,
    Operator,
    Agent(String),
    UnnameableAgent(String),
}
const AGENT: &str = "agent:";
const UNNAMEABLE: &str = "agent:?";
impl Issuer {
    pub fn label(&self) -> String {
        match self {
            Self::Runtime => "runtime".to_owned(),
            Self::Operator => "operator".to_owned(),
            Self::Agent(name) => format!("{AGENT}{name}"),
            Self::UnnameableAgent(claim) => format!("{UNNAMEABLE}{}", mark(claim)),
        }
    }
}
"#;
pub fn fifth(shape: &str, arm: &str) -> String {
    let changed = FOUR.replace(
        "    UnnameableAgent(String),\n}",
        &format!("    UnnameableAgent(String),\n    {shape},\n}}"),
    );
    let changed = changed.replace(
        "        }\n    }",
        &format!("            {arm}\n        }}\n    }}"),
    );
    assert_ne!(changed, FOUR);
    assert!(changed.contains("Scheduler"));
    changed
}
