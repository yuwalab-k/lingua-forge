use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{
    Content, ContentWithSentences, CreateContentRequest, UpdateContentRequest,
    UpdateSentenceRequest, ForceQuery, Sentence, split_sentences,
};

pub async fn list_contents(
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<Content>>, StatusCode> {
    let contents = sqlx::query_as::<_, Content>(
        "SELECT id, title, source, summary, created_at FROM contents ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(contents))
}

pub async fn get_content(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<ContentWithSentences>, StatusCode> {
    let content = fetch_content(&pool, &id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    let sentences = fetch_sentences(&pool, &id).await?;
    Ok(Json(to_response(content, sentences)))
}

pub async fn create_content(
    State(pool): State<SqlitePool>,
    Json(req): Json<CreateContentRequest>,
) -> Result<Json<ContentWithSentences>, StatusCode> {
    let content_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query("INSERT INTO contents (id, title, source, created_at) VALUES (?, ?, ?, ?)")
        .bind(&content_id)
        .bind(&req.title)
        .bind(&req.source)
        .bind(&now)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let english_sentences = split_sentences(&req.english_text);
    let mut saved_sentences = Vec::new();

    for (index, english) in english_sentences.iter().enumerate() {
        let sentence_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO sentences (id, content_id, sentence_index, english_text, japanese_text, created_at) VALUES (?, ?, ?, ?, NULL, ?)",
        )
        .bind(&sentence_id)
        .bind(&content_id)
        .bind(index as i64)
        .bind(english)
        .bind(&now)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        saved_sentences.push(Sentence {
            id: sentence_id,
            content_id: content_id.clone(),
            sentence_index: index as i64,
            english_text: english.clone(),
            japanese_text: None,
            created_at: now.clone(),
        });
    }

    Ok(Json(ContentWithSentences {
        id: content_id,
        title: req.title,
        source: req.source,
        summary: None,
        created_at: now,
        sentences: saved_sentences,
    }))
}

pub async fn delete_content(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("DELETE FROM contents WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn translate_content(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Query(query): Query<ForceQuery>,
) -> Result<Json<ContentWithSentences>, (StatusCode, String)> {
    let sentences = fetch_sentences(&pool, &id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DBエラー".to_string()))?;

    let untranslated: Vec<(String, String)> = sentences
        .iter()
        .filter(|s| query.force || s.japanese_text.is_none())
        .map(|s| (s.id.clone(), s.english_text.clone()))
        .collect();

    if !untranslated.is_empty() {
        let texts: Vec<String> = untranslated.iter().map(|(_, t)| t.clone()).collect();
        let translations = crate::ai::translate_sentences(&texts)
            .await
            .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e))?;

        for ((sentence_id, _), translation) in untranslated.iter().zip(translations.iter()) {
            sqlx::query("UPDATE sentences SET japanese_text = ? WHERE id = ?")
                .bind(translation)
                .bind(sentence_id)
                .execute(&pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    let content = fetch_content(&pool, &id)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "教材が見つかりません".to_string()))?;
    let updated_sentences = fetch_sentences(&pool, &id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DBエラー".to_string()))?;

    Ok(Json(to_response(content, updated_sentences)))
}

pub async fn summarize_content(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Query(_query): Query<ForceQuery>,
) -> Result<Json<ContentWithSentences>, (StatusCode, String)> {
    let sentences = fetch_sentences(&pool, &id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DBエラー".to_string()))?;

    let texts: Vec<String> = sentences.iter().map(|s| s.english_text.clone()).collect();
    let summary = crate::ai::summarize(&texts)
        .await
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e))?;

    sqlx::query("UPDATE contents SET summary = ? WHERE id = ?")
        .bind(&summary)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let content = fetch_content(&pool, &id)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "教材が見つかりません".to_string()))?;

    Ok(Json(to_response(content, sentences)))
}

pub async fn update_content(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(req): Json<UpdateContentRequest>,
) -> Result<Json<ContentWithSentences>, StatusCode> {
    // 英文を変更するので summary もリセット
    sqlx::query("UPDATE contents SET title = ?, source = ?, summary = NULL WHERE id = ?")
        .bind(&req.title)
        .bind(&req.source)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("DELETE FROM sentences WHERE content_id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let now = Utc::now().to_rfc3339();
    for (index, english) in split_sentences(&req.english_text).iter().enumerate() {
        sqlx::query(
            "INSERT INTO sentences (id, content_id, sentence_index, english_text, japanese_text, created_at) VALUES (?, ?, ?, ?, NULL, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&id)
        .bind(index as i64)
        .bind(english)
        .bind(&now)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let content = fetch_content(&pool, &id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    let sentences = fetch_sentences(&pool, &id).await?;
    Ok(Json(to_response(content, sentences)))
}

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

async fn fetch_content(pool: &SqlitePool, id: &str) -> Result<Content, sqlx::Error> {
    sqlx::query_as::<_, Content>(
        "SELECT id, title, source, summary, created_at FROM contents WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

async fn fetch_sentences(pool: &SqlitePool, content_id: &str) -> Result<Vec<Sentence>, StatusCode> {
    sqlx::query_as::<_, Sentence>(
        "SELECT id, content_id, sentence_index, english_text, japanese_text, created_at FROM sentences WHERE content_id = ? ORDER BY sentence_index",
    )
    .bind(content_id)
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn to_response(content: Content, sentences: Vec<Sentence>) -> ContentWithSentences {
    ContentWithSentences {
        id: content.id,
        title: content.title,
        source: content.source,
        summary: content.summary,
        created_at: content.created_at,
        sentences,
    }
}
