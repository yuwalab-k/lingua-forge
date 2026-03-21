use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{SourceMaster, CreateSourceMasterRequest, UpdateSourceMasterRequest};

pub async fn list_sources(
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<SourceMaster>>, StatusCode> {
    let sources = sqlx::query_as::<_, SourceMaster>(
        "SELECT id, name, created_at FROM source_masters ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(sources))
}

pub async fn create_source(
    State(pool): State<SqlitePool>,
    Json(req): Json<CreateSourceMasterRequest>,
) -> Result<Json<SourceMaster>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO source_masters (id, name, created_at) VALUES (?, ?, ?)",
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&now)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SourceMaster { id, name: req.name, created_at: now }))
}

pub async fn update_source(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSourceMasterRequest>,
) -> Result<Json<SourceMaster>, StatusCode> {
    sqlx::query("UPDATE source_masters SET name = ? WHERE id = ?")
        .bind(&req.name)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 名前変更時、紐づく contents の source も更新
    sqlx::query("UPDATE contents SET source = ? WHERE source_master_id = ?")
        .bind(&req.name)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let master = sqlx::query_as::<_, SourceMaster>(
        "SELECT id, name, created_at FROM source_masters WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(master))
}

pub async fn delete_source(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("UPDATE contents SET source_master_id = NULL WHERE source_master_id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("DELETE FROM source_masters WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
