//! Loading a specification from a directory, and holding the compiled IR.
//!
//! The IR is `Serialize`-only, sealed against field access, and carries no format version — so it is
//! never written down and read back. The runtime links the compiler and compiles the specification
//! at boot, which also means the guards it evaluates are the parsed `Predicate` values the compiler
//! produced rather than strings this crate would have to re-parse.
//!
//! One step of the CLI's pipeline is not reusable: `ess-cli`'s input discovery is private to that
//! crate. It is reimplemented here, and deliberately only for the manifest form — a directory with
//! an `ess-inputs.yaml` naming its files exactly. The legacy form reads every `*.yaml` beneath the
//! directory, which is what put a run's own records into a specification in the first place.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use ess_compiler::EssIr;
use ess_compiler::source::SourceMap;

/// The manifest that names a specification's files.
const MANIFEST: &str = "ess-inputs.yaml";
const MANIFEST_FORMAT: &str = "ess-inputs/1";

/// Why a specification did not load.
#[derive(Debug)]
pub enum LoadError {
    /// The directory, or a file the manifest names, could not be read.
    Unreadable { path: PathBuf, why: std::io::Error },
    /// There is no `ess-inputs.yaml`, or it is not one.
    Manifest { path: PathBuf, why: String },
    /// A file is not valid ESS.
    Parse { source: String, why: String },
    /// The files do not assemble, or do not resolve, into one system.
    Refused { problems: Vec<String> },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { path, why } => write!(f, "{}: {why}", path.display()),
            Self::Manifest { path, why } => write!(f, "{}: {why}", path.display()),
            Self::Parse { source, why } => write!(f, "{source}: {why}"),
            Self::Refused { problems } => {
                writeln!(f, "the specification does not resolve:")?;
                for problem in problems {
                    writeln!(f, "  - {problem}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for LoadError {}

/// A compiled specification, and where it was read from.
pub struct Spec {
    ir: EssIr,
    root: PathBuf,
    files: usize,
}

impl Spec {
    /// Reads `<dir>/ess-inputs.yaml`, then every file it names, and compiles them into one system.
    pub fn load(dir: impl AsRef<Path>) -> Result<Self, LoadError> {
        let root = dir.as_ref().to_path_buf();
        let named = manifest_inputs(&root)?;

        let mut parsed = Vec::new();
        let mut texts = SourceMap::new();
        let mut problems = Vec::new();

        for relative in &named {
            let path = root.join(relative);
            let text = fs::read_to_string(&path).map_err(|why| LoadError::Unreadable {
                path: path.clone(),
                why,
            })?;
            let source = ess_domain::system::Source::new(relative.clone());
            texts.insert(source.as_str(), text.as_str());
            match ess_domain::spec::RawSpecFile::parse(&text) {
                Ok(raw) => parsed.push((source, raw)),
                Err(why) => problems.push(format!("{relative}: {why}")),
            }
        }

        if !problems.is_empty() {
            return Err(LoadError::Refused { problems });
        }

        let assembled = ess_domain::spec::Specification::assemble(parsed).map_err(|errors| {
            LoadError::Refused {
                problems: errors.as_slice().iter().map(ToString::to_string).collect(),
            }
        })?;

        let ir = ess_compiler::compile(&assembled, &texts).map_err(|diagnostics| {
            LoadError::Refused {
                problems: diagnostics
                    .as_slice()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            }
        })?;

        Ok(Self {
            ir,
            root,
            files: named.len(),
        })
    }

    /// The resolved IR. Everything the interpreter does, it does by reading this.
    pub fn ir(&self) -> &EssIr {
        &self.ir
    }

    /// Where the specification was read from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// How many files it was assembled from.
    pub fn files(&self) -> usize {
        self.files
    }
}

/// The `specification:` list of a `ess-inputs/1` manifest, in the order it declares.
fn manifest_inputs(root: &Path) -> Result<Vec<String>, LoadError> {
    let path = root.join(MANIFEST);
    let text = fs::read_to_string(&path).map_err(|why| LoadError::Unreadable {
        path: path.clone(),
        why,
    })?;

    let document: serde_yaml::Value =
        serde_yaml::from_str(&text).map_err(|why| LoadError::Manifest {
            path: path.clone(),
            why: why.to_string(),
        })?;

    let format = document.get("format").and_then(|value| value.as_str());
    if format != Some(MANIFEST_FORMAT) {
        return Err(LoadError::Manifest {
            path,
            why: format!("expected `format: {MANIFEST_FORMAT}`, found {format:?}"),
        });
    }

    let listed = document
        .get("specification")
        .and_then(|value| value.as_sequence())
        .ok_or_else(|| LoadError::Manifest {
            path: path.clone(),
            why: "no `specification:` list".to_owned(),
        })?;

    let mut inputs = Vec::with_capacity(listed.len());
    for entry in listed {
        let name = entry.as_str().ok_or_else(|| LoadError::Manifest {
            path: path.clone(),
            why: format!("`specification:` holds {entry:?}, which is not a file name"),
        })?;
        inputs.push(name.to_owned());
    }

    if inputs.is_empty() {
        return Err(LoadError::Manifest {
            path,
            why: "`specification:` is empty".to_owned(),
        });
    }

    Ok(inputs)
}
