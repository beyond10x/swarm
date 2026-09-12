//! Serves one swarm manager.
//!
//! Boot is three things: compile the specification, open every swarm already on disk, and start the
//! trigger that drives whatever periodic bindings the specification declares. The specification is
//! read at startup and held — so what this server does is decided by `src/core/`, and a command
//! added there is reachable here without this crate being touched.

use std::path::PathBuf;
use std::sync::Arc;

use ess_runtime::Spec;

use swarm_server::state::Server;
use swarm_server::{http, trigger};

/// Where the specification lives, relative to the repository root.
const SPEC: &str = "src/core";
/// Where the swarms live.
const DATA: &str = "data";
/// The port to listen on, unless `SWARM_PORT` says otherwise.
const PORT: u16 = 5000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "swarm_server=info,tower_http=warn".into()),
        )
        .init();

    let root = repository_root();
    let spec_at = root.join(SPEC);
    let data_at = root.join(DATA);

    // A specification that does not resolve stops the server here rather than at the first request.
    // The alternative is a server that starts and then refuses everything, which is a worse way to
    // learn the same thing.
    let spec = Spec::load(&spec_at)?;
    tracing::info!(
        system = %spec.ir().system(),
        files = spec.files(),
        at = %spec_at.display(),
        "specification compiled"
    );

    let server = Arc::new(Server::start(spec, data_at.clone()).await?);
    let open = server.slugs().await;
    tracing::info!(swarms = ?open, at = %data_at.display(), "swarms opened");

    tokio::spawn(trigger::run(Arc::clone(&server)));

    let port: u16 = std::env::var("SWARM_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(PORT);

    let app = http::routes(server)
        // The canvas is served by vite in development, on another origin.
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    tracing::info!(port, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}

/// The repository root, whether this was started from the workspace or from the crate.
fn repository_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is src/runtime/swarm-server; the root is three levels up.
    let from_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize();

    match from_manifest {
        Ok(root) if root.join(SPEC).is_dir() => root,
        _ => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    }
}
