use crate::cid::Cid;
use crate::request::{self, Client};
use crate::url::Url;
use crate::util::random_choice;
use axum::{
    Router,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, head, post},
};
use serde::Deserialize;
use std::collections::HashSet;
use std::{fs, io::Write};
use std::{path::PathBuf, time::Duration};
use tokio::sync::mpsc;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;
use url::Origin;

#[derive(Clone)]
pub struct ServerConfig {
    /// The address the server should listen on
    pub addr: String,
    /// The directory where content-addressed files will be stored
    pub dir: PathBuf,
    pub allow_post: bool,
    /// Allow federating with any peer?
    pub allow_all: bool,
    pub notify: Vec<Url>,
    pub allow: HashSet<Origin>,
    pub deny: HashSet<Origin>,
}

impl ServerConfig {
    pub fn new(
        addr: String,
        dir: PathBuf,
        allow_post: bool,
        allow_all: bool,
        notify: Vec<Url>,
        allow: Vec<Url>,
        deny: Vec<Url>,
    ) -> Self {
        ServerConfig {
            addr,
            dir,
            allow_post,
            allow_all,
            notify,
            allow: HashSet::from_iter(allow.into_iter().map(|url| url.origin())),
            deny: HashSet::from_iter(deny.into_iter().map(|url| url.origin())),
        }
    }
}

#[derive(Clone)]
struct ServerState {
    config: ServerConfig,
    pub client: Client,
    pub notification_sender: mpsc::Sender<NotifyTask>,
}

impl ServerState {
    pub fn new(
        config: ServerConfig,
        client: Client,
        notification_sender: mpsc::Sender<NotifyTask>,
    ) -> Self {
        ServerState {
            config,
            client,
            notification_sender,
        }
    }
}

#[derive(Debug)]
pub enum ServerError {
    IoError(std::io::Error),
    RequestError(reqwest::Error),
    FileNotFound,
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::RequestError(err) => write!(f, "RequestError({})", err),
            ServerError::IoError(err) => write!(f, "IoError({})", err),
            ServerError::FileNotFound => write!(f, "File not found"),
        }
    }
}

impl std::error::Error for ServerError {}

impl From<reqwest::Error> for ServerError {
    fn from(err: reqwest::Error) -> Self {
        ServerError::RequestError(err)
    }
}

impl From<std::io::Error> for ServerError {
    fn from(err: std::io::Error) -> Self {
        ServerError::IoError(err)
    }
}

/// Multithread server (number of threads = number of CPUs)
#[tokio::main(flavor = "multi_thread")]
pub async fn serve(config: ServerConfig) {
    // Create the file storage directory if it doesn't exist
    std::fs::create_dir_all(&config.dir).expect("Unable to create file storage directory");

    // Setup tracing (logs)
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    // Create message channel for background tasks
    let (notification_sender, notification_receiver) = mpsc::channel::<NotifyTask>(1024);

    // Create HTTP client
    let client =
        request::build_client(Duration::from_secs(2)).expect("Could not create HTTP client");

    let addr = config.addr.clone();
    let worker_client = client.clone();
    let worker_notify_peers = config.notify.clone();

    let state = ServerState::new(config, client, notification_sender);

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
    notify_peers: Vec<Url>,
) {
    tracing::info!("Notification worker started");
    // Process notifications until the channel is closed
    while let Some(task) = receiver.recv().await {
        // Select up to 12 random peers to notify
        let selected_peers = random_choice(notify_peers.clone(), 12);

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

    let file_path = state.config.dir.join(&cid.to_string());

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

    let file_path = state.config.dir.join(&cid.to_string());

    if file_path.exists() {
        (StatusCode::OK, "").into_response()
    } else {
        (StatusCode::NOT_FOUND, "File not found").into_response()
    }
}

/// Should we notify this peer?
/// - Deny list is always honored
/// - Otherwise, notifications are restricted to allow list unless allow_all is true
pub fn should_notify(
    peer: &Url,
    allow: &HashSet<url::Origin>,
    deny: &HashSet<url::Origin>,
    allow_all: bool,
) -> bool {
    let peer_origin = peer.origin();
    // Always honor deny list
    if deny.contains(&peer_origin) {
        return false;
    }
    // If peer is not in the deny list, and we allow all, return true
    if allow_all {
        return true;
    }
    // Otherwise check against allow list
    allow.contains(&peer_origin)
}

async fn post_notify(State(state): State<ServerState>, headers: HeaderMap) -> Response {
    // Get CID from request header
    let peer = match headers
        .get("magnetize-peer")
        .and_then(|s| s.to_str().ok())
        .and_then(|s| Url::parse(s).ok())
    {
        Some(origin) => origin,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "magnetize-peer header missing or invalid",
            )
                .into_response();
        }
    };

    // If peer is not in the list of known peers, do nothing
    if !should_notify(
        &peer,
        &state.config.allow,
        &state.config.deny,
        state.config.allow_all,
    ) {
        return (StatusCode::BAD_REQUEST, "Untrusted peer").into_response();
    }

    let cid = match headers
        .get("magnetize-cid")
        .and_then(|s| s.to_str().ok())
        .and_then(|s| Cid::parse(s).ok())
    {
        Some(cid) => cid,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "magnetize-cid header missing or invalid",
            )
                .into_response();
        }
    };

    let file_path = state.config.dir.join(&cid.to_string());

    // Exit early if we already know about this CID
    if file_path.exists() {
        return (StatusCode::OK, "Resource exists").into_response();
    }

    let Ok(url) = peer.join(&cid.to_string()) else {
        return (StatusCode::BAD_REQUEST, "Invalid URL").into_response();
    };

    // Fetch the file from the origin
    let Ok(body) = request::get_cid(&state.client, &url, &cid).await else {
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

    // Add notification task to the queue
    // If queue is full, drop the task
    if let Err(err) = state.notification_sender.try_send(NotifyTask {
        cid: cid.clone(),
        url: url.clone(),
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
    if state.config.allow_post == false {
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
        let path = PathBuf::from(state.config.dir).join(&cid_string);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_notify() {
        let peer = Url::parse("https://example.com/resource").unwrap();
        let peer2 = Url::parse("https://allowed.com/resource").unwrap();
        let peer3 = Url::parse("https://denied.com/resource").unwrap();

        let allow = HashSet::from_iter(vec![
            url::Url::parse("https://allowed.com").unwrap().origin(),
        ]);

        let deny = HashSet::from_iter(vec![
            url::Url::parse("https://denied.com").unwrap().origin(),
        ]);

        // Test with allow_all = true
        assert!(
            should_notify(&peer, &allow, &deny, true),
            "When allow_all is true, non-denied peer should be notified"
        );

        assert!(
            should_notify(&peer2, &allow, &deny, true),
            "When allow_all is true, allowed peer should be notified"
        );

        assert!(
            !should_notify(&peer3, &allow, &deny, true),
            "When allow_all is true, denied peer should not be notified"
        );

        // Test with allow_all = false
        assert!(
            !should_notify(&peer, &allow, &deny, false),
            "When allow_all is false, non-allowed peer should not be notified"
        );

        assert!(
            should_notify(&peer2, &allow, &deny, false),
            "When allow_all is false, allowed peer should be notified"
        );

        assert!(
            !should_notify(&peer3, &allow, &deny, false),
            "When allow_all is false, denied peer should not be notified"
        );
    }
}
