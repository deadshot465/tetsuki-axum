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
    let mut args: Vec<String> = vec![
        "-p".into(),
        payload.word_count.to_string(),
        payload.keyword.clone(),
    ];

    if let Some(ref novel) = payload.novel {
        args.push("-n".into());
        args.push(novel.clone());
    } else {
        args.push("--merge".into());
    }

    if let Some(ref instructions) = payload.additional_instructions {
        args.push(instructions.clone());
    }

    let result = tokio::spawn(async move {
        Command::new("claude")
            .args(args)
            .current_dir(format!("./{}", NOVEL_DIRECTORY))
            .spawn()
            .map_err(|e| tracing::error!("Failed to start child process: {}", e))
            .expect("Failed to start child process.")
            .wait_with_output()
            .await
    })
    .await;

    if let Ok(output) = result
        .map_err(|e| tracing::error!("Error happened when joining the child process: {}", e))
        .and_then(|r| r.map_err(|e| tracing::error!("Failed to get output: {}", e)))
    {
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
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ServerError::with_message(
                "Failed to invoke Claude for codex summary.",
            )),
        )
            .into_response()
    }
}
