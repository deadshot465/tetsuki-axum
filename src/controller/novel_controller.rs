use crate::model::app_state::AppState;
use crate::model::claim::Claim;
use crate::model::errors::ServerError;
use crate::model::novel::{CodexSummaryRequest, CodexSummaryResponse};
use crate::shared::constants::NOVEL_DIRECTORY;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tokio::process::Command;

pub async fn summarize_codex(
    _claim: Claim,
    State(_): State<AppState>,
    Json(payload): Json<CodexSummaryRequest>,
) -> Response {
    let mut prompt = format!(
        "/novel-codex-summary {} {}",
        payload.word_count, payload.keyword
    );

    if let Some(ref novel) = payload.novel {
        prompt.push_str(&format!(" -n {}", novel));
    } else {
        prompt.push_str("--merge");
    }

    if let Some(ref instructions) = payload.additional_instructions {
        prompt.push(' ');
        prompt.push_str(instructions);
    }

    let result = Command::new("claude")
        .arg("-p")
        .arg(prompt)
        .current_dir(format!("./{}", NOVEL_DIRECTORY))
        .output()
        .await;

    match result {
        Err(e) => {
            let error_message = format!("Failed to get output from Claude: {}", e);
            tracing::error!("{}", &error_message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ServerError::with_message(error_message)),
            )
                .into_response()
        }
        Ok(output) => {
            if !output.status.success() {
                let code = output.status.code();
                if let Ok(error_message) = String::from_utf8(output.stderr) {
                    let formatted_error_message = format!(
                        "Failed to get output from LLM: {}, Exit Code: {:?}",
                        error_message, code
                    );
                    tracing::error!("{}", &formatted_error_message);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ServerError::with_message(formatted_error_message)),
                    )
                        .into_response()
                } else {
                    let error_message = format!("Exit code: {:?}", code);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ServerError::with_message(error_message)),
                    )
                        .into_response()
                }
            } else {
                if let Ok(output_message) = String::from_utf8(output.stdout) {
                    (
                        StatusCode::CREATED,
                        Json(CodexSummaryResponse {
                            output: output_message,
                            image: Vec::new(),
                        }),
                    )
                        .into_response()
                } else {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ServerError::with_message(
                            "Claude invocation succeeded but encountered an error.".to_string(),
                        )),
                    )
                        .into_response()
                }
            }
        }
    }
}
