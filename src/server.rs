use crate::cid::Cid;
use axum::{
    Router,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rand::rng;
use rand::seq::SliceRandom;
use std::path::{self, PathBuf};
use std::{fs, io::Write};
use tokio::{fs::File, io::AsyncReadExt};

#[derive(Clone)]
pub struct ServerState {
    /// The address the server should listen on
    pub address: String,
    /// The directory where content-addressed files will be stored
    pub file_storage_dir: PathBuf,
    /// A list of other CDNs to federate with
    pub feds: Vec<String>,
    pub allow_post: bool,
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

// Handler for GET /CID
async fn get_file(State(state): State<ServerState>, Path(filename): Path<String>) -> Response {
    match read_or_get_then_store_and_forward(&state.feds, &state.file_storage_dir, &filename).await
    {
        Ok(contents) => (StatusCode::OK, contents).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "File not found ").into_response(),
    }
}

/// Read file from storage, if it exists, otherwise, request it from feds, store it, and forward.
/// Tries each of the feds in order, returning the first successful response.
async fn read_or_get_then_store_and_forward(
    feds: &[String],
    dir: &path::Path,
    filename: &str,
) -> Result<Vec<u8>, ServerError> {
    let path = dir.join(filename);
    match File::open(&path).await {
        Ok(mut file) => {
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).await?;
            return Ok(contents);
        }
        Err(_) => {
            // Randomize the order of the feds to avoid overloading any one fed.
            let mut shuffled_feds = feds.to_vec();

            if !shuffled_feds.is_empty() {
                shuffled_feds.shuffle(&mut rng());
            }

            for fed in shuffled_feds {
                let url = format!("{}/{}", fed, filename);
                if let Ok(contents) = get_then_store_and_forward(&url, dir, filename).await {
                    return Ok(contents);
                }
            }
            Err(ServerError::FileNotFound)
        }
    }
}

/// Fetches a file from a URL and stores it in a directory, returning its bytes
async fn get_then_store_and_forward(
    url: &str,
    dir: &path::Path,
    filename: &str,
) -> Result<Vec<u8>, ServerError> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    let path = dir.join(filename);
    fs::write(path, &bytes)?;
    Ok(bytes.to_vec())
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
