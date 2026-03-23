use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TranslateTextRequest {
    pub text: String,
}

#[derive(Serialize)]
pub struct TranslateTextResponse {
    pub result: String,
}

pub async fn translate_text(
    Json(req): Json<TranslateTextRequest>,
) -> Result<Json<TranslateTextResponse>, StatusCode> {
    if !crate::ai::is_local_llm_enabled() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    match crate::ai::translate_single(&req.text).await {
        Ok(result) => Ok(Json(TranslateTextResponse { result })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
