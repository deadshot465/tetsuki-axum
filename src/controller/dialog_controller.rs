use crate::model::dialog_info::DialogInfo;
use crate::shared::constants::ASSET_DIRECTORY;
use crate::shared::web_driver::get_dialog;
use actix_web::web::ServiceConfig;
use actix_web::{get, post, HttpResponse, Responder};
use once_cell::sync::Lazy;
use std::collections::HashMap;

const BACKGROUNDS_PATH: &str = "/dialog/images/backgrounds";
const CHARACTERS_PATH: &str = "/dialog/images/characters";

static BACKGROUNDS_LIST: Lazy<Vec<String>> = Lazy::new(|| build_list(BACKGROUNDS_PATH));
static CHARACTERS_LIST: Lazy<Vec<String>> = Lazy::new(|| build_list(CHARACTERS_PATH));

pub fn config_dialog_controller(cfg: &mut ServiceConfig) {
    cfg.service(generate_dialog).service(get_dialog_options);
}

#[post("/dialog")]
async fn generate_dialog(request: actix_web::web::Json<DialogInfo>) -> impl Responder {
    if !CHARACTERS_LIST.contains(&request.character)
        || !BACKGROUNDS_LIST.contains(&request.background)
        || request.text.is_empty()
    {
        return HttpResponse::BadRequest()
            .body("The specified character/background doesn't exist, or the text is empty.");
    }

    match get_dialog(request.into_inner()).await {
        Ok(result) => HttpResponse::Ok().body(result),
        Err(e) => {
            log::error!("An error occurred when generating the dialog: {}", e);
            HttpResponse::BadRequest().body(e.to_string())
        }
    }
}

#[get("/dialog")]
async fn get_dialog_options() -> impl Responder {
    let mut dialog_options = HashMap::new();
    dialog_options.insert("characters".to_string(), CHARACTERS_LIST.clone());
    dialog_options.insert("backgrounds".to_string(), BACKGROUNDS_LIST.clone());
    HttpResponse::Ok().json(dialog_options)
}

fn build_list(path: &str) -> Vec<String> {
    let files_path = String::from(ASSET_DIRECTORY) + path;
    let files_directory = std::path::Path::new(&files_path);
    if !files_directory.exists() {
        log::error!("{} folder doesn't exist.", path);
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
