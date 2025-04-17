use crate::cid::Cid;
use crate::db::{Database, OriginStatus};
use crate::magnet::MagnetLink;
use crate::request::{self, Client};
use crate::url::Url;
use axum::Json;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, head, post},
};
use rand;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{self, PathBuf};
use std::time::Duration;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// The address the server should listen on
    pub addr: String,
    /// Public-facing URL of the server
    pub url: String,
    /// The directory where content-addressed files will be stored
    pub dir: PathBuf,
    /// File path to SQLite database storing server state
    pub db: PathBuf,
    /// Allow federating with any peer?
    pub fed_all: bool,
}

#[derive(Clone)]
struct ServerState {
    pub url: Url,
    pub dir: PathBuf,
    pub db: PathBuf,
    pub fed_all: bool,
    pub client: Client,
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

    // Set up database
    Database::open(&config.db)
        .expect("Unable to open database")
        .migrate()
        .expect("Unable to set up database");

    // Create HTTP client
    let client =
        request::build_client(Duration::from_secs(2)).expect("Could not create HTTP client");

    let addr = config.addr.clone();
    let url = Url::parse(&config.url).expect("Server URL is invalid");

    let state = ServerState {
        dir: config.dir,
        fed_all: config.fed_all,
        db: config.db,
        url,
        client,
    };

    // Build our application with routes
    let app = Router::new()
        .route("/", get(get_index))
        .route("/", post(post_index))
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

/// Sleep for a random duration between 0 and `max` milliseconds.
async fn sleep_jitter(max: Duration) {
    let jitter = rand::random_range(Duration::from_secs(0)..max);
    tokio::time::sleep(jitter).await;
}

/// Notify other feds of content, issuing POST requests to their endpoint
async fn notify_feds(client: &reqwest::Client, db_path: &path::Path, magnet: &MagnetLink) {
    tracing::info!("Notifying feds");

    let mut db = match Database::open(db_path) {
        Ok(db) => db,
        Err(err) => {
            tracing::error!(err = format!("{}", err), "Unable to open database");
            return;
        }
    };

    // Choose N random feds to notify
    let selected_feds = match db.choose_random_notify(12) {
        Ok(feds) => feds,
        Err(err) => {
            tracing::error!(err = format!("{}", err), "Unable to choose feds");
            return;
        }
    };

    // Add some jitter (random delay) to spread out traffic spikes in the network and prevent congestion
    // See: <https://aws.amazon.com/builders-library/timeouts-retries-and-backoff-with-jitter/>
    // See: <https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/>
    sleep_jitter(Duration::from_millis(250)).await;

    for fed in selected_feds {
        match request::post_fed_cid_with_magnet(client, &fed, &magnet).await {
            Ok(_) => {
                tracing::info!(
                    fed = &fed.to_string(),
                    cid = &magnet.cid.to_string(),
                    "Notified fed"
                );
            }
            Err(err) => {
                tracing::info!(
                    err = format!("{}", err),
                    fed = fed.to_string(),
                    "Unable to notify fed"
                );
            }
        }
    }
}

async fn post_index(State(state): State<ServerState>, Json(magnet): Json<MagnetLink>) -> Response {
    let file_path = state.dir.join(&magnet.cid.to_string());

    // Exit early if we already know about this CID
    if file_path.exists() {
        return (StatusCode::OK, "Resource exists").into_response();
    }

    // Open database connection
    let db = match Database::open(&state.db) {
        Ok(db) => db,
        Err(err) => {
            tracing::error!("Failed to open database: {}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, "").into_response();
        }
    };

    for url in magnet.urls() {
        // Get trust status of fed
        let fed_status = match db.read_origin_status(&url) {
            Ok(status) => status,
            Err(err) => {
                tracing::error!("Failed to read origin status: {}", err);
                return (StatusCode::INTERNAL_SERVER_ERROR, "").into_response();
            }
        };

        // Always deny if fed is on deny list
        if fed_status == OriginStatus::Deny {
            tracing::info!(url = url.as_str(), "Fed on deny list. Denying.");
            continue;
        }

        // Deny if fed is unknown and fed_all is false
        if fed_status == OriginStatus::Unknown && state.fed_all == false {
            tracing::info!(
                url = url.as_str(),
                "Fed not on allow-list. Only federating with allow-listed feds. Skipping."
            );
            continue;
        }

        // Otherwise, URL is either on allow list or fed_all is true

        // Fetch the file from the origin
        let body = match request::get_and_check_cid(&state.client, &url, &magnet.cid).await {
            Ok(body) => body,
            Err(err) => {
                tracing::info!(err = format!("{}", err), "Request failed");
                continue;
            }
        };

        // Write bytes to storage
        let Ok(_) = std::fs::write(&file_path, body) else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create resource",
            )
                .into_response();
        };

        // Create a new magnet link with only the CID and our fed
        let mut our_magnet = MagnetLink::new(magnet.cid.clone());
        our_magnet.cdn.push(state.url);

        // Notify other feds about the new resource
        notify_feds(&state.client, &state.db, &our_magnet).await;

        // Return the CID
        return (StatusCode::CREATED, format!("{}", &our_magnet.cid)).into_response();
    }

    // Content not found on any trusted fed
    (StatusCode::BAD_GATEWAY, "").into_response()
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
