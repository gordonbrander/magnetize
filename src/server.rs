use crate::cid::Cid;
use crate::db::{Database, OriginStatus};
use crate::request::{self, Client};
use crate::url::Url;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, head, post},
};
use rand;
use serde::{Deserialize, Serialize};
use std::fs;
use std::{path::PathBuf, time::Duration};
use tokio::sync::mpsc;
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
    pub notification_sender: mpsc::Sender<NotifyTask>,
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

    // Create message channel for background tasks
    let (notification_sender, notification_receiver) = mpsc::channel::<NotifyTask>(1024);

    // Create HTTP client
    let client =
        request::build_client(Duration::from_secs(2)).expect("Could not create HTTP client");

    let addr = config.addr.clone();
    let worker_client = client.clone();
    let worker_db_path = config.db.clone();

    let url = Url::parse(&config.url).expect("Server URL is invalid");

    let state = ServerState {
        dir: config.dir,
        fed_all: config.fed_all,
        db: config.db,
        url,
        client,
        notification_sender,
    };

    // Build our application with routes
    let app = Router::new()
        .route("/", get(get_index))
        .route("/notify", post(post_notify))
        .route("/{cid}", get(get_cid))
        .route("/{cid}", head(head_cid))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state);

    // Spawn worker
    let worker_db = Database::open(&worker_db_path).expect("Unable to open database file");
    tokio::spawn(notify_worker(
        notification_receiver,
        worker_db,
        worker_client,
    ));

    // Run the server
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Unable to bind server to address");

    tracing::info!(addr = &addr, "server listening");

    axum::serve(listener, app)
        .await
        .expect("Unable to start server");
}

/// Sleep for a random duration between 0 and `max` milliseconds.
async fn sleep_jitter(max: Duration) {
    let jitter = rand::random_range(Duration::from_secs(0)..max);
    tokio::time::sleep(jitter).await;
}

// Task to notify a peer
#[derive(Debug, Clone)]
struct NotifyTask {
    cid: Cid,
    url: Url,
}

// Worker that processes background tasks
async fn notify_worker(
    mut receiver: mpsc::Receiver<NotifyTask>,
    mut db: Database,
    client: reqwest::Client,
) {
    tracing::info!("Notification worker started");

    // Process notifications until the channel is closed
    while let Some(task) = receiver.recv().await {
        // Choose N random feds to notify
        let selected_feds = match db.choose_random_notify(12) {
            Ok(feds) => feds,
            Err(err) => {
                tracing::error!(err = format!("{}", err), "Unable to choose feds");
                continue;
            }
        };

        // Add some jitter (random delay) to spread out traffic spikes in the network and prevent congestion
        // See: <https://aws.amazon.com/builders-library/timeouts-retries-and-backoff-with-jitter/>
        // See: <https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/>
        sleep_jitter(Duration::from_millis(500)).await;

        for peer in selected_feds {
            match request::post_notify(&client, &peer, &task.url, &task.cid).await {
                Ok(_) => {
                    tracing::info!(
                        peer = &peer.to_string(),
                        cid = &task.cid.to_string(),
                        "Notified peer"
                    );
                }
                Err(err) => {
                    tracing::info!(
                        err = format!("{}", err),
                        peer = peer.to_string(),
                        "Failed to notify peer"
                    );
                }
            }
        }
    }

    tracing::info!("Notification worker shutting down");
}

// Handler for GET /
async fn get_index() -> Response {
    (StatusCode::OK, "GET /{CID}").into_response()
}

#[derive(Deserialize)]
struct GetCidParams {
    dn: Option<String>,
}

// Handler for GET /CID
async fn get_cid(
    State(state): State<ServerState>,
    Path(cid): Path<String>,
    query: Query<GetCidParams>,
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

async fn post_notify(State(state): State<ServerState>, headers: HeaderMap) -> Response {
    let ws = match headers
        .get("ws")
        .and_then(|s| s.to_str().ok())
        .and_then(|s| Url::parse(s).ok())
    {
        Some(origin) => origin,
        None => {
            return (StatusCode::BAD_REQUEST, "ws header missing or invalid").into_response();
        }
    };

    // Get CID from request header
    let cid = match headers
        .get("cid")
        .and_then(|s| s.to_str().ok())
        .and_then(|s| Cid::parse(s).ok())
    {
        Some(cid) => cid,
        None => {
            return (StatusCode::BAD_REQUEST, "cid header missing or invalid").into_response();
        }
    };

    let file_path = state.dir.join(&cid.to_string());

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

    // Get trust status of fed
    let fed_status = match db.read_origin_status(&ws) {
        Ok(status) => status,
        Err(err) => {
            tracing::error!("Failed to read origin status: {}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, "").into_response();
        }
    };

    // Always deny if fed is on deny list
    if fed_status == OriginStatus::Deny {
        return (StatusCode::BAD_REQUEST, "Untrusted origin").into_response();
    }

    // Deny if fed is unknown and fed_all is false
    if fed_status == OriginStatus::Unknown && state.fed_all == false {
        return (StatusCode::BAD_REQUEST, "Untrusted origin").into_response();
    }

    // Otherwise, we're either on allow list or fed_all is true

    // Fetch the file from the origin
    let Ok(body) = request::get_and_check_cid(&state.client, &ws, &cid).await else {
        return (StatusCode::BAD_REQUEST, format!("CID not found {}", &cid)).into_response();
    };

    // Write bytes to storage
    let Ok(_) = std::fs::write(&file_path, body) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create resource",
        )
            .into_response();
    };

    // Construct URL pointing to the resource on our peer
    let Ok(our_ws) = state.url.join(&cid.to_string()) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to create URL to resource",
        )
            .into_response();
    };

    // Add notification task to the queue
    // If queue is full, drop the task
    if let Err(err) = state.notification_sender.try_send(NotifyTask {
        cid: cid.clone(),
        url: our_ws,
    }) {
        tracing::error!(
            err = format!("{}", err),
            "Failed to queue notification task"
        );
    }

    // Return the CID
    (StatusCode::CREATED, format!("{}", &cid)).into_response()
}
