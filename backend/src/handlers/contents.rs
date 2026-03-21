use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{Content, ContentWithSentences, CreateContentRequest, UpdateContentRequest, ForceQuery, Sentence, split_sentences};
use super::helpers::{resolve_source_name, fetch_content, fetch_sentences, to_response};

pub async fn list_contents(
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<Content>>, StatusCode> {
    let contents = sqlx::query_as::<_, Content>(
        "SELECT id, title, source, source_master_id, source_url, is_translating, created_at FROM contents ORDER BY created_at DESC",
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
    let source_name = resolve_source_name(&pool, req.source_master_id.as_deref()).await;

    sqlx::query(
        "INSERT INTO contents (id, title, source, source_master_id, source_url, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&content_id)
    .bind(&req.title)
    .bind(&source_name)
    .bind(&req.source_master_id)
    .bind(&req.source_url)
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
            text_completed: false,
            speech_completed: false,
            created_at: now.clone(),
        });
    }

    Ok(Json(ContentWithSentences {
        id: content_id,
        title: req.title,
        source: source_name,
        source_master_id: req.source_master_id,
        source_url: req.source_url,
        is_translating: false,
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
    // 翻訳開始フラグをセット
    sqlx::query("UPDATE contents SET is_translating = 1 WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let result = async {
        // 再翻訳の場合は一旦全文の訳をクリア（進捗追跡のため）
        if query.force {
            sqlx::query("UPDATE sentences SET japanese_text = NULL WHERE content_id = ?")
                .bind(&id)
                .execute(&pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        let sentences = fetch_sentences(&pool, &id)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DBエラー".to_string()))?;

        let untranslated: Vec<(String, String)> = sentences
            .iter()
            .filter(|s| query.force || s.japanese_text.is_none())
            .map(|s| (s.id.clone(), s.english_text.clone()))
            .collect();

        for (sentence_id, english_text) in &untranslated {
            // キャンセルされていたらループを抜ける
            let still_translating: bool = sqlx::query_scalar(
                "SELECT is_translating FROM contents WHERE id = ?",
            )
            .bind(&id)
            .fetch_one(&pool)
            .await
            .unwrap_or(false);
            if !still_translating {
                break;
            }

            let translation = crate::ai::translate_single(english_text)
                .await
                .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e))?;

            sqlx::query("UPDATE sentences SET japanese_text = ? WHERE id = ?")
                .bind(&translation)
                .bind(sentence_id)
                .execute(&pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        let content = fetch_content(&pool, &id)
            .await
            .map_err(|_| (StatusCode::NOT_FOUND, "教材が見つかりません".to_string()))?;
        let updated_sentences = fetch_sentences(&pool, &id)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DBエラー".to_string()))?;

        Ok(Json(to_response(content, updated_sentences)))
    }
    .await;

    // 成功・失敗にかかわらずフラグをリセット
    let _ = sqlx::query("UPDATE contents SET is_translating = 0 WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await;

    result
}

pub async fn update_content(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(req): Json<UpdateContentRequest>,
) -> Result<Json<ContentWithSentences>, StatusCode> {
    let source_name = resolve_source_name(&pool, req.source_master_id.as_deref()).await;

    sqlx::query(
        "UPDATE contents SET title = ?, source = ?, source_master_id = ?, source_url = ? WHERE id = ?",
    )
    .bind(&req.title)
    .bind(&source_name)
    .bind(&req.source_master_id)
    .bind(&req.source_url)
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

pub async fn cancel_translate(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> StatusCode {
    let _ = sqlx::query("UPDATE contents SET is_translating = 0 WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await;
    StatusCode::NO_CONTENT
}
