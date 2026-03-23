use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{ContentWithSentences, InsertSentenceRequest, Sentence, UpdateSentenceRequest};
use super::helpers::{fetch_content, fetch_sentences, to_response};

pub async fn update_sentence(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSentenceRequest>,
) -> Result<Json<Sentence>, StatusCode> {
    if let Some(english_text) = &req.english_text {
        sqlx::query("UPDATE sentences SET english_text = ? WHERE id = ?")
            .bind(english_text)
            .bind(&id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    if let Some(japanese_text) = &req.japanese_text {
        sqlx::query("UPDATE sentences SET japanese_text = ? WHERE id = ?")
            .bind(japanese_text)
            .bind(&id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    if let Some(text_completed) = req.text_completed {
        sqlx::query("UPDATE sentences SET text_completed = ? WHERE id = ?")
            .bind(text_completed)
            .bind(&id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    if let Some(speech_completed) = req.speech_completed {
        sqlx::query("UPDATE sentences SET speech_completed = ? WHERE id = ?")
            .bind(speech_completed)
            .bind(&id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let sentence = sqlx::query_as::<_, Sentence>(
        "SELECT id, content_id, sentence_index, english_text, japanese_text, text_completed, speech_completed, created_at FROM sentences WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(sentence))
}

pub async fn delete_sentence(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<ContentWithSentences>, StatusCode> {
    let row = sqlx::query_as::<_, (String, i64)>(
        "SELECT content_id, sentence_index FROM sentences WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;
    let (content_id, deleted_index) = row;

    sqlx::query("DELETE FROM sentences WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "UPDATE sentences SET sentence_index = sentence_index - 1 WHERE content_id = ? AND sentence_index > ?",
    )
    .bind(&content_id)
    .bind(deleted_index)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let content = fetch_content(&pool, &content_id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    let sentences = fetch_sentences(&pool, &content_id).await?;
    Ok(Json(to_response(content, sentences)))
}

pub async fn insert_sentence(
    State(pool): State<SqlitePool>,
    Path(content_id): Path<String>,
    Json(req): Json<InsertSentenceRequest>,
) -> Result<Json<ContentWithSentences>, StatusCode> {
    sqlx::query(
        "UPDATE sentences SET sentence_index = sentence_index + 1 WHERE content_id = ? AND sentence_index >= ?",
    )
    .bind(&content_id)
    .bind(req.insert_at)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO sentences (id, content_id, sentence_index, english_text, japanese_text, created_at) VALUES (?, ?, ?, ?, NULL, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&content_id)
    .bind(req.insert_at)
    .bind(&req.english_text)
    .bind(&now)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let content = fetch_content(&pool, &content_id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    let sentences = fetch_sentences(&pool, &content_id).await?;
    Ok(Json(to_response(content, sentences)))
}
