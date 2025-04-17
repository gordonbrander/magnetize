use crate::cid::Cid;
use crate::peers::{self, should_allow_peer};
use crate::request::{self, Client};
use crate::url::Url;
use crate::util::{open_or_create_file, random_choice};
use axum::{
    Router,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, head, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{fs, fs::File, io::Write};
use std::{path::PathBuf, time::Duration};
use tokio::sync::mpsc;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;
use url::Origin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// The address the server should listen on
    pub addr: String,
    pub url: String,
    /// The directory where content-addressed files will be stored
    pub dir: PathBuf,
    pub allow_post: bool,
    /// Allow federating with any peer?
    pub allow_all: bool,
    pub notify: PathBuf,
    pub allow: PathBuf,
    pub deny: PathBuf,
}

#[derive(Clone)]
struct ServerState {
    pub url: Url,
    /// The directory where content-addressed files will be stored
    pub dir: PathBuf,
    pub allow_post: bool,
    /// Allow federating with any peer?
    pub allow_all: bool,
    pub notify: PathBuf,
    pub allow: HashSet<Origin>,
    pub deny: HashSet<Origin>,
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

    // Create message channel for background tasks
    let (notification_sender, notification_receiver) = mpsc::channel::<NotifyTask>(1024);

    // Create HTTP client
    let client =
        request::build_client(Duration::from_secs(2)).expect("Could not create HTTP client");

    let addr = config.addr.clone();
    let worker_client = client.clone();
    let worker_notify_peers = config.notify.clone();

    let url = Url::parse(&config.url).expect("Server URL is invalid");

    let allow_file = open_or_create_file(&config.allow).expect("Could not open file");
    let allow: HashSet<Origin> = peers::read_valid_urls_from_lines(allow_file)
        .map(|url| url.origin())
        .collect();

    let deny_file = open_or_create_file(&config.deny).expect("Could not open file");
    let deny: HashSet<Origin> = peers::read_valid_urls_from_lines(deny_file)
        .map(|url| url.origin())
        .collect();

    let state = ServerState {
        dir: config.dir,
        allow_post: config.allow_post,
        allow_all: config.allow_all,
        notify: config.notify,
        url,
        allow,
        deny,
        client,
        notification_sender,
    };

    // Build our application with routes
    let app = Router::new()
        .route("/", get(get_index))
        .route("/", post(post_file))
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
    tokio::spawn(notify_worker(
        notification_receiver,
        worker_client,
        worker_notify_peers,
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
    client: reqwest::Client,
    notify_path: PathBuf,
) {
    tracing::info!("Notification worker started");
    // Process notifications until the channel is closed
    while let Some(task) = receiver.recv().await {
        let notify_file = match File::open(&notify_path) {
            Ok(file) => file,
            Err(err) => {
                tracing::error!(err = err.to_string(), "Unable to open notify file");
                continue;
            }
        };

        // Load peers from notify file
        let all_peers: Vec<Url> = peers::read_valid_urls_from_lines(notify_file).collect();

        // Select up to 12 random peers to notify
        let selected_peers = random_choice(all_peers, 12);

        // Add some jitter (random delay) to spread out traffic spikes in the network and prevent congestion
        // See: <https://aws.amazon.com/builders-library/timeouts-retries-and-backoff-with-jitter/>
        // See: <https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/>
        sleep_jitter(Duration::from_millis(500)).await;

        for peer in selected_peers {
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

    // Do we want to listen to notifications about this peer and fetch content from it?
    if !should_allow_peer(&ws, &state.allow, &state.deny, state.allow_all) {
        return (StatusCode::BAD_REQUEST, "Untrusted peer origin").into_response();
    }

    // Fetch the file from the origin
    let Ok(body) = request::get_and_check_cid(&state.client, &ws, &cid).await else {
        return (StatusCode::BAD_REQUEST, format!("CID not found {}", &cid)).into_response();
    };

    // Write bytes to file
    let Ok(_) = std::fs::write(&file_path, body) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create resource",
        )
            .into_response();
    };

    // Construct URL pointing to the resource
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

// Handler for POST /
async fn post_file(State(state): State<ServerState>, mut multipart: Multipart) -> Response {
    if state.allow_post == false {
        return (StatusCode::FORBIDDEN, "Uploads are not allowed").into_response();
    }

    while let Ok(Some(field)) = multipart.next_field().await {
        // Get the file data
        let data = match field.bytes().await {
            Ok(data) => data,
            Err(_) => return (StatusCode::BAD_REQUEST, "Failed to read file data").into_response(),
        };

        let cid = Cid::of(&data);
        let cid_string = cid.to_string();

        // Save the file with the hash as the name
        let path = PathBuf::from(state.dir).join(&cid_string);
        match std::fs::File::create(&path) {
            Ok(mut file) => {
                if let Err(_) = file.write_all(&data) {
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to write file")
                        .into_response();
                }
            }
            Err(_) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create file")
                    .into_response();
            }
        }

        return (StatusCode::CREATED, cid_string).into_response();
    }

    (StatusCode::BAD_REQUEST, "No file provided").into_response()
}
