//! What the server holds: the specification, and every swarm open against it.
//!
//! One specification for all of them. A swarm is data, not a program — two swarms differ in what
//! their logs contain and in nothing else, which is what makes "the spec is the system" true rather
//! than aspirational.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tokio::sync::RwLock;

use ess_runtime::Spec;

use crate::budget::{Caps, Reached};
use crate::swarm::{Refused, Summary, Swarm, now};

/// The entity a slug names one of. Its lifecycle decides what "finished" means.
const SWARM: &str = "swarm.manager.Swarm";

/// How long one swarm gets to answer what it holds before the list goes on without it.
const LIST_BUDGET: Duration = Duration::from_millis(50);

/// Whether a slug is a name this server will put on a path or in the handle map.
///
/// One rule, used by `open`, `get` and `remove` alike, because an entry point that accepts a name
/// another refuses is how a check is walked around: `./busy` and `busy` are one directory and two
/// map keys, and a removal reached the second while the refusal looked at the first.
///
/// A slug must be exactly one ordinary path component, spelled the way it is stored. `..`, `.`,
/// anything with a separator in it, an empty name and a leading dot are all refused —
/// `story:slug-is-not-validated` is the general home for this rule, and it is open; this is the
/// part of it a destructive verb cannot wait for.
pub fn check(slug: &str) -> Result<(), Removal> {
    let bad = |why: &str| {
        Err(Removal::BadSlug {
            slug: slug.to_owned(),
            why: why.to_owned(),
        })
    };
    if slug.is_empty() {
        return bad("a slug cannot be empty");
    }
    if slug.starts_with('.') {
        return bad("a slug cannot start with a dot");
    }
    if slug.contains('\0') {
        return bad("a slug cannot contain a NUL");
    }
    let mut parts = std::path::Path::new(slug).components();
    match (parts.next(), parts.next()) {
        (Some(std::path::Component::Normal(only)), None) if only == std::ffi::OsStr::new(slug) => {
            Ok(())
        }
        _ => bad("a slug has to be one ordinary path component"),
    }
}

/// Whether one `swarm.manager.Swarm` record is finished: its state can be read, and the
/// specification calls that state an end.
///
/// One question, asked by the removal and by the listing alike. They used to spell it separately
/// and disagreed about a record whose `state` was missing or was not a string — the removal
/// dropped it and counted the slug removable, the listing kept it and counted the slug live. No
/// record has that shape today, which is exactly why it was worth settling before one does: an
/// unreadable state is not evidence of being finished, and the reading that refuses a removal is
/// the one that cannot erase a log by accident.
fn finished(record: &serde_json::Value, terminal: &[String]) -> bool {
    record
        .get("state")
        .and_then(|state| state.as_str())
        .is_some_and(|state| terminal.iter().any(|end| end == state))
}

/// The log, everything SQLite keeps beside it, and then the directory — in that order.
///
/// The three files together: removing the database and leaving the `-wal` behind makes the next
/// open of this slug run WAL recovery against a fresh empty file.
fn unlink(directory: &std::path::Path) -> Result<(), Removal> {
    let database = directory.join("eventlog.sqlite3");
    for file in [
        database.clone(),
        with_suffix(&database, "-wal"),
        with_suffix(&database, "-shm"),
    ] {
        match std::fs::remove_file(&file) {
            Ok(()) => {}
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            Err(why) => {
                return Err(Removal::Undeletable {
                    path: file.display().to_string(),
                    why: why.to_string(),
                });
            }
        }
    }

    match std::fs::remove_dir_all(directory) {
        Ok(()) => Ok(()),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(Removal::Undeletable {
            path: directory.display().to_string(),
            why: why.to_string(),
        }),
    }
}

/// The whole server.
pub struct Server {
    spec: Arc<Spec>,
    root: PathBuf,
    swarms: RwLock<BTreeMap<String, Arc<Swarm>>>,
    /// One lock per slug, held across the construction of that slug's swarm, so at most one
    /// `Swarm::open` per slug runs at a time in this `Server`.
    ///
    /// The map's own lock cannot do this job: it has to be dropped across the await, which is what
    /// let two opens of one slug both construct a handle. It also cannot be left to the store to
    /// arbitrate — the event log sets no `busy_timeout` (`ess-runtime/src/store.rs:21-23`), so two
    /// simultaneous opens of one `eventlog.sqlite3` do not queue, one of them is refused outright.
    ///
    /// Per slug and not one lock for all of them, because `Swarm::open` is a store open plus a full
    /// replay of the log: one lock made an open of a slug with no directory, no log and nothing in
    /// common with another slug wait 253 ms for that other slug's 4000-command replay. Two clients
    /// creating two different swarms (`http.rs`, the only caller) have no reason to queue.
    ///
    /// The outer lock is `std` and is held only long enough to clone one `Arc` out of the map —
    /// never across an await. One entry per slug ever opened, which is bounded by the swarms on
    /// disk; nothing removes them, because a slug that was opened once may be opened again.
    opening: Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// Swarms a removal has evicted whose files are still on disk, because something else was
    /// still holding a handle when the removal ran.
    ///
    /// Unlinking under a live handle is not a filesystem problem on this platform — it succeeds —
    /// it is a HONESTY problem: the holder's writes keep landing in the unlinked inode and keep
    /// answering `created`, so a client is told a command was applied that no reader of that slug
    /// can ever see. Measured. Truncating the files instead kills the process: SQLite has the
    /// `-shm` mapped, and a write to a truncated mapping is a SIGBUS (measured, signal 7).
    ///
    /// So a removal that cannot have the files to itself parks the handle here, and the files go
    /// when the last holder lets go — `sweep` does it, at the next entry point anybody uses. An
    /// `open` of the slug before that takes the entry back and returns the SAME handle, which is
    /// not a courtesy: constructing a second one over a log the first is still writing to is the
    /// two-worlds-over-one-log defect the `opening` gate exists to prevent.
    pending: Mutex<BTreeMap<String, Arc<Swarm>>>,
    /// When this process started, RFC 3339.
    started_at: String,
    /// When the trigger will next fire, RFC 3339. `None` until the trigger has said.
    next_tick_at: Mutex<Option<String>>,
    /// How many times the trigger has fired since start.
    ticks: Mutex<u64>,
    /// Goals whose coordinator turn is running right now, as `slug/goal_id`.
    in_flight: Mutex<HashSet<String>>,
    /// Goals the loop has stopped asking about, keyed `slug/goal_id`. Holding the reason here and
    /// not only on the stream is what lets a page that connected afterwards still say which cap.
    capped: Mutex<BTreeMap<String, CappedGoal>>,
    /// What one goal may use up.
    caps: Caps,
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
                // The fourth door. A directory whose name is not a slug this server will accept
                // would be opened here and then be unreachable through `get` and unremovable
                // through `remove`, which is one rule at three doors and a different one at the
                // last.
                if let Err(why) = check(&slug) {
                    tracing::warn!(%slug, error = %why, "a directory under swarms/ is not a swarm name; not opened");
                    continue;
                }
                let swarm = Swarm::open(Arc::clone(&spec), &root, &slug).await?;
                swarms.insert(slug, Arc::new(swarm));
            }
        }

        Ok(Self {
            spec,
            root,
            swarms: RwLock::new(swarms),
            opening: Mutex::new(BTreeMap::new()),
            pending: Mutex::new(BTreeMap::new()),
            started_at: now(),
            next_tick_at: Mutex::new(None),
            ticks: Mutex::new(0),
            in_flight: Mutex::new(HashSet::new()),
            capped: Mutex::new(BTreeMap::new()),
            caps: Caps::configured(),
        })
    }

    /// Opens a swarm, or returns the one already open.
    ///
    /// Every call for one slug returns the same handle, however many run at once — and a caller
    /// that arrives while another is still constructing waits for it rather than being refused.
    ///
    /// Opening is a read of the map, an await on `Swarm::open`, then an insert, and a second open
    /// of the same slug could finish inside that await: both missed the read, both constructed a
    /// handle, and the later insert replaced the earlier handle in the map. A handle can hold the
    /// only copy of an owed at-least-once delivery, so the replaced one took that delivery with it
    /// — no `give_up`, no `What::Undelivered`, no log line. Two handles over one event log is also
    /// two in-memory worlds, which disagree from the first command either one applies.
    ///
    /// So construction is serialised per slug on `opening`, and the map is re-read under that
    /// slug's lock: the second caller finds the first's handle and never constructs one, so there
    /// is no losing handle to drop and no insert that can land over a live one. That also answers
    /// the refusal the store would otherwise hand back: with no `busy_timeout` set
    /// (`ess-runtime/src/store.rs:21-23`), two simultaneous opens of one `eventlog.sqlite3` end
    /// with one refused `database is locked`, which reached `POST /swarms` as a `500` for a swarm
    /// that was open and healthy. Only one open of a given file is now in flight at a time.
    ///
    /// The lock is per slug, so an open of one swarm never waits on another's replay.
    ///
    /// When the store fails, the error is returned.
    pub async fn open(&self, slug: &str) -> Result<Arc<Swarm>, Refused> {
        // Before the map, before the gate, before anything is joined onto a path.
        check(slug).map_err(|why| Refused::Command(why.to_string()))?;

        if let Some(open) = self.swarms.read().await.get(slug) {
            return Ok(Arc::clone(open));
        }

        // Anything a previous removal could not finish. Run with no gate held, which is what keeps
        // this from being able to deadlock against another sweep.
        self.sweep().await;

        // This slug's construction lock, and nobody else's. Taken under an `std` guard that is
        // dropped on the next line, so no await happens while it is held.
        let gate = Arc::clone(
            self.opening
                .lock()
                .expect("not poisoned")
                .entry(slug.to_owned())
                .or_default(),
        );
        let _constructing = gate.lock().await;

        // Whoever held this slug's gate before this call was opening this slug, so it is in the map
        // now and nothing is constructed twice.
        if let Some(open) = self.swarms.read().await.get(slug) {
            return Ok(Arc::clone(open));
        }

        // A removal that has not finished still owns this slug's files, and its handle is still
        // being written through. Take it back rather than building a second handle over the same
        // log: the removal is cancelled by the open, and `sweep` will see the slug in the map and
        // leave the files alone. Claimed under this slug's gate, which is the same gate `sweep`
        // takes before it unlinks, so exactly one of the two wins.
        // Bound in its own statement: the `std` guard has to be dropped before the await below,
        // or this future stops being `Send` and every handler that calls it stops compiling.
        let parked = self.pending.lock().expect("not poisoned").remove(slug);
        if let Some(parked) = parked {
            self.swarms
                .write()
                .await
                .insert(slug.to_owned(), Arc::clone(&parked));
            return Ok(parked);
        }

        let swarm = Arc::new(Swarm::open(Arc::clone(&self.spec), &self.root, slug).await?);
        self.swarms
            .write()
            .await
            .insert(slug.to_owned(), Arc::clone(&swarm));
        Ok(swarm)
    }

    /// Removes a swarm: its handle, its log, and the directory it lived in.
    ///
    /// Removable means **absent from the model or finished**. A place made by `POST /swarms` that
    /// no `CreateSwarm` ever followed has no `swarm.manager.Swarm` record at all, and nothing in
    /// the model can be asked about it — that is the swarm the operator could not remove and
    /// removed by hand. A slug whose records are all in a terminal state is finished. Anything
    /// else is refused with the error the specification already declares for a command that acts
    /// from a state the instance is not in.
    ///
    /// **ANY live record refuses it, not "the" state.** A slug's log can hold several
    /// `swarm.manager.Swarm` records — `CreateSwarm` with a fresh `tmux_session` is accepted
    /// however many are already there — and `Swarm::summary` keeps only the last one it walks. A
    /// slug holding a `Running` record and a `Deleted` one was removable whenever the `Deleted`
    /// one happened to sort last by minted id.
    ///
    /// Which states are terminal is read from the specification, not written here: `manager.yaml`
    /// says `terminal: [Deleted]`, and a lifecycle that grows a second end should not need this
    /// file edited to agree with it.
    ///
    /// Four things this has to get right:
    ///
    /// 1. **The slug is a single path component**, checked before it is joined onto anything. This
    ///    is the only route in the server that destroys, and an unchecked `{slug}` from the wire is
    ///    an erase-any-directory primitive: `DELETE /swarms/..%2Fescaped` is one segment on the
    ///    wire and `../escaped` in the handler. It is also what made the state check skippable —
    ///    `./busy` and `busy` are one directory and two map keys, so an alias reached the files
    ///    while the refusal looked at nothing.
    /// 2. **The per-slug `opening` gate is taken**, the same one `Server::open` takes. Removal and
    ///    construction of one slug are the same critical section: without it an open that missed
    ///    the map read constructs a handle while this is unlinking and inserts it afterwards,
    ///    leaving a live handle over a log that is gone. Measured: red in round 0 of
    ///    `a_removal_racing_an_open_leaves_the_map_agreeing_with_the_disk` without it.
    /// 3. **The handle leaves the map and every `Arc` is gone before the files go.** Dropping the
    ///    last one closes the SQLite handle. When something else still holds one, the files do NOT
    ///    go now — see `pending`; they go in `sweep`, when the last holder lets go.
    /// 4. **`eventlog.sqlite3`, `-wal` and `-shm` go together.** Removing the database and leaving
    ///    the WAL behind makes the next open of this slug run recovery against a fresh empty file.
    pub async fn remove(&self, slug: &str) -> Result<Removed, Removal> {
        check(slug)?;

        // Anything an earlier removal could not finish, first, and with no gate held.
        self.sweep().await;

        // The same gate `open` takes, and for the same reason: at most one of "construct this
        // slug" and "remove this slug" runs at a time.
        let gate = self.gate(slug);
        let _removing = gate.lock().await;

        // EVERY decision below is taken against this one lookup, and `handle_of` is the only way
        // this method finds a handle. Branching on `swarms` alone is what made a retried removal
        // unlink under a live holder: a parked slug is not in `swarms`, so the state check never
        // ran and the park branch was never reached, and `202 Accepted` — an invitation to retry —
        // was the ordinary way to get there.
        let held = self.handle_of(slug).await;
        let directory = self.root.join("swarms").join(slug);
        if held.is_none() && !directory.exists() {
            return Err(Removal::NotHere(slug.to_owned()));
        }

        // What the model says, if the model has been told this exists at all. Every record is
        // asked, not the one `summary()` happens to keep — and a record written through a PARKED
        // handle is a record, so this runs for a parked slug exactly as it does for a served one.
        if let Some(swarm) = &held
            && let Some(state) = self.live_state(swarm).await
        {
            return Err(Removal::StateConflict {
                slug: slug.to_owned(),
                state,
            });
        }

        // Out of both maps: from here nothing new can be handed this handle, and the reference
        // count below means the same thing whichever map it came from — our clone, plus holders.
        self.swarms.write().await.remove(slug);
        self.pending.lock().expect("not poisoned").remove(slug);

        if let Some(swarm) = held {
            // Our own clone plus whatever else is out there. Anything above one is a holder that
            // is still writing through this handle, and unlinking under it would leave that writer
            // being told `created` for records no reader of this slug could ever reach.
            let outstanding = Arc::strong_count(&swarm) > 1;
            if outstanding {
                self.pending
                    .lock()
                    .expect("not poisoned")
                    .insert(slug.to_owned(), swarm);
                return Ok(Removed::WhenReadersLetGo);
            }
            drop(swarm);
        }

        unlink(&directory)?;
        Ok(Removed::Now)
    }

    /// One slug's handle, from wherever this server is keeping it.
    ///
    /// **There are two places a handle lives and this function is the only thing that knows it.**
    /// `swarms` holds the ones being served; `pending` holds the ones a removal has evicted and
    /// could not unlink yet, and a handle in `pending` is no less live — it is there *because*
    /// something is still writing through it.
    ///
    /// A decision taken against one map while the handle sits in the other is not a near-miss, it
    /// is the whole defect: the second `remove` of a parked slug found `None`, skipped the state
    /// refusal, skipped the park, and unlinked under the holder.
    ///
    /// **If a third place for a handle is ever added to `Server`, it is added here.** That is not
    /// left to a reader's diligence: `a_handle_can_only_live_where_this_lookup_looks` reads the
    /// fields of `Server` out of this file and fails if one holds `Arc<Swarm>` and is not named in
    /// this function's body.
    async fn handle_of(&self, slug: &str) -> Option<Arc<Swarm>> {
        if let Some(served) = self.swarms.read().await.get(slug) {
            return Some(Arc::clone(served));
        }
        self.pending
            .lock()
            .expect("not poisoned")
            .get(slug)
            .map(Arc::clone)
    }

    /// Finishes every removal whose holders have let go.
    ///
    /// Called at the entry points rather than on a timer, and always with no gate held: it takes
    /// one slug's gate at a time and never two, so it cannot be half of a cycle.
    async fn sweep(&self) {
        let parked: Vec<String> = self
            .pending
            .lock()
            .expect("not poisoned")
            .keys()
            .cloned()
            .collect();

        for slug in parked {
            let gate = self.gate(&slug);
            let _sweeping = gate.lock().await;

            // Something opened it again while the removal was pending. The open took the handle
            // back and the files belong to a live swarm now; the removal is off.
            if self.swarms.read().await.contains_key(&slug) {
                self.pending.lock().expect("not poisoned").remove(&slug);
                continue;
            }

            let ready = {
                let mut pending = self.pending.lock().expect("not poisoned");
                match pending.get(&slug) {
                    // One reference, and it is this map's. Nobody is writing through it.
                    Some(swarm) if Arc::strong_count(swarm) == 1 => pending.remove(&slug),
                    _ => None,
                }
            };
            if ready.is_some() {
                drop(ready);
                let _ = unlink(&self.root.join("swarms").join(&slug));
            }
        }
    }

    /// This slug's construction lock, and nobody else's. Taken under an `std` guard that is
    /// dropped before it is returned, so no await happens while it is held.
    fn gate(&self, slug: &str) -> Arc<tokio::sync::Mutex<()>> {
        Arc::clone(
            self.opening
                .lock()
                .expect("not poisoned")
                .entry(slug.to_owned())
                .or_default(),
        )
    }

    /// The state of any record this swarm holds that the specification does not call an end, or
    /// `None` when it holds none — either because there are no records or because all of them are
    /// finished.
    ///
    /// Every record, because a slug can hold more than one and `summary()` keeps only the last.
    async fn live_state(&self, swarm: &Swarm) -> Option<String> {
        let terminal = self.terminal_states();
        swarm
            .instances(SWARM)
            .await
            .iter()
            .find(|record| !finished(record, &terminal))
            .map(|record| {
                record
                    .get("state")
                    .and_then(|state| state.as_str())
                    // A record that refuses a removal without being able to say which state it is
                    // in is still a record that refuses it.
                    .unwrap_or("unreadable")
                    .to_owned()
            })
    }

    /// The states the specification calls ends for a swarm.
    ///
    /// Read from the compiled IR rather than spelled here, so `terminal: [Deleted]` remains the one
    /// place that decides it.
    fn terminal_states(&self) -> Vec<String> {
        self.spec
            .ir()
            .entities()
            .iter()
            .find(|(name, _)| name.to_string() == SWARM)
            .map(|(_, entity)| {
                entity
                    .lifecycle
                    .terminal
                    .iter()
                    .map(|state| state.as_str().to_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// One open swarm.
    ///
    /// The slug is checked here too, and not only because this is a map lookup that cannot escape
    /// anything: `./busy` and `busy` are one directory and two map keys, and one entry point that
    /// accepts a name another one refuses is how a check gets walked around rather than passed.
    /// One rule, at every door.
    ///
    /// A handle handed out here outlives this call, and a removal that runs in between does not
    /// take it back. That is what `pending` is for: the files of a swarm somebody still holds are
    /// not unlinked, so a write through an old handle is either refused or readable afterwards —
    /// never answered `created` into a log nothing can reach.
    pub async fn get(&self, slug: &str) -> Result<Arc<Swarm>, Refused> {
        check(slug).map_err(|why| Refused::Command(why.to_string()))?;
        self.swarms
            .read()
            .await
            .get(slug)
            .map(Arc::clone)
            .ok_or_else(|| Refused::View(format!("no swarm `{slug}`")))
    }

    /// Every open swarm, by slug. Terminal ones included: this is what the process holds.
    pub async fn slugs(&self) -> Vec<String> {
        self.swarms.read().await.keys().cloned().collect()
    }

    /// Every swarm a client is shown, by slug — which is every open one that is not finished.
    ///
    /// `DeleteSwarm` promises the swarm "no longer appears in the swarms list"
    /// (`manager.yaml:326`), and for as long as the list was the keys of the handle map that
    /// promise was simply false: six swarms sat in `Deleted` and all six were listed. Terminal is
    /// the specification's word, read from the lifecycle, not a name matched here.
    ///
    /// Separate from `slugs()` rather than a filter inside it: what the process HOLDS and what a
    /// client is SHOWN are two different questions, and boot logs the first.
    pub async fn listed(&self) -> Vec<String> {
        self.sweep().await;

        let terminal = Arc::new(self.terminal_states());
        // The map's lock is dropped before the first await: reading a swarm's records takes that
        // swarm's own world lock, and holding the map across that is how two locks become an order
        // to get wrong.
        let held: Vec<(String, Arc<Swarm>)> = self
            .swarms
            .read()
            .await
            .iter()
            .map(|(slug, swarm)| (slug.clone(), Arc::clone(swarm)))
            .collect();

        // Concurrently, and with a budget. Asking each swarm in turn took 219 ms for ten swarms
        // under command traffic against 9 µs for the keys this list used to be, because the world
        // lock a record is read under is the same one `issue` holds across its commit and `tick`
        // holds across its cascade. `GET /swarms` runs on every page load; one busy swarm must not
        // delay the whole list.
        //
        // A swarm that does not answer inside the budget is LISTED. Being busy is not being
        // finished, and a list that hides what it could not read would hide a live swarm.
        let mut asking = Vec::with_capacity(held.len());
        for (slug, swarm) in held {
            let terminal = Arc::clone(&terminal);
            asking.push(tokio::spawn(async move {
                let done = tokio::time::timeout(LIST_BUDGET, async {
                    let records = swarm.instances(SWARM).await;
                    !records.is_empty() && records.iter().all(|record| finished(record, &terminal))
                })
                .await
                .unwrap_or(false);
                (slug, done)
            }));
        }

        let mut shown = Vec::new();
        for asked in asking {
            match asked.await {
                Ok((slug, false)) => shown.push(slug),
                Ok((_, true)) => {}
                // A panicked task is not a reason to drop a swarm out of the list silently.
                Err(why) => tracing::warn!(error = %why, "a swarm could not be asked for the list"),
            }
        }
        shown.sort();
        shown
    }

    /// Every open swarm, for the trigger to walk.
    ///
    /// The trigger holds these `Arc`s for a whole tick — a coordinator turn included — which is
    /// exactly the case `pending` exists for: a removal during a turn evicts the handle and the
    /// files go here, at the start of the next tick, once that Vec has dropped.
    pub async fn all(&self) -> Vec<Arc<Swarm>> {
        self.sweep().await;
        self.swarms.read().await.values().map(Arc::clone).collect()
    }

    /// The specification, for anything that needs to read the model.
    pub fn spec(&self) -> &Spec {
        &self.spec
    }

    /// The trigger says when it will next fire, and that it just did.
    pub fn tick_scheduled(&self, in_: Duration) {
        let at = time::OffsetDateTime::now_utc() + in_;
        *self.next_tick_at.lock().expect("not poisoned") = at
            .format(&time::format_description::well_known::Rfc3339)
            .ok();
    }

    /// One more firing of the trigger.
    pub fn ticked(&self) {
        *self.ticks.lock().expect("not poisoned") += 1;
    }

    /// Claims a goal for a coordinator turn. `false` when one is already running for it, which is
    /// the `overlap: serial_per_instance` the binding declares, enforced here by the host.
    pub fn claim_turn(&self, slug: &str, goal_id: &str) -> bool {
        self.in_flight
            .lock()
            .expect("not poisoned")
            .insert(format!("{slug}/{goal_id}"))
    }

    /// The turn is over, whichever way.
    pub fn release_turn(&self, slug: &str, goal_id: &str) {
        self.in_flight
            .lock()
            .expect("not poisoned")
            .remove(&format!("{slug}/{goal_id}"));
    }

    /// How many coordinator turns are running right now.
    pub fn turns_in_flight(&self) -> usize {
        self.in_flight.lock().expect("not poisoned").len()
    }

    /// What one goal may use up.
    pub fn caps(&self) -> Caps {
        self.caps
    }

    /// Records that a goal has been capped. `false` when it already was, so the report is made
    /// once rather than every period.
    pub fn report_capped(&self, capped: CappedGoal) -> bool {
        self.capped
            .lock()
            .expect("not poisoned")
            .insert(format!("{}/{}", capped.swarm, capped.goal), capped)
            .is_none()
    }

    /// Forgets a cap report, so raising a cap reports the next one afresh.
    pub fn forget_capped(&self, slug: &str, goal_id: &str) {
        self.capped
            .lock()
            .expect("not poisoned")
            .remove(&format!("{slug}/{goal_id}"));
    }

    /// Every goal the loop has stopped asking about, with why.
    pub fn capped_goals(&self) -> Vec<CappedGoal> {
        self.capped
            .lock()
            .expect("not poisoned")
            .values()
            .cloned()
            .collect()
    }

    /// Every periodic binding, with the period the specification declares for it.
    pub fn periods(&self) -> Vec<(String, Duration)> {
        self.spec
            .ir()
            .bindings()
            .iter()
            .filter_map(|(name, binding)| {
                let periodic = binding.cause.periodic()?;
                Some((
                    name.to_string(),
                    Duration::from_secs(u64::from(periodic.contract.every.seconds())),
                ))
            })
            .collect()
    }

    /// Where the runtime is: what it runs, since when, what it drives, and every swarm in one row.
    pub async fn status(&self) -> Status {
        let mut swarms = Vec::new();
        for swarm in self.all().await {
            let mut summary = swarm.summary().await;
            summary.watchers = swarm.watchers();
            let events = swarm.count().await.unwrap_or_default();
            swarms.push(SwarmStatus { summary, events });
        }
        // What a server with no environment and no config will actually run — not what
        // `SWARM_COORDINATOR` says. `configured()` read only that variable, so after resolution
        // gained a default this endpoint answered `configured: false` for a server that was about
        // to spend money every thirty seconds. That is the same defect the resolution change fixed,
        // one layer up: a reader being told "nothing is configured" when something will run.
        let coordinator = crate::coordinator::fallback();
        Status {
            system: self.spec.ir().system().to_string(),
            spec_files: self.spec.files(),
            started_at: self.started_at.clone(),
            now: now(),
            periodic: self
                .periods()
                .into_iter()
                .map(|(binding, every)| Periodic {
                    binding,
                    every_s: every.as_secs(),
                })
                .collect(),
            next_tick_at: self.next_tick_at.lock().expect("not poisoned").clone(),
            ticks: *self.ticks.lock().expect("not poisoned"),
            caps: self.caps,
            capped: self.capped_goals(),
            coordinator: Coordinator {
                configured: true,
                program: Some(coordinator.describe()),
                source: coordinator.source.to_string(),
                in_flight: self.turns_in_flight(),
            },
            swarms,
        }
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
                    // Which states are ends. A canvas draws a finished instance differently, and
                    // deriving that from a name would be guessing at what the lifecycle declares.
                    terminal: entity
                        .lifecycle
                        .terminal
                        .iter()
                        .map(|state| state.as_str().to_owned())
                        .collect(),
                    fields: entity
                        .fields
                        .iter()
                        .map(|field| field.name.as_str().to_owned())
                        .collect(),
                    // What this entity points at, and through which field. A canvas draws these as
                    // edges: the specification already says a goal references the swarm it belongs
                    // to, so nothing has to be told that twice.
                    relations: entity
                        .relations
                        .iter()
                        .map(|relation| RelationShape {
                            name: relation.name.clone(),
                            target: relation.target.name().to_string(),
                            via: relation.via.clone(),
                            owns: matches!(relation.kind, ess_domain::entity::RelationKind::Owns),
                        })
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

/// The same path with something appended to its file name: `x.sqlite3` and `-wal` is
/// `x.sqlite3-wal`, which is not what `set_extension` would give.
fn with_suffix(path: &std::path::Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// What a removal did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removed {
    /// Handle evicted, files unlinked, directory gone.
    Now,
    /// Handle evicted and the slug is no longer served, but something else still holds the handle,
    /// so the files stay until it lets go. `sweep` finishes it at the next entry point anybody
    /// uses. Unlinking under a live handle succeeds on this platform and is exactly the lie this
    /// avoids: the holder's writes keep answering `created` into a log nothing can read back.
    WhenReadersLetGo,
}

/// Why a swarm was not removed.
#[derive(Debug)]
pub enum Removal {
    /// Not a name this server will put on a path or in the map.
    BadSlug { slug: String, why: String },
    /// No handle and no directory: there is nothing here by that name.
    NotHere(String),
    /// The model holds a record, and it is not finished. The specification declares this refusal
    /// for every command that acts from a state the instance is not in; removal is not a command,
    /// but it is the same answer to the same question and inventing a second name for it would put
    /// two words in front of a client for one fact.
    StateConflict { slug: String, state: String },
    /// The files would not go.
    Undeletable { path: String, why: String },
}

impl std::fmt::Display for Removal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadSlug { slug, why } => write!(f, "`{slug}` is not a swarm name: {why}"),
            Self::NotHere(slug) => write!(f, "no swarm `{slug}`"),
            Self::StateConflict { slug, state } => write!(
                f,
                "`{slug}` is {state}, and only a swarm with no record or in a terminal state is removed"
            ),
            Self::Undeletable { path, why } => write!(f, "{path} could not be removed: {why}"),
        }
    }
}

impl std::error::Error for Removal {}

/// One goal the loop has stopped asking about.
#[derive(Clone, Debug, Serialize)]
pub struct CappedGoal {
    pub swarm: String,
    pub goal: String,
    pub turns: u64,
    pub spent_usd: Option<f64>,
    pub reached: Reached,
    pub why: String,
}

/// Where the runtime is.
#[derive(Debug, Serialize)]
pub struct Status {
    pub system: String,
    pub spec_files: usize,
    pub started_at: String,
    pub now: String,
    pub periodic: Vec<Periodic>,
    pub next_tick_at: Option<String>,
    pub ticks: u64,
    /// What one goal may use up before the loop stops asking.
    pub caps: Caps,
    /// Every goal the loop has stopped asking about, with why.
    pub capped: Vec<CappedGoal>,
    pub coordinator: Coordinator,
    pub swarms: Vec<SwarmStatus>,
}

#[derive(Debug, Serialize)]
pub struct Periodic {
    pub binding: String,
    pub every_s: u64,
}

#[derive(Debug, Serialize)]
pub struct Coordinator {
    /// Always true: resolution ends at a default, so something always runs. Kept because the UI
    /// reads it, and because `false` would now be a lie rather than a state.
    pub configured: bool,
    pub program: Option<String>,
    /// Which of the three sources decided it — the swarm's config, the environment, or the default.
    pub source: String,
    /// Turns running right now, across every swarm.
    pub in_flight: usize,
}

#[derive(Debug, Serialize)]
pub struct SwarmStatus {
    #[serde(flatten)]
    pub summary: Summary,
    /// Events in the log.
    pub events: u64,
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
    pub terminal: Vec<String>,
    pub fields: Vec<String>,
    pub relations: Vec<RelationShape>,
}

/// One edge the specification declares between two entities.
#[derive(Debug, Serialize)]
pub struct RelationShape {
    pub name: String,
    pub target: String,
    /// The field holding the other instance's identity.
    pub via: String,
    /// `owns` rather than `references`: the target does not outlive this one.
    pub owns: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The residue pass 2 left: `live_state` dropped a record whose `state` could not be read and
    /// counted the slug removable, while `listed()` twelve lines away treated the same record as
    /// live. Nobody can produce that record shape today — every state the runtime writes is a
    /// string the lifecycle declares — so this is not a bug, it is two rules for one field, which
    /// is how it becomes one.
    ///
    /// The rule, written once: **a record is finished only if its state can be read AND the
    /// specification calls that state an end.** Unreadable is not evidence of being finished, and
    /// the safe reading is the one that refuses a removal rather than the one that erases a log.
    #[test]
    fn a_record_whose_state_cannot_be_read_counts_as_live() {
        let terminal = vec!["Deleted".to_owned()];

        assert!(finished(
            &serde_json::json!({"state": "Deleted"}),
            &terminal
        ));
        assert!(!finished(
            &serde_json::json!({"state": "Running"}),
            &terminal
        ));
        assert!(
            !finished(&serde_json::json!({"state": null}), &terminal),
            "a null state is not a terminal state",
        );
        assert!(
            !finished(&serde_json::json!({}), &terminal),
            "a record with no state at all is not a finished record",
        );
        assert!(
            !finished(&serde_json::json!({"state": 7}), &terminal),
            "a state that is not a string is not a terminal state",
        );
    }

    /// And both readers ask that one question, rather than each spelling its own.
    #[test]
    fn the_removal_and_the_listing_read_the_state_field_the_same_way() {
        let source = include_str!("state.rs");
        for (name, body) in [
            ("live_state", body_of(source, "async fn live_state")),
            ("listed", body_of(source, "pub async fn listed")),
        ] {
            assert!(
                body.contains("finished("),
                "`{name}` reads the state field its own way instead of asking `finished`, which \
                 is how one field ends up with two rules",
            );
        }
    }

    /// The text of a function, from its signature to the next one at the same indent.
    fn body_of<'a>(source: &'a str, signature: &str) -> &'a str {
        let at = source
            .find(signature)
            .unwrap_or_else(|| panic!("`{signature}` is no longer in this file"));
        let rest = &source[at..];
        let end = rest[1..]
            .find("\n    }\n")
            .unwrap_or_else(|| panic!("`{signature}` does not end where a method ends"));
        &rest[..end]
    }
}
