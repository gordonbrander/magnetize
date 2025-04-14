use crate::cid::Cid;
use crate::request;
use axum::{
    Router,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, head, post},
};
use serde::Deserialize;
use std::{fs, io::Write};
use std::{path::PathBuf, time::Duration};

#[derive(Clone)]
pub struct ServerState {
    /// The address the server should listen on
    pub address: String,
    /// The directory where content-addressed files will be stored
    pub file_storage_dir: PathBuf,
    pub allow_post: bool,
    pub client: reqwest::Client,
}

impl ServerState {
    pub fn new(address: String, file_storage_dir: PathBuf, allow_post: bool) -> Self {
        let client =
            request::build_client(Duration::from_secs(2)).expect("Could not create HTTP client");
        ServerState {
            address,
            file_storage_dir,
            allow_post,
            client,
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
pub async fn serve(state: ServerState) {
    // Create the file storage directory if it doesn't exist
    std::fs::create_dir_all(&state.file_storage_dir)
        .expect("Unable to create file storage directory");

    // Build our application with routes
    let app = Router::new()
        .route("/", get(get_index))
        .route("/", post(post_file))
        .route("/{filename}", get(get_file))
        .route("/{filename}", head(head_file))
        .with_state(state.clone());

    // Run the server
    let listener = tokio::net::TcpListener::bind(&state.address)
        .await
        .expect("Unable to bind server to address");

    println!("Server listening on {}", &state.address);

    axum::serve(listener, app)
        .await
        .expect("Unable to start server");
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
async fn get_file(
    State(state): State<ServerState>,
    Path(filename): Path<String>,
    query: Query<GetCidParams>,
) -> Response {
    // Only allow GET requests for valid CIDs
    let Ok(cid) = Cid::parse(&filename) else {
        return (StatusCode::BAD_REQUEST, "Invalid CID").into_response();
    };

    let file_path = state.file_storage_dir.join(&cid.to_string());

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
async fn head_file(State(state): State<ServerState>, Path(filename): Path<String>) -> Response {
    // Only allow GET requests for valid CIDs
    let Ok(cid) = Cid::parse(&filename) else {
        return (StatusCode::BAD_REQUEST, "Invalid CID").into_response();
    };

    let file_path = state.file_storage_dir.join(&cid.to_string());

    if file_path.exists() {
        (StatusCode::OK, "").into_response()
    } else {
        (StatusCode::NOT_FOUND, "File not found").into_response()
    }
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
        let path = PathBuf::from(state.file_storage_dir).join(&cid_string);
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
