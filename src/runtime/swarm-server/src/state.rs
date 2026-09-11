//! What the server holds: the specification, and every swarm open against it.
//!
//! One specification for all of them. A swarm is data, not a program — two swarms differ in what
//! their logs contain and in nothing else, which is what makes "the spec is the system" true rather
//! than aspirational.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::RwLock;

use ess_runtime::Spec;

use crate::swarm::{Refused, Swarm};

/// The whole server.
pub struct Server {
    spec: Arc<Spec>,
    root: PathBuf,
    swarms: RwLock<BTreeMap<String, Arc<Swarm>>>,
}

impl Server {
    /// Compiles the specification and opens every swarm already on disk.
    pub async fn start(spec: Spec, root: PathBuf) -> Result<Self, Refused> {
        let spec = Arc::new(spec);
        let mut swarms = BTreeMap::new();

        // A swarm that exists is a directory that exists. Nothing is registered anywhere else, so
        // there is no index to fall out of step with the disk.
        if let Ok(entries) = std::fs::read_dir(root.join("swarms")) {
            for entry in entries.flatten() {
                if !entry.path().is_dir() {
                    continue;
                }
                let Some(slug) = entry.file_name().to_str().map(ToOwned::to_owned) else {
                    continue;
                };
                let swarm = Swarm::open(Arc::clone(&spec), &root, &slug).await?;
                swarms.insert(slug, Arc::new(swarm));
            }
        }

        Ok(Self {
            spec,
            root,
            swarms: RwLock::new(swarms),
        })
    }

    /// Opens a swarm, or returns the one already open.
    pub async fn open(&self, slug: &str) -> Result<Arc<Swarm>, Refused> {
        if let Some(open) = self.swarms.read().await.get(slug) {
            return Ok(Arc::clone(open));
        }
        let swarm = Arc::new(Swarm::open(Arc::clone(&self.spec), &self.root, slug).await?);
        self.swarms
            .write()
            .await
            .insert(slug.to_owned(), Arc::clone(&swarm));
        Ok(swarm)
    }

    /// One open swarm.
    pub async fn get(&self, slug: &str) -> Result<Arc<Swarm>, Refused> {
        self.swarms
            .read()
            .await
            .get(slug)
            .map(Arc::clone)
            .ok_or_else(|| Refused::View(format!("no swarm `{slug}`")))
    }

    /// Every open swarm, by slug.
    pub async fn slugs(&self) -> Vec<String> {
        self.swarms.read().await.keys().cloned().collect()
    }

    /// Every open swarm, for the trigger to walk.
    pub async fn all(&self) -> Vec<Arc<Swarm>> {
        self.swarms.read().await.values().map(Arc::clone).collect()
    }

    /// The specification, for anything that needs to read the model.
    pub fn spec(&self) -> &Spec {
        &self.spec
    }

    /// The entities a canvas draws: every entity the specification declares.
    ///
    /// Not a list kept here. A canvas that draws what the model declares gains a node kind when the
    /// model does, and a hand-kept list is the thing that would stop that being true.
    pub fn drawable_entities(&self) -> Vec<String> {
        self.spec
            .ir()
            .entities()
            .keys()
            .map(ToString::to_string)
            .collect()
    }

    /// What this system is, as the UI needs to know it.
    pub fn describe(&self) -> Description {
        let ir = self.spec.ir();
        Description {
            system: ir.system().to_string(),
            entities: ir
                .entities()
                .iter()
                .map(|(name, entity)| EntityShape {
                    name: name.to_string(),
                    identity: entity.identity.name.as_str().to_owned(),
                    states: entity
                        .lifecycle
                        .states
                        .iter()
                        .map(|state| state.as_str().to_owned())
                        .collect(),
                    fields: entity
                        .fields
                        .iter()
                        .map(|field| field.name.as_str().to_owned())
                        .collect(),
                })
                .collect(),
            commands: ir
                .commands()
                .iter()
                .map(|(name, command)| CommandShape {
                    name: name.to_string(),
                    input: command
                        .input
                        .iter()
                        .map(|field| InputShape {
                            name: field.name.as_str().to_owned(),
                            optional: field.type_ref.is_optional(),
                        })
                        .collect(),
                })
                .collect(),
            views: ir.views().keys().map(ToString::to_string).collect(),
        }
    }
}

/// The shape of the system, for a UI that would rather read it than be told it.
#[derive(Debug, Serialize)]
pub struct Description {
    pub system: String,
    pub entities: Vec<EntityShape>,
    pub commands: Vec<CommandShape>,
    pub views: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EntityShape {
    pub name: String,
    pub identity: String,
    pub states: Vec<String>,
    pub fields: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CommandShape {
    pub name: String,
    pub input: Vec<InputShape>,
}

#[derive(Debug, Serialize)]
pub struct InputShape {
    pub name: String,
    pub optional: bool,
}
