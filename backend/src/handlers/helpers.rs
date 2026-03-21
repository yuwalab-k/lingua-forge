use axum::http::StatusCode;
use sqlx::SqlitePool;

use crate::models::{Content, ContentWithSentences, Sentence};

pub async fn resolve_source_name(pool: &SqlitePool, source_master_id: Option<&str>) -> Option<String> {
    let Some(id) = source_master_id else { return None };
    sqlx::query_scalar::<_, String>("SELECT name FROM source_masters WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

pub async fn fetch_content(pool: &SqlitePool, id: &str) -> Result<Content, sqlx::Error> {
    sqlx::query_as::<_, Content>(
        "SELECT id, title, source, source_master_id, source_url, is_translating, created_at FROM contents WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn fetch_sentences(pool: &SqlitePool, content_id: &str) -> Result<Vec<Sentence>, StatusCode> {
    sqlx::query_as::<_, Sentence>(
        "SELECT id, content_id, sentence_index, english_text, japanese_text, created_at FROM sentences WHERE content_id = ? ORDER BY sentence_index",
    )
    .bind(content_id)
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn to_response(content: Content, sentences: Vec<Sentence>) -> ContentWithSentences {
    ContentWithSentences {
        id: content.id,
        title: content.title,
        source: content.source,
        source_master_id: content.source_master_id,
        source_url: content.source_url,
        is_translating: content.is_translating,
        created_at: content.created_at,
        sentences,
    }
}
