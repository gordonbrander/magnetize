use crate::cid::Cid;
use axum::{
    Router,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::io::Write;
use std::path::PathBuf;
use tokio::{fs::File, io::AsyncReadExt};

#[derive(Clone)]
pub struct ServerState {
    pub address: String,
    pub file_storage_dir: PathBuf,
    pub allow_post: bool,
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
    let path = PathBuf::from(state.file_storage_dir).join(&filename);

    match File::open(&path).await {
        Ok(mut file) => {
            let mut contents = Vec::new();
            if let Err(_) = file.read_to_end(&mut contents).await {
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response();
            }

            (StatusCode::OK, contents).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "File not found").into_response(),
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
