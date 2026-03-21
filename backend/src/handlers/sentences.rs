use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::SqlitePool;

use crate::models::{Sentence, UpdateSentenceRequest};

pub async fn update_sentence(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSentenceRequest>,
) -> Result<Json<Sentence>, StatusCode> {
    sqlx::query("UPDATE sentences SET japanese_text = ? WHERE id = ?")
        .bind(&req.japanese_text)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let sentence = sqlx::query_as::<_, Sentence>(
        "SELECT id, content_id, sentence_index, english_text, japanese_text, created_at FROM sentences WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(sentence))
}
