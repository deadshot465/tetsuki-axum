use crate::model::app_state::AppState;
use crate::model::claim::Claim;
use crate::model::errors::ServerError;
use crate::model::novel::{
    CodexSummaryContainerResponse, CodexSummaryRequest, CodexSummaryResponseLogs,
};
use crate::shared::configuration::CONFIGURATION;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bollard::Docker;
use bollard::config::ContainerCreateBody;
use bollard::container::LogOutput;
use bollard::models::HostConfig;
use bollard::query_parameters::LogsOptionsBuilder;
use futures::StreamExt;
use std::collections::HashMap;

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

    let docker = connect_to_docker_socket();

    if docker.is_none() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ServerError::with_message(
                "Failed to connect to Docker socket.",
            )),
        )
            .into_response();
    }

    let docker = docker.expect("Failed to connect to docker.");

    let config = ContainerCreateBody {
        image: Some("ubuntu:latest".into()),
        host_config: Some(HostConfig {
            network_mode: Some(CONFIGURATION.docker_network_name.clone()),
            ..Default::default()
        }),
        volumes: Some(vec![
            "/root/.local/bin/claude:/usr/local/bin/claude".to_string(),
            "/root/Novel:/root/Novel".to_string(),
            "/root/.claude:/root/.claude".to_string(),
        ]),
        entrypoint: Some(vec!["claude".to_string(), "-p".to_string()]),
        cmd: Some(vec![prompt]),
        working_dir: Some("/root/Novel".into()),
        ..Default::default()
    };

    let mut error_messages = vec![];
    let mut container_id = String::new();
    let mut container_id_clone = String::new();

    match docker
        .create_container(None, config)
        .await
        .map_err(|e| {
            let msg = format!("Failed to create container: {}", e);
            error_messages.push(msg);
        })
        .map(|res| {
            container_id.push_str(&res.id);
            container_id_clone.push_str(&res.id);
            docker.start_container(&container_id_clone, None)
        }) {
        Err(_) => {
            let error_messages = error_messages.join("\n");
            tracing::error!("{}", &error_messages);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ServerError::with_message(error_messages)),
            )
                .into_response()
        }
        Ok(response) => {
            if let Err(e) = response.await {
                let msg = format!("Failed to start container: {}", e);
                error_messages.push(msg);
                let error_messages = error_messages.join("\n");
                tracing::error!("{}", &error_messages);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ServerError::with_message(error_messages)),
                )
                    .into_response()
            } else {
                (
                    StatusCode::CREATED,
                    Json(CodexSummaryContainerResponse { container_id }),
                )
                    .into_response()
            }
        }
    }
}

pub async fn get_summary_container_result(
    _claim: Claim,
    Path(container_id): Path<String>,
    State(_): State<AppState>,
) -> Response {
    let docker = connect_to_docker_socket();

    if docker.is_none() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ServerError::with_message(
                "Failed to connect to Docker socket.",
            )),
        )
            .into_response();
    }

    let docker = docker.expect("Failed to connect to Docker.");

    match docker.inspect_container(&container_id, None).await {
        Err(e) => {
            let error_message = format!("Failed to inspect container: {}", e);
            tracing::error!("{}", &error_message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ServerError::with_message(error_message)),
            )
                .into_response()
        }
        Ok(res) => {
            let mut payload = HashMap::new();
            payload.insert("state", res.state);
            (StatusCode::OK, Json(payload)).into_response()
        }
    }
}

pub async fn get_summary_result(
    _claim: Claim,
    Path(container_id): Path<String>,
    State(_): State<AppState>,
) -> Response {
    let docker = connect_to_docker_socket();

    if docker.is_none() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ServerError::with_message(
                "Failed to connect to Docker socket.",
            )),
        )
            .into_response();
    }

    let docker = docker.expect("Failed to connect to Docker.");

    let log_options = LogsOptionsBuilder::default()
        .stdout(true)
        .stderr(true)
        .tail("100")
        .build();

    let results = docker
        .logs(&container_id, Some(log_options))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>();

    if let Err(e) = docker.remove_container(&container_id, None).await {
        tracing::error!("Failed to remove container: {}", e);
    }

    match results {
        Err(e) => {
            let error_message = format!("Failed to get log output: {}", e);
            tracing::error!("{}", &error_message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ServerError::with_message(error_message)),
            )
                .into_response()
        }
        Ok(payload) => {
            let mut response = CodexSummaryResponseLogs::default();

            for p in payload.into_iter() {
                match p {
                    LogOutput::StdErr { message } => {
                        let bytes = message.into_iter().collect::<Vec<_>>();
                        response
                            .errors
                            .push(String::from_utf8(bytes).unwrap_or_default());
                    }
                    LogOutput::StdOut { message } => {
                        let bytes = message.into_iter().collect::<Vec<_>>();
                        response
                            .outs
                            .push(String::from_utf8(bytes).unwrap_or_default());
                    }
                    LogOutput::StdIn { message } => {
                        let bytes = message.into_iter().collect::<Vec<_>>();
                        response
                            .ins
                            .push(String::from_utf8(bytes).unwrap_or_default());
                    }
                    LogOutput::Console { message } => {
                        let bytes = message.into_iter().collect::<Vec<_>>();
                        response
                            .console
                            .push(String::from_utf8(bytes).unwrap_or_default());
                    }
                }
            }

            (StatusCode::OK, Json(response)).into_response()
        }
    }
}

fn connect_to_docker_socket() -> Option<Docker> {
    Docker::connect_with_socket_defaults()
        .map_err(|e| tracing::error!("Failed to connect to Docker socket: {}", e))
        .ok()
}
