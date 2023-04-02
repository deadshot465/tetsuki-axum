use crate::model::dialog_info::DialogInfo;
use crate::shared::constants::ASSET_DIRECTORY;
use crate::shared::web_driver::get_dialog;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use once_cell::sync::Lazy;
use std::collections::HashMap;

const BACKGROUNDS_PATH: &str = "/dialog/images/backgrounds";
const CHARACTERS_PATH: &str = "/dialog/images/characters";

static BACKGROUNDS_LIST: Lazy<Vec<String>> = Lazy::new(|| build_list(BACKGROUNDS_PATH));
static CHARACTERS_LIST: Lazy<Vec<String>> = Lazy::new(|| build_list(CHARACTERS_PATH));

pub async fn generate_dialog(Json(dialog_info): Json<DialogInfo>) -> Response {
    if !CHARACTERS_LIST.contains(&dialog_info.character)
        || !BACKGROUNDS_LIST.contains(&dialog_info.background)
        || dialog_info.text.is_empty()
    {
        return (
            StatusCode::BAD_REQUEST,
            "The specified character/background doesn't exist, or the text is empty.",
        )
            .into_response();
    }

    match get_dialog(dialog_info).await {
        Ok(result) => (StatusCode::OK, result).into_response(),
        Err(e) => {
            tracing::error!("An error occurred when generating the dialog: {}", e);
            (StatusCode::BAD_REQUEST, e.to_string()).into_response()
        }
    }
}

pub async fn get_dialog_options() -> Response {
    let mut dialog_options = HashMap::new();
    dialog_options.insert("characters".to_string(), CHARACTERS_LIST.clone());
    dialog_options.insert("backgrounds".to_string(), BACKGROUNDS_LIST.clone());
    (StatusCode::OK, Json(dialog_options)).into_response()
}

fn build_list(path: &str) -> Vec<String> {
    let files_path = String::from(ASSET_DIRECTORY) + path;
    let files_directory = std::path::Path::new(&files_path);
    if !files_directory.exists() {
        tracing::error!("{} folder doesn't exist.", path);
        vec![]
    } else {
        std::fs::read_dir(files_directory)
            .and_then(|read_dir| {
                read_dir
                    .collect::<Vec<_>>()
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .map(|v| {
                        v.into_iter()
                            .map(|entry| entry.file_name().into_string().unwrap_or_default())
                            .map(|s| s.split('.').take(1).collect())
                            .collect::<Vec<_>>()
                    })
            })
            .unwrap_or_default()
    }
}
