use crate::controller::{start_hangman, start_quiz, start_tictactoe};
use crate::model::MinigameRequest;
use actix_web::web::{Json, ServiceConfig};
use actix_web::{post, HttpResponse, Responder};

pub fn config_minigame_controller(cfg: &mut ServiceConfig) {
    cfg.service(dispatch_minigame);
}

#[post("/minigame/start")]
async fn dispatch_minigame(request_data: actix_web::web::Json<MinigameRequest>) -> impl Responder {
    let Json(data) = request_data;
    match data {
        MinigameRequest::Quiz {
            user,
            channel_id,
            application_id,
            interaction_token,
        } => actix_web::rt::spawn(async move {
            start_quiz(user, channel_id, application_id, interaction_token.clone())
                .await
                .expect("Failed to start a quiz game.");
        }),
        MinigameRequest::Hangman {
            user,
            channel_id,
            application_id,
            interaction_token,
        } => actix_web::rt::spawn(async move {
            start_hangman(user, channel_id, application_id, interaction_token.clone())
                .await
                .expect("Failed to start a hangman game.");
        }),
        MinigameRequest::Tictactoe {
            user,
            channel_id,
            application_id,
            interaction_token,
        } => actix_web::rt::spawn(async move {
            start_tictactoe(user, channel_id, application_id, interaction_token.clone())
                .await
                .expect("Failed to start a quiz game.");
        }),
    };
    HttpResponse::Ok().finish()
}
