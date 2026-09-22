use crate::model::app_state::AppState;
use crate::model::claim::Claim;
use crate::model::errors::ServerError;
use crate::model::novel::{
    CodexSummaryContainerResponse, CodexSummaryRequest, CodexSummaryResponseLogs,
    CodexSummaryTrackItem, NovelEntityCardRecord,
};
use crate::model::swc::LinePushMessage;
use crate::shared::HTTP_CLIENT;
use crate::shared::configuration::CONFIGURATION;
use crate::shared::constants::NOVEL_ABSOULTE_PATH;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bollard::Docker;
use bollard::config::ContainerCreateBody;
use bollard::container::LogOutput;
use bollard::models::HostConfig;
use bollard::plugin::{ContainerState, ContainerStateStatusEnum};
use bollard::query_parameters::LogsOptionsBuilder;
use convert_case::ccase;
use dashmap::DashMap;
use futures::StreamExt;
use once_cell::sync::Lazy;
use sqlx::{Connection, SqliteConnection};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::time::{interval, sleep};

static ENTITY_CARDS_TRACK_MAP: Lazy<DashMap<String, CodexSummaryTrackItem>> =
    Lazy::new(DashMap::new);

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
        prompt.push_str(" --merge");
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
        image: Some("novel:latest".into()),
        host_config: Some(HostConfig {
            network_mode: Some(CONFIGURATION.docker_network_name.clone()),
            binds: Some(vec![
                "/root/.local/bin/claude:/usr/local/bin/claude:ro".to_string(),
                "/root/Novel:/root/Novel:rw".to_string(),
                "/root/.claude:/root/.claude:rw".to_string(),
            ]),
            ..Default::default()
        }),
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
                let current_records = get_latest_entity_card_records()
                    .await
                    .into_iter()
                    .map(|rec| (rec.id, rec))
                    .collect::<HashSet<_>>();

                let track_item = CodexSummaryTrackItem {
                    container_id: container_id.to_string(),
                    keyword: payload.keyword,
                    card_records: current_records,
                    requested_language: payload.request_language,
                    push_to_line: payload.push_to_line,
                };

                ENTITY_CARDS_TRACK_MAP.insert(container_id.to_string(), track_item.clone());

                if payload.schedule_polling {
                    tokio::spawn(async move {
                        schedule_auto_polling(track_item).await;
                    });
                }

                (
                    StatusCode::CREATED,
                    Json(CodexSummaryContainerResponse { container_id }),
                )
                    .into_response()
            }
        }
    }
}

pub async fn get_summary_container_status(
    _claim: Claim,
    Path(container_id): Path<String>,
    State(_): State<AppState>,
) -> Response {
    match get_container_status(container_id).await {
        Err(e) => {
            let error_message = format!("{}", e);
            tracing::error!("{}", &error_message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ServerError::with_message(error_message)),
            )
                .into_response()
        }
        Ok(state) => {
            let mut payload = HashMap::new();
            payload.insert("state".to_string(), state);
            (StatusCode::OK, Json(payload)).into_response()
        }
    }
}

pub async fn get_summary_result(
    _claim: Claim,
    Path(container_id): Path<String>,
    State(_): State<AppState>,
) -> Response {
    match get_summary(container_id).await {
        Err(e) => {
            let error_message = format!("{}", e);
            tracing::error!("{}", &error_message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ServerError::with_message(error_message)),
            )
                .into_response()
        }
        Ok(logs) => (StatusCode::OK, Json(logs)).into_response(),
    }
}

pub async fn get_all_entity_cards(_claim: Claim, State(_): State<AppState>) -> Response {
    let records = get_latest_entity_card_records().await;
    let map = HashMap::from([("records".to_string(), records)]);
    (StatusCode::OK, Json(map)).into_response()
}

async fn get_container_status(container_id: String) -> anyhow::Result<Option<ContainerState>> {
    let docker = connect_to_docker_socket();

    if docker.is_none() {
        return Err(anyhow::anyhow!("Failed to connect to Docker socket."));
    }

    let docker = docker.expect("Failed to connect to Docker.");

    match docker.inspect_container(&container_id, None).await {
        Err(e) => {
            let error_message = format!("Failed to inspect container: {}", e);
            Err(anyhow::anyhow!(error_message))
        }
        Ok(res) => Ok(res.state),
    }
}

async fn get_summary(container_id: String) -> anyhow::Result<CodexSummaryResponseLogs> {
    if let Some(state) = get_container_status(container_id.clone()).await?
        && let Some(status) = state.status
        && status != ContainerStateStatusEnum::EXITED
        && status != ContainerStateStatusEnum::DEAD
    {
        return Err(anyhow::anyhow!("Container hasn't exited yet."));
    }

    let log_options = LogsOptionsBuilder::default()
        .stdout(true)
        .stderr(true)
        .tail("100")
        .build();

    let docker = connect_to_docker_socket();

    if docker.is_none() {
        return Err(anyhow::anyhow!("Failed to connect to Docker socket."));
    }

    let docker = docker.expect("Failed to connect to Docker.");

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
            Err(anyhow::anyhow!(error_message))
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

            let new_records = get_new_records(&container_id)
                .await
                .into_iter()
                .map(|rec| rec.image_path)
                .collect::<Vec<_>>();

            if !new_records.is_empty() {
                response.images.extend_from_slice(&new_records);
            } else {
                if let Some(found_item) = search_record_image_path(&container_id).await {
                    response.images.push(found_item);
                }
            }

            if let Some(track_item) = ENTITY_CARDS_TRACK_MAP.get(&container_id) {
                let push_to_line = track_item.push_to_line;

                if push_to_line {
                    publish_summary(response.outs.join("\n"), new_records).await;
                }
            }

            ENTITY_CARDS_TRACK_MAP.remove(&container_id);

            Ok(response)
        }
    }
}

fn connect_to_docker_socket() -> Option<Docker> {
    Docker::connect_with_socket_defaults()
        .map_err(|e| tracing::error!("Failed to connect to Docker socket: {}", e))
        .ok()
}

async fn get_latest_entity_card_records() -> Vec<NovelEntityCardRecord> {
    let conn =
        SqliteConnection::connect(&format!("sqlite:///{}/novel.sqlite", NOVEL_ABSOULTE_PATH)).await;

    match conn {
        Err(e) => {
            tracing::error!("Failed to establish SQLite connection: {}", e);
            Vec::new()
        }
        Ok(mut connection) => {
            let query = sqlx::query_as::<_, NovelEntityCardRecord>("SELECT * FROM entity_cards")
                .fetch_all(&mut connection)
                .await;

            query
                .map_err(|e| tracing::error!("Failed to query from SQLite: {}", e))
                .unwrap_or_default()
        }
    }
}

async fn get_new_records(container_id: &str) -> Vec<NovelEntityCardRecord> {
    let latest_records = get_latest_entity_card_records()
        .await
        .into_iter()
        .map(|rec| (rec.id, rec))
        .collect::<HashSet<_>>();

    let mut records = Vec::with_capacity(latest_records.len());

    if let Some(track_item) = ENTITY_CARDS_TRACK_MAP.get(container_id) {
        let last_records = &track_item.card_records;

        let diff = latest_records
            .difference(last_records)
            .map(|(k, v)| (*k, v.clone()))
            .collect::<Vec<_>>();

        if !diff.is_empty() {
            let mut diff = diff.into_iter().map(|(_, v)| v).collect();
            records.append(&mut diff);
        }
    }

    records
}

async fn publish_summary(summary: String, new_records: Vec<String>) {
    let payload = LinePushMessage::NovelCodexSummary {
        summary,
        image_paths: new_records,
    };

    if let Err(e) = HTTP_CLIENT
        .post(&CONFIGURATION.neo_ellia_publication_endpoint)
        .json(&payload)
        .send()
        .await
    {
        tracing::error!("Failed to publish novel codex summary images: {}", e);
    }
}

async fn search_record_image_path(container_id: &str) -> Option<String> {
    let latest_records = get_latest_entity_card_records().await;

    let track_item = if let Some(item) = ENTITY_CARDS_TRACK_MAP.get(container_id) {
        item.clone()
    } else {
        CodexSummaryTrackItem::default()
    };

    let latest_records = latest_records
        .into_iter()
        .filter(|rec| rec.language == track_item.requested_language.to_string().as_str())
        .collect::<Vec<_>>();

    let keyword = track_item.keyword.as_str();
    let mut possible_cases = vec![
        ccase!(title -> kebab, keyword),
        ccase!(title -> snake, keyword),
        ccase!(title -> train, keyword),
        ccase!(title -> ada, keyword),
        ccase!(title -> pascal, keyword),
        ccase!(title -> camel, keyword),
    ];

    let split_keywords = keyword
        .split(" ")
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    possible_cases.extend_from_slice(&split_keywords);

    let lowercase_keywords = split_keywords
        .into_iter()
        .map(|s| s.to_lowercase())
        .collect::<Vec<_>>();

    possible_cases.extend_from_slice(&lowercase_keywords);

    for case in possible_cases.into_iter() {
        let found = latest_records
            .iter()
            .find(|rec| rec.image_path.contains(&case));

        if let Some(record) = found {
            return Some(record.image_path.clone());
        }
    }

    None
}

async fn schedule_auto_polling(track_item: CodexSummaryTrackItem) {
    let sleep = sleep(Duration::from_mins(10));
    let mut timeout = std::pin::pin!(sleep);

    let mut interval = interval(Duration::from_secs(10));
    interval.tick().await;
    let mut exited = false;

    loop {
        let container_id = track_item.container_id.clone();

        tokio::select! {
            _ = &mut timeout => {
                tracing::error!("Failed to poll container result: time out.");
                break;
            }

            _ = interval.tick() => {
                match get_container_status(container_id).await {
                    Err(e) => {
                        tracing::error!("Failed to poll container status: {}", e);
                    }
                    Ok(res) => {
                        if let Some(ContainerStateStatusEnum::EXITED | ContainerStateStatusEnum::DEAD) = res.and_then(|r| r.status) {
                            exited = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    if exited && let Err(e) = get_summary(track_item.container_id.clone()).await {
        let error_message = format!("{}", e);
        tracing::error!("{}", &error_message);
    }
}
