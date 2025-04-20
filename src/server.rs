use crate::cid::Cid;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, head},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// The address the server should listen on
    pub addr: String,
    /// The directory where content-addressed files will be stored
    pub dir: PathBuf,
}

#[derive(Clone)]
struct ServerState {
    pub dir: PathBuf,
}

/// Multithread server (number of threads = number of CPUs)
#[tokio::main(flavor = "multi_thread")]
pub async fn serve(config: ServerConfig) {
    // Setup tracing (logs)
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    // Create the file storage directory if it doesn't exist
    std::fs::create_dir_all(&config.dir).expect("Unable to create file storage directory");

    let addr = config.addr.clone();

    let state = ServerState { dir: config.dir };

    // Build our application with routes
    let app = Router::new()
        .route("/", get(get_index))
        .route("/{cid}", get(get_cid))
        .route("/{cid}", head(head_cid))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state);

    // Run the server
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Unable to bind server to address");

    tracing::info!(addr = &addr, "server listening");

    axum::serve(listener, app)
        .await
        .expect("Unable to start server");
}

// Handler for GET /
async fn get_index() -> Response {
    (StatusCode::OK, "GET /{CID}").into_response()
}

#[derive(Deserialize)]
struct CidParams {
    dn: Option<String>,
}

// Handler for GET /CID
async fn get_cid(
    State(state): State<ServerState>,
    Path(cid): Path<String>,
    query: Query<CidParams>,
) -> Response {
    // Only allow GET requests for valid CIDs
    let Ok(cid) = Cid::parse(&cid) else {
        return (StatusCode::BAD_REQUEST, "Invalid CID").into_response();
    };

    let file_path = state.dir.join(&cid.to_string());

    // Read and return file contents if it exists
    // Include content-digest header.
    // See <https://www.ietf.org/archive/id/draft-ietf-httpbis-digest-headers-08.html>
    match fs::read(&file_path) {
        Ok(contents) => {
            let content_disposition = match query.dn {
                Some(ref dn) => format!("attachment; filename=\"{}\"", dn),
                None => format!("attachment"),
            };

            return (
                StatusCode::OK,
                [
                    (
                        "content-digest",
                        format!("cid=:{}:", cid.to_string()).as_str(),
                    ),
                    ("content-type", "application/octet-stream"),
                    ("content-disposition", &content_disposition),
                    ("content-length", contents.len().to_string().as_str()),
                ],
                contents,
            )
                .into_response();
        }
        Err(_) => {
            return (StatusCode::NOT_FOUND, "File not found").into_response();
        }
    }
}

// Handler for HEAD /CID
async fn head_cid(State(state): State<ServerState>, Path(cid): Path<String>) -> Response {
    // Only allow GET requests for valid CIDs
    let Ok(cid) = Cid::parse(&cid) else {
        return (StatusCode::BAD_REQUEST, "Invalid CID").into_response();
    };

    let file_path = state.dir.join(&cid.to_string());

    if file_path.exists() {
        (StatusCode::OK, "").into_response()
    } else {
        (StatusCode::NOT_FOUND, "File not found").into_response()
    }
}
