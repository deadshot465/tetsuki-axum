use crate::controller::{start_hangman, start_quiz, start_tictactoe};
use actix_web::web::{Json, ServiceConfig};
use actix_web::{post, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
enum MinigameRequest {
    Quiz {
        user_id: u64,
        channel_id: u64,
        application_id: u64,
        interaction_token: String,
    },
    Hangman {
        user_id: u64,
        channel_id: u64,
        application_id: u64,
        interaction_token: String,
    },
    Tictactoe {
        user_id: u64,
        channel_id: u64,
        application_id: u64,
        interaction_token: String,
    },
}

pub fn config_minigame_controller(cfg: &mut ServiceConfig) {
    cfg.service(dispatch_minigame);
}

#[post("/minigame/start")]
async fn dispatch_minigame(request_data: actix_web::web::Json<MinigameRequest>) -> impl Responder {
    let Json(data) = request_data;
    match data {
        MinigameRequest::Quiz {
            user_id,
            channel_id,
            application_id,
            interaction_token,
        } => actix_web::rt::spawn(async move {
            start_quiz(
                user_id,
                channel_id,
                application_id,
                interaction_token.clone(),
            )
            .await
            .expect("Failed to start a quiz game.");
        }),
        MinigameRequest::Hangman {
            user_id,
            channel_id,
            application_id,
            interaction_token,
        } => actix_web::rt::spawn(async move {
            start_hangman(
                user_id,
                channel_id,
                application_id,
                interaction_token.clone(),
            )
            .await
            .expect("Failed to start a hangman game.");
        }),
        MinigameRequest::Tictactoe {
            user_id,
            channel_id,
            application_id,
            interaction_token,
        } => actix_web::rt::spawn(async move {
            start_tictactoe(
                user_id,
                channel_id,
                application_id,
                interaction_token.clone(),
            )
            .await
            .expect("Failed to start a quiz game.");
        }),
    };
    HttpResponse::Ok().finish()
}
