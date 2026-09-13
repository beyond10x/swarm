//! The frame a turn runs under: what it may do, where, and the seal that fixes both.
//!
//! Until 2026-09-13 a coordinator turn was launched `--decisions observe`, which metaharness's own
//! help calls "the capture mode, and nothing else": every call the model made was allowed and
//! recorded. This module builds the document that replaces it — a sealed `metaharness.frame/1`,
//! passed as `--frame <file>` with `--decisions frame`, from which the adapter decides each call
//! with no round trip.
//!
//! # What a frame is and is not
//!
//! It is refusal at the decision seam. It is **not** containment: `--substrate`,
//! `--substrate-embedded`, `--cgroup-root` and `--write-scope` are `b10x` only — metaharness
//! refuses them by name for the `claude` arm — so `AGENTS.md`'s standing rule, that nothing
//! confines a coordinator, is not repealed by this file. A call outside the admitted set is
//! refused; a process that never makes a call is not stopped from anything.
//!
//! Measured on 2026-09-13 against metaharness 0.6.4: a frame admitting one operation left all 26
//! vendor tools *offered*, `available_operations` at seven and `withheld: null`, and refused the
//! `Bash` call the model then made (`census.denied: 1`, `by_decider: {"frame": 1}`). The frame
//! narrows what is **attempted**, not what is offered — which is why
//! [`crate::coordinator::prompt_for`] states the admitted set in words as well. metaharness's own
//! flag for that, `--scope-announce`, is `b10x` only too.
//!
//! # The digest, and why it cannot be computed over the bytes written
//!
//! metaharness seals a frame with `sha256` over **its own `serde_json` serialisation of the parsed
//! `Frame`**, with `digest` and `format` absent
//! (`metaharness-protocol/src/frame.rs:345-357`, identical in 0.6.4 and 0.7.0). Hashing the bytes
//! this runtime wrote does not reproduce it: parsing fills defaults and drops fields a
//! hand-written document may spell differently. A coordinator measured exactly that on 2026-09-13
//! — `24bcbf9c…` by hand against metaharness's `4ebba9f9…` for one document.
//!
//! So [`digest_of`] hashes a `serde_json::Value`, whose map is a `BTreeMap` and therefore already
//! in the key order metaharness's own serialisation produces, and [`document`] builds that value
//! with exactly the fields `Frame` declares — nothing omitted that the struct requires, nothing
//! added that it does not know. `the_digest_is_metaharnesss_own_and_the_fixture_proves_it` checks
//! that against metaharness's published canonical fixture, which carries its own correct digest.
//! It is the offline oracle for this file: when it is green the seal is right, and no paid run was
//! needed to find out.
//!
//! # The subject scope is derived here and is not a config field
//!
//! `swarm.config.HarnessLaunch` carries `admitted_operations`, because narrowing the verb set is a
//! choice an operator may make and narrowing is always safe. The **subject scope** is not there: it
//! is derived from the swarm's own work directory by [`scope`]. A scope in the config is a scope a
//! coordinator can widen by writing itself a config — `swarm do` reaches `swarm.config.DraftConfig`
//! like every other command — and a confinement the confined party can edit confines nothing.

use std::path::Path;

use serde_json::{Map, Value as Json, json};

/// The format tag metaharness checks first, and refuses a document without.
pub const FORMAT: &str = "metaharness.frame/1";

/// Every operation metaharness's closed vocabulary names without a parameter.
///
/// `mcp.call` is absent because it is parameterised by server and tool, so there is no name for it
/// a list of strings could carry (`metaharness-protocol::Operation::PARAMETERLESS`).
pub const VOCABULARY: [&str; 10] = [
    "dir.list",
    "file.edit",
    "file.read",
    "file.write",
    "search",
    "shell",
    "skill.load",
    "subagent.spawn",
    "task.todo",
    "web.read",
];

/// What a turn admits when the swarm's config narrows nothing.
///
/// The seven of metaharness's own canonical frame. Narrower than what the run had before this
/// existed: the 2026-09-13 probe measured `available_operations` at seven *including* `web.read`
/// and `subagent.spawn` under `--decisions observe`, and neither is here.
///
/// `shell` is the one that cannot be narrowed away: `swarm do`, the only route from a turn back
/// into this system, is a shell call. It is also the operation the subject scope cannot judge —
/// metaharness derives subjects from a call's path arguments (`metaharness-tools::resolve`), and a
/// shell call has none, so a scope has no opinion on it. Said here rather than left to be
/// discovered.
pub const ADMITTED: [&str; 7] = [
    "dir.list",
    "file.edit",
    "file.read",
    "file.write",
    "search",
    "shell",
    "skill.load",
];

/// Of [`VOCABULARY`], the operations that only read.
///
/// What an agent is allowed on the swarm's shared directory, intersected with what its turn admits
/// at all. `skill.load` is absent although it reads: it names a skill rather than a path, so no
/// subject rule decides it and admitting it "on the shared directory" would be admitting it
/// everywhere under a name that suggested otherwise.
pub const READS: [&str; 3] = ["dir.list", "file.read", "search"];

/// Why a turn has no frame, which is always a reason not to launch it.
#[derive(Debug)]
pub enum Unframed {
    /// An operation named by a config is not in metaharness's closed vocabulary.
    Unadmitted { operation: String },
    /// The document could not be written where the turn would read it from.
    Unwritable { why: String },
}

impl std::fmt::Display for Unframed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unadmitted { operation } => write!(
                f,
                "the operation {operation:?} is not one metaharness admits; the closed vocabulary \
                 is {VOCABULARY:?}"
            ),
            Self::Unwritable { why } => write!(f, "the frame could not be written: {why}"),
        }
    }
}

/// What one turn is, as much of it as a frame carries.
#[derive(Debug)]
pub struct Turn<'a> {
    /// The workflow this turn belongs to, as this runtime names it.
    pub workflow: &'a str,
    /// The state the run is in — the node of the workflow.
    pub state: &'a str,
    /// Which step of the run this is. The turn number.
    pub index: u32,
    /// Which attempt at that step, from 1.
    pub attempt: u32,
    /// What must hold while here, verbatim, one line each.
    pub obligations: Vec<String>,
    /// What does not hold yet on the way out.
    pub reaching: Vec<String>,
    /// The operations admitted, by their metaharness names.
    pub operations: Vec<String>,
    /// The AGENT's own work directory, which is the whole of where those operations may WRITE.
    ///
    /// Per agent and not per swarm since correction round 1 of the 2026-09-13a wave. One directory
    /// for every agent was correct while a swarm had one, and the prompt tells every agent to keep
    /// a `NOTES.md` in it — so two members in one swarm wrote over each other's memory of what
    /// they had done, in a wave whose whole point is that there are two members.
    pub work: &'a Path,
    /// The swarm's shared work directory — the parent of [`Self::work`] — which every agent may
    /// READ and none may write.
    ///
    /// Readable because a member that cannot see what the coordinator or its peers left is working
    /// blind, and the alternative (copying it in) would be two copies going out of step. Not
    /// writable because that is precisely the overwriting this separation exists to end: the
    /// routes an agent has for reaching another agent are `swarm send` and `swarm do`, both of
    /// which the specification records.
    pub shared: &'a Path,
}

/// The digest metaharness computes for this frame: `sha256` over its own serialisation of the
/// parsed document, with `digest` and `format` absent, hex, lowercase.
#[must_use]
pub fn digest_of(frame: &Json) -> String {
    let mut value = frame.clone();
    if let Some(object) = value.as_object_mut() {
        object.remove("digest");
        object.remove("format");
    }
    let bytes = serde_json::to_vec(&value).expect("a serde_json::Value serialises");
    sha256_hex(&bytes)
}

/// Where the admitted operations may act: the agent's own directory to write, the swarm's shared
/// one to read, and nothing else at all.
///
/// Six rules, in this order, because metaharness's scope is **first match wins** and "a scope that
/// is not empty ends in a catch-all" (`SubjectScope`'s own doc):
///
/// 1. **an escaping relative path is refused** — `../x`, `a/../../x` — before any rule admits it;
/// 2. **the agent's own directory is admitted**, everything the turn admits. It lies inside the
///    shared directory, so it must be matched first or rule 3 would swallow it;
/// 3. **the swarm's shared directory is readable and not writable** — [`READS`] intersected with
///    what the turn admits. A member can read the coordinator's `NOTES.md` and its peers'; it
///    cannot write over them;
/// 4. **every other absolute path is refused.** This is the rule that makes the scope a
///    confinement rather than a list of verbs;
/// 5. **what is left is admitted**: a relative path that does not climb, which is a path inside
///    `--cwd`, which is the agent's own directory;
/// 6. **everything else is refused** — every subject that is not a file: `proc:`, `host:`.
///
/// Rule 5 is not a loophole and rule 1 is why. metaharness builds subjects from a call's own path
/// arguments **verbatim** — `Subject::file` is `format!("file:{path}")` and canonicalises nothing
/// (`harness-wire/src/envelope.rs:113`) — so a relative path arrives relative and an absolute one
/// arrives absolute, and the two need different rules. Without rule 5, a `Grep` given a relative
/// `path` inside the agent's directory would fall to the catch-all and be refused, and the agent
/// would spend a billed turn learning that its own directory was out of bounds.
///
/// A consequence worth stating: the shared directory is reachable only by its ABSOLUTE path, since
/// a relative route to it must climb and rule 1 refuses that. The prompt gives the absolute one.
///
/// # The condition rule 4 holds under, which is not "always"
///
/// metaharness judges a call by **the first rule ANY of its subjects matches**
/// (`SubjectScope::verdict`), and a call's subject list is a `Vec`. So rule 4 refuses an outside
/// path **when the call names no admitted path beside it**. A call carrying both — the agent's own
/// file and `/etc/passwd` in one input — matches rule 2 first, and rule 2's verdict governs every
/// subject of that call, including the one rule 4 exists to refuse.
///
/// **Nothing found reaches it.** metaharness builds the list from three argument names —
/// `file_path`, `notebook_path`, `path` (`metaharness-tools::subjects_of_vendor_call`) — so it
/// takes a vendor tool input carrying two of them at once, and the adversary of correction round 2
/// looked for one and did not find it. Claude Code's `Read`, `Write` and `Edit` take a single
/// `file_path`; `Grep` and `Glob` take a single `path`.
///
/// **And no rule ordering closes it.** Refusing the pair needs a rule that matches the outside path
/// and NOT the agent's own, placed before rule 2 — so it must discriminate on a literal prefix,
/// and this matcher has `*`, `**` and literals with no negation and no character classes (its own
/// doc says so). The complement of "under the work directory" IS expressible as a finite union of
/// literal-prefix globs — agree on the first *i* bytes, differ at *i*, for every *i* and every
/// other byte — and for a real work directory that is **14,848 patterns**, regenerated per swarm
/// and sealed into every frame. `SubjectScope` exists to be "a boundary somebody has to be able to
/// read at a glance"; buying this against a defect with nothing reaching it would spend that
/// property to gain nothing measurable. Stated here instead, which is what the sealed document can
/// honestly claim.
///
/// What this does NOT reach either: a symbolic link inside a work directory pointing out of it.
/// The subject is the path as written and nothing here resolves one. That is kernel-level
/// confinement, it is `b10x` only, and this file does not claim it.
#[must_use]
pub fn scope(own: &Path, shared: &Path, operations: &[String]) -> Json {
    let own = own.display().to_string();
    let shared = shared.display().to_string();
    let admitted: Vec<Json> = operations.iter().map(|op| json!({"op": op})).collect();
    let readable: Vec<Json> = operations
        .iter()
        .filter(|op| READS.contains(&op.as_str()))
        .map(|op| json!({"op": op}))
        .collect();
    json!({
        "rules": [
            {
                "subjects": ["file:..", "file:../**", "file:**/..", "file:**/../**"],
                "operations": [],
            },
            {
                "subjects": [format!("file:{own}"), format!("file:{own}/**")],
                "operations": admitted,
            },
            {
                "subjects": [format!("file:{shared}"), format!("file:{shared}/**")],
                "operations": readable,
            },
            {
                "subjects": ["file:/**"],
                "operations": [],
            },
            {
                "subjects": ["file:**"],
                "operations": admitted,
            },
            {
                "subjects": ["**"],
                "operations": [],
            },
        ]
    })
}

/// The sealed document, as the file `--frame` is given.
///
/// Every field `metaharness_protocol::Frame` declares is present: a document missing one is
/// refused as `Invalid` before a process exists, which is the same refusal a wrong digest gets and
/// is just as fatal to the turn.
///
/// # Errors
///
/// [`Unframed::Unadmitted`] when an operation is outside metaharness's closed vocabulary — a
/// config typo, caught here rather than by a refusal from a binary that has already been started.
pub fn document(turn: &Turn<'_>) -> Result<String, Unframed> {
    let mut operations: Vec<String> = Vec::new();
    for operation in &turn.operations {
        if !VOCABULARY.contains(&operation.as_str()) {
            return Err(Unframed::Unadmitted {
                operation: operation.clone(),
            });
        }
        if !operations.contains(operation) {
            operations.push(operation.clone());
        }
    }
    // `OperationSet` is a `BTreeSet` ordered by the operation's own wire name, and the digest is
    // over that order: "operations sorted by their `op` name" is the rule an outside producer
    // follows (`Operation::sort_key`). Sorted here rather than trusted from the caller.
    operations.sort();

    let line = |text: &String| json!({"text": text, "asked_by": null});
    let mut frame = Map::new();
    frame.insert(
        "workflow".into(),
        json!({"id": turn.workflow, "version": "1"}),
    );
    frame.insert("node".into(), json!({"id": turn.state}));
    frame.insert(
        "step".into(),
        json!({"workflow": turn.workflow, "state": turn.state,
               "index": turn.index, "attempt": turn.attempt}),
    );
    frame.insert("prior".into(), json!([]));
    frame.insert(
        "obligations".into(),
        Json::Array(turn.obligations.iter().map(line).collect()),
    );
    frame.insert(
        "reaching".into(),
        Json::Array(turn.reaching.iter().map(line).collect()),
    );
    frame.insert("next".into(), json!([]));
    frame.insert("handoff".into(), json!({"handoff": "none"}));
    frame.insert(
        "operations".into(),
        Json::Array(operations.iter().map(|op| json!({"op": op})).collect()),
    );
    frame.insert(
        "subjects".into(),
        scope(turn.work, turn.shared, &operations),
    );
    frame.insert("entities".into(), Json::Null);

    let mut sealed = Json::Object(frame);
    let digest = digest_of(&sealed);
    let object = sealed.as_object_mut().expect("an object");
    object.insert("digest".into(), Json::String(digest));
    object.insert("format".into(), Json::String(FORMAT.to_owned()));
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&sealed).expect("a serde_json::Value serialises")
    ))
}

/// Writes the sealed frame where the turn will read it from, and answers with that path.
///
/// # Errors
///
/// [`Unframed`], either way it fails. There is no third answer and in particular no `Option`: a
/// turn whose frame could not be written is a turn that does not launch.
pub fn write(at: &Path, turn: &Turn<'_>) -> Result<std::path::PathBuf, Unframed> {
    let document = document(turn)?;
    if let Some(parent) = at.parent() {
        std::fs::create_dir_all(parent).map_err(|why| Unframed::Unwritable {
            why: format!("{}: {why}", parent.display()),
        })?;
    }
    std::fs::write(at, document).map_err(|why| Unframed::Unwritable {
        why: format!("{}: {why}", at.display()),
    })?;
    Ok(at.to_owned())
}

/// The digest of these bytes, hex, lowercase.
///
/// Exported for `coordinator::agent_directory`, which needs a stable injective tail for a slug
/// that is not already a safe path segment. The alternative was a second hash in the crate.
#[must_use]
pub fn digest_of_bytes(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}

/// SHA-256, hex, lowercase.
///
/// Written here rather than taken from `sha2`, because the only alternative was a new dependency
/// in `Cargo.lock`, which is not this unit's file to change. FIPS 180-4 §6.2, checked against that
/// document's own vectors and against metaharness's canonical frame below — three published
/// answers this cannot agree with by accident.
fn sha256_hex(bytes: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];

    let mut message = bytes.to_vec();
    let bits = (bytes.len() as u64) * 8;
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bits.to_be_bytes());

    for chunk in message.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (index, word) in chunk.as_chunks::<4>().0.iter().enumerate() {
            w[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let one = h
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let two = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(one);
            d = c;
            c = b;
            b = a;
            a = one.wrapping_add(two);
        }
        for (held, computed) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *held = held.wrapping_add(computed);
        }
    }

    let mut hex = String::with_capacity(64);
    for word in state {
        hex.push_str(&format!("{word:08x}"));
    }
    hex
}

/// Every key `metaharness_protocol::Frame` deserialises, so a document missing one is refused
/// before a process exists. Named here so the case below can enumerate rather than spot-check.
#[cfg(test)]
const REQUIRED: [&str; 11] = [
    "workflow",
    "node",
    "step",
    "prior",
    "obligations",
    "reaching",
    "next",
    "handoff",
    "operations",
    "entities",
    "digest",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn a_turn<'a>(work: &'a Path, shared: &'a Path, operations: &'a [String]) -> Turn<'a> {
        Turn {
            workflow: "swarm/coordinator",
            state: "pursuing",
            index: 3,
            attempt: 1,
            obligations: vec!["end with one VERDICT line".to_owned()],
            reaching: vec!["to finish: the goal as written is met".to_owned()],
            operations: operations.to_vec(),
            work,
            shared,
        }
    }

    fn admitted() -> Vec<String> {
        ADMITTED.iter().map(|op| (*op).to_owned()).collect()
    }

    /// The offline oracle. metaharness's own canonical frame carries the digest metaharness
    /// computes for it; this runtime's sealer computes the same number over the same document or
    /// it does not seal frames metaharness will accept.
    ///
    /// A coordinator's two probes cost $0.224 establishing that the digest cannot be had by
    /// hashing the bytes one writes. This case is what that money bought: a paid run is no longer
    /// how this file finds out whether it is right.
    #[test]
    fn the_digest_is_metaharnesss_own_and_the_fixture_proves_it() {
        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/metaharness-frame-canonical.json");
        let text = std::fs::read_to_string(&fixture).expect("the canonical frame is vendored");
        let canonical: Json = serde_json::from_str(&text).expect("it is JSON");
        let stated = canonical["digest"].as_str().expect("it states its digest");

        assert_eq!(
            digest_of(&canonical),
            stated,
            "metaharness computes {stated} for its own canonical frame"
        );
        assert_eq!(
            stated, "43a6f845a21f3475569323950a9d276bfed3df11979adc3edf18878da6963a12",
            "and the fixture is the one the story named"
        );
    }

    /// FIPS 180-4's own two vectors, so a green oracle above cannot be a hash that is wrong in a
    /// way the one fixture happens not to see.
    #[test]
    fn the_hash_answers_the_published_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    /// Clause 4, as far as it can be had without spending money: a frame this runtime seals is
    /// one metaharness reads back — every field its `Frame` declares is present, and the digest
    /// stated is the digest the contents imply.
    #[test]
    fn a_frame_this_runtime_seals_states_the_digest_its_contents_imply() {
        let operations = admitted();
        let text = document(&a_turn(
            Path::new("/data/swarms/s/work/coordinator"),
            Path::new("/data/swarms/s/work"),
            &operations,
        ))
        .expect("the admitted set is metaharness's own vocabulary");
        let parsed: Json = serde_json::from_str(&text).expect("the document is JSON");

        assert_eq!(parsed["format"], FORMAT, "the tag metaharness checks first");
        for key in REQUIRED {
            assert!(
                parsed.get(key).is_some(),
                "a document without {key} is refused as not a frame before a process exists"
            );
        }
        let stated = parsed["digest"].as_str().expect("a digest");
        assert_eq!(stated.len(), 64, "sha256, hex: {stated}");
        assert!(
            stated
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
            "lowercase hex: {stated}"
        );
        assert_eq!(
            digest_of(&parsed),
            stated,
            "the seal describes the contents, which is the whole of what the digest is for"
        );
    }

    /// The seal is over the contents: change one character of the document and it no longer
    /// describes it. This is the property the 0.6.4 refusal measured — exit 2, no session, nothing
    /// billed — read from this side.
    #[test]
    fn an_edited_frame_no_longer_states_its_own_digest() {
        let operations = admitted();
        let text = document(&a_turn(
            Path::new("/data/swarms/s/work/coordinator"),
            Path::new("/data/swarms/s/work"),
            &operations,
        ))
        .expect("it seals");
        let edited: Json =
            serde_json::from_str(&text.replace("pursuing", "whatever")).expect("still JSON");
        assert_ne!(
            digest_of(&edited),
            edited["digest"].as_str().expect("a digest"),
            "a frame edited after sealing is refused rather than silently repaired"
        );
    }

    /// Clause 3, and the finding that two members shared one directory: the scope admits the
    /// AGENT's own directory, lets it read the swarm's shared one, and refuses every way out.
    ///
    /// The rules are ordered and metaharness takes the first that matches, so this reads them as a
    /// sequence. What each is for:
    ///
    /// 1. an escaping relative path — `../x`, `a/../../x` — refused before anything admits it;
    /// 2. **the agent's own directory**, fully admitted. It is inside the shared one, so it has to
    ///    come first: first match wins;
    /// 3. **the swarm's shared directory, readable and not writable.** A member can read what the
    ///    coordinator and its peers left; it cannot write over any of it;
    /// 4. every other absolute path, refused — the rule that makes this a confinement;
    /// 5. what is left is relative, and `--cwd` is the agent's own directory, so it is inside it;
    /// 6. everything else: every subject that is not a file at all, `proc:`, `host:`.
    #[test]
    fn the_scope_is_the_agents_own_directory_and_a_readable_shared_one() {
        let operations = admitted();
        let text = document(&a_turn(
            Path::new("/data/swarms/s/work/builder"),
            Path::new("/data/swarms/s/work"),
            &operations,
        ))
        .expect("it seals");
        let parsed: Json = serde_json::from_str(&text).expect("JSON");
        let rules = parsed["subjects"]["rules"]
            .as_array()
            .expect("the scope is sealed with everything else");

        let subjects: Vec<&Json> = rules.iter().map(|rule| &rule["subjects"]).collect();
        assert_eq!(
            subjects,
            vec![
                &json!(["file:..", "file:../**", "file:**/..", "file:**/../**"]),
                &json!([
                    "file:/data/swarms/s/work/builder",
                    "file:/data/swarms/s/work/builder/**"
                ]),
                &json!(["file:/data/swarms/s/work", "file:/data/swarms/s/work/**"]),
                &json!(["file:/**"]),
                &json!(["file:**"]),
                &json!(["**"]),
            ],
            "first match wins, so the order IS the rule — and the agent's own directory is inside \
             the shared one, so it has to be read first"
        );

        let names = |rule: &Json| -> Vec<String> {
            rule["operations"]
                .as_array()
                .expect("a set")
                .iter()
                .map(|op| op["op"].as_str().unwrap_or_default().to_owned())
                .collect()
        };
        assert_eq!(
            names(&rules[0]),
            Vec::<String>::new(),
            "the way out: nothing"
        );
        assert_eq!(
            names(&rules[1]),
            admitted(),
            "its own directory: everything the turn admits"
        );
        assert_eq!(
            names(&rules[2]),
            vec!["dir.list", "file.read", "search"],
            "the shared directory: what a reader needs and nothing that writes. A member that \
             could write here would be writing over the coordinator's NOTES.md and its peers'"
        );
        assert_eq!(names(&rules[3]), Vec::<String>::new(), "other absolutes");
        assert_eq!(
            names(&rules[4]),
            admitted(),
            "relative, and --cwd is the agent's own directory"
        );
        assert_eq!(
            names(&rules[5]),
            Vec::<String>::new(),
            "and everything else"
        );
    }

    /// The read-only share is an intersection, not a fixed list: a turn that does not admit an
    /// operation at all does not get it back through the shared directory.
    #[test]
    fn the_shared_directory_cannot_admit_what_the_turn_does_not() {
        let operations = vec!["file.read".to_owned(), "file.write".to_owned()];
        let text =
            document(&a_turn(Path::new("/w/one"), Path::new("/w"), &operations)).expect("it seals");
        let parsed: Json = serde_json::from_str(&text).expect("JSON");
        let shared = &parsed["subjects"]["rules"][2];

        assert_eq!(
            shared["subjects"],
            json!(["file:/w", "file:/w/**"]),
            "the rule under test is the shared one"
        );
        assert_eq!(
            shared["operations"],
            json!([{"op": "file.read"}]),
            "`dir.list` and `search` are not admitted by this turn, so they are not admitted here \
             either; `file.write` is admitted but writes, so it is not admitted here"
        );
    }

    /// A config that names an operation metaharness does not have is refused here, by name. The
    /// alternative is a binary started and refused, which costs a process and says less.
    #[test]
    fn an_operation_metaharness_does_not_know_is_refused_by_name() {
        let operations = vec!["file.read".to_owned(), "rm -rf".to_owned()];
        let why = document(&a_turn(Path::new("/w/one"), Path::new("/w"), &operations))
            .expect_err("the vocabulary is closed");
        assert!(
            why.to_string().contains("rm -rf") && why.to_string().contains("file.read"),
            "the refusal names what was asked for and what there is: {why}"
        );
    }

    /// The digest is over the operations in one order, whatever order they were given in — the
    /// rule an outside producer follows is "sorted by their `op` name".
    #[test]
    fn the_operations_are_sealed_in_one_order_whatever_order_they_arrive_in() {
        let forwards = admitted();
        let mut backwards = admitted();
        backwards.reverse();
        let work = Path::new("/w/one");
        let shared = Path::new("/w");
        assert_eq!(
            document(&a_turn(work, shared, &forwards)).expect("seals"),
            document(&a_turn(work, shared, &backwards)).expect("seals"),
        );
    }
}
