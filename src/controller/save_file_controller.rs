use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::borrow::Cow;

use crate::model::app_state::AppState;
use crate::model::claim::Claim;
use crate::shared::save_file::SaveFileRequest;

pub async fn save_file(
    _claim: Claim,
    State(_): State<AppState>,
    Json(save_file_request): Json<SaveFileRequest>,
) -> Response {
    let SaveFileRequest { filename, file_url } = save_file_request;

    let directory = std::fs::read_dir("./upload");
    match directory {
        Ok(read_dir) => {
            let filenames = read_dir
                .flat_map(|res| res.map(|entry| entry.file_name()))
                .map(|os_str| match os_str.to_string_lossy() {
                    Cow::Borrowed(s) => s.to_string(),
                    Cow::Owned(s) => s,
                })
                .collect::<Vec<_>>();

            if filenames.contains(&filename) {
                (
                    StatusCode::BAD_REQUEST,
                    "A file with exactly the same name already exists.",
                )
                    .into_response()
            } else {
                let payload = reqwest::get(file_url).await;

                match payload {
                    Ok(response) => {
                        let bytes = response.bytes().await;

                        match bytes {
                            Ok(bytes) => {
                                let bytes = bytes.to_vec();
                                let path = format!("./upload/{}", &filename);

                                if let Err(e) = std::fs::write(path, bytes) {
                                    let error_message =
                                        format!("Failed to store file on the server: {}", e);
                                    tracing::error!("{}", &error_message);
                                    (StatusCode::INTERNAL_SERVER_ERROR, error_message)
                                        .into_response()
                                } else {
                                    StatusCode::CREATED.into_response()
                                }
                            }
                            Err(e) => {
                                let error_message =
                                    format!("Failed to convert response to bytes: {}", e);
                                tracing::error!("{}", &error_message);
                                (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
                            }
                        }
                    }
                    Err(e) => {
                        let error_message = format!("Failed to get file from URL: {}", e);
                        tracing::error!("{}", &error_message);
                        (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
                    }
                }
            }
        }
        Err(e) => {
            let error_message = format!("Failed to read upload directory: {}", e);
            tracing::error!("{}", &error_message);
            (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
        }
    }
}
