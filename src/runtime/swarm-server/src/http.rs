//! The HTTP surface: commands in, views out, changes streamed.
//!
//! Deliberately thin. Every route is one of three things — issue a command, read a view, or watch
//! what happens — because those are the three things the specification describes, and a fourth kind
//! of route would be this layer inventing a concept the model does not have.
//!
//! The routes are not generated from the specification, and that is a gap worth naming rather than
//! hiding: `ess generate --kind openapi` projects a surface from the same IR, and one day these
//! should be that. Today they are hand-written, so a command added to the specification is reachable
//! through `POST /swarms/:slug/commands/:command` without anybody touching this file, but a
//! purpose-built route for it is a thing somebody has to write.

use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::state::Server;
use crate::swarm::{Outgoing, Refused};

/// The body of a command request.
#[derive(Debug, Deserialize)]
pub struct Issue {
    /// The command's input, as the specification declares it.
    #[serde(default)]
    pub input: Map<String, Value>,
    /// Who is issuing it. `None` means the system itself, which skips the actor check.
    #[serde(default)]
    pub actor: Option<String>,
    /// The key this request commits under. It guards the APPEND, and it does not make re-issuing
    /// the command harmless.
    ///
    /// `store.rs` scopes the key to the INSTANCE'S STREAM: committing it twice on one stream with
    /// the same events appends nothing, and committing it with different events is refused. That
    /// is the whole of what it buys. `Swarm::issue` applies the command BEFORE it appends, against
    /// the world the first attempt left — so a client that resends after a dropped response gets
    /// the specification's answer to the second application, not a copy of the first. Measured:
    /// two `ActivateConfig` calls under one key answer `activated` then `wrong-state`. For a
    /// command that CREATES, the second attempt mints a fresh instance, so the key lands on a
    /// stream it was never spent on and nothing refuses it: two `DraftConfig` calls under one key
    /// leave two Configs. Both are in `tests/redelivery_under_attack.rs` under
    /// `story:request-key-is-not-idempotency`.
    ///
    /// So what a disconnected client gets is this and no more: it will not double-APPEND to a
    /// stream the key was already spent on. Everything else it must establish by reading, and a
    /// client that cannot tolerate the second answer should read before it resends. The log
    /// (`GET /swarms/{slug}/log`) carries the `request` each event was written under, so it
    /// answers exactly whether this key's first attempt landed. The canvas (`GET /swarms/{slug}`)
    /// answers the weaker question of whether the effect is there: a canvas record is an instance
    /// — entity, id, state, fields, revision — and names no key at all.
    ///
    /// Omitting the field gives up that guard. `issue_command` mints a fresh uuid when it is
    /// absent, and a key spent on no stream can refuse nothing, so a repeat appends. Measured:
    /// two `MoveBox` calls under one key append one `BoxMoved`, the same two under minted keys
    /// append two (`tests/the_request_key_contract.rs`). Sending a key of the client's own is the
    /// only way to have the guard at all; omitting it is honest about wanting neither.
    ///
    /// Changing that means giving `Swarm::issue` a request-to-answer record, which is a schema in
    /// `ess-runtime` and not a comment. Until then this doc and those two cases move together.
    #[serde(default)]
    pub request: Option<String>,
}

/// What a refusal looks like on the wire.
#[derive(Debug, Serialize)]
struct Problem {
    /// What was refused, in one phrase.
    error: String,
    /// Which kind of refusal it was, so a caller can tell a bad request from a broken store.
    kind: &'static str,
}

impl IntoResponse for Refused {
    fn into_response(self) -> Response {
        let (status, kind) = match &self {
            // The specification refused it: the caller asked for something the model does not admit.
            Self::Command(_) => (StatusCode::BAD_REQUEST, "command"),
            Self::View(_) => (StatusCode::NOT_FOUND, "view"),
            // These are ours, not the caller's.
            Self::Routing(_) => (StatusCode::INTERNAL_SERVER_ERROR, "routing"),
            Self::Store(_) => (StatusCode::INTERNAL_SERVER_ERROR, "store"),
        };
        let body = axum::Json(Problem {
            error: self.to_string(),
            kind,
        });
        (status, body).into_response()
    }
}

/// Every route this server serves.
pub fn routes(server: Arc<Server>) -> Router {
    Router::new()
        .route("/swarms", get(list_swarms).post(create_swarm))
        .route("/swarms/{slug}", get(read_swarm))
        .route("/swarms/{slug}/events", get(watch_swarm))
        .route("/swarms/{slug}/views/{view}", get(read_view))
        .route("/swarms/{slug}/commands/{command}", post(issue_command))
        .route("/swarms/{slug}/reload", post(reload_swarm))
        .route("/swarms/{slug}/log", get(read_log))
        .route("/swarms/{slug}/mail", post(send_mail))
        .route("/swarms/{slug}/turns", get(list_turns))
        .route("/swarms/{slug}/turns/{name}", get(read_turn))
        .route("/spec", get(read_spec))
        .route("/status", get(read_status))
        .with_state(server)
}

/// The swarms this server holds.
async fn list_swarms(State(server): State<Arc<Server>>) -> impl IntoResponse {
    axum::Json(server.slugs().await)
}

/// Opens a swarm's log, creating its directory if this is the first time.
///
/// This creates a PLACE for a swarm, not a swarm: the `swarm.manager.Swarm` record is made by
/// issuing `CreateSwarm`, like every other fact in the system. A directory is not a state machine.
async fn create_swarm(
    State(server): State<Arc<Server>>,
    axum::Json(body): axum::Json<NewSwarm>,
) -> Result<impl IntoResponse, Refused> {
    server.open(&body.slug).await?;
    Ok((StatusCode::CREATED, axum::Json(body)))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NewSwarm {
    pub slug: String,
}

/// Everything on one swarm's canvas: its instances, by entity.
async fn read_swarm(
    State(server): State<Arc<Server>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, Refused> {
    let swarm = server.get(&slug).await?;
    let mut canvas = Map::new();
    for entity in server.drawable_entities() {
        canvas.insert(entity.clone(), Value::Array(swarm.instances(&entity).await));
    }
    Ok(axum::Json(canvas))
}

/// One view, computed.
async fn read_view(
    State(server): State<Arc<Server>>,
    Path((slug, view)): Path<(String, String)>,
) -> Result<impl IntoResponse, Refused> {
    let swarm = server.get(&slug).await?;
    Ok(axum::Json(swarm.view(&view).await?))
}

/// Issues one command against one swarm.
async fn issue_command(
    State(server): State<Arc<Server>>,
    Path((slug, command)): Path<(String, String)>,
    axum::Json(body): axum::Json<Issue>,
) -> Result<impl IntoResponse, Refused> {
    let swarm = server.get(&slug).await?;
    let request = body
        .request
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let issued = swarm
        .issue(body.actor.as_deref(), &command, body.input, &request)
        .await?;
    Ok(axum::Json(issued))
}

/// Everything that happens to one swarm, as it happens.
///
/// A reader that falls further behind than the channel holds is dropped rather than served stale
/// history — it should reload the canvas, which is cheap, instead of being fed a gap it cannot see.
async fn watch_swarm(
    State(server): State<Arc<Server>>,
    Path(slug): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, Refused> {
    let swarm = server.get(&slug).await?;
    let changes = BroadcastStream::new(swarm.watch());

    let stream = changes.filter_map(|change| {
        let change = change.ok()?;
        let data = serde_json::to_string(&change).ok()?;
        Some(Ok(Event::default().data(data)))
    });

    Ok(Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default()))
}

/// Throws the in-memory world away and rebuilds it from the log.
///
/// The served world is a cache of the replay. This is how that claim is checked rather than
/// asserted — if a reload changes what the canvas shows, the cache was lying.
async fn reload_swarm(
    State(server): State<Arc<Server>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, Refused> {
    let swarm = server.get(&slug).await?;
    swarm.reload().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The most recent events in one swarm's log, oldest first.
///
/// This is the record itself, not the stream: a reader that connects late sees what happened
/// before it arrived, and a reader that doubts the stream can check it against this.
async fn read_log(
    State(server): State<Arc<Server>>,
    Path(slug): Path<String>,
    Query(page): Query<LogPage>,
) -> Result<impl IntoResponse, Refused> {
    let swarm = server.get(&slug).await?;
    Ok(axum::Json(swarm.history(page.limit.min(1000)).await?))
}

#[derive(Debug, Deserialize)]
pub struct LogPage {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    200
}

/// The body of a mail request.
#[derive(Debug, Deserialize)]
pub struct Mail {
    /// `agent` or `agent/mailbox`. Absent means broadcast.
    #[serde(default)]
    pub to: Option<String>,
    pub sender: String,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub reply_to: Option<String>,
}

/// Posts a message, to one mailbox or to every open one.
///
/// The host resolves the address, because a mailbox is addressed by the pair `(agent, name)` and
/// only something that can do a lookup can turn that into the id `PostMessage` takes.
async fn send_mail(
    State(server): State<Arc<Server>>,
    Path(slug): Path<String>,
    axum::Json(body): axum::Json<Mail>,
) -> Result<impl IntoResponse, Refused> {
    let swarm = server.get(&slug).await?;
    let mail = Outgoing {
        to: body.to.as_deref().unwrap_or_default(),
        sender: &body.sender,
        subject: &body.subject,
        body: &body.body,
        reply_to: body.reply_to.as_deref(),
    };
    let posted = match body.to.as_deref() {
        Some(_) => swarm.post(mail).await?,
        None => swarm.broadcast(mail).await?,
    };
    Ok(axum::Json(posted))
}

/// Every recorded coordinator run, newest first.
async fn list_turns(
    State(server): State<Arc<Server>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, Refused> {
    let swarm = server.get(&slug).await?;
    Ok(axum::Json(swarm.turns()))
}

/// One recorded coordinator run, every event.
async fn read_turn(
    State(server): State<Arc<Server>>,
    Path((slug, name)): Path<(String, String)>,
) -> Result<impl IntoResponse, Refused> {
    let swarm = server.get(&slug).await?;
    Ok(axum::Json(swarm.turn(&name)?))
}

/// Where the runtime is: uptime, the trigger, the coordinator, and every swarm in one row.
async fn read_status(State(server): State<Arc<Server>>) -> impl IntoResponse {
    axum::Json(server.status().await)
}

/// What the specification says this system is.
///
/// The UI reads this to know which entities exist, which commands may be issued and which views may
/// be read — so a command added to the specification becomes reachable without the UI being
/// redeployed, which is the whole reason the interpreter reads the model at boot.
async fn read_spec(State(server): State<Arc<Server>>) -> impl IntoResponse {
    axum::Json(server.describe())
}
