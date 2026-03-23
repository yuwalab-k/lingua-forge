use axum::Json;
use serde::Serialize;

use crate::ai::is_local_llm_enabled;

#[derive(Serialize)]
pub struct AppConfig {
    pub ai_enabled: bool,
}

pub async fn get_config() -> Json<AppConfig> {
    Json(AppConfig {
        ai_enabled: is_local_llm_enabled(),
    })
}
