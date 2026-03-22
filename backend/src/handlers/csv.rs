use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::split_sentences;
use super::helpers::resolve_source_name;

// ── Export ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ExportQuery {
    /// カンマ区切りのID。省略時は全件エクスポート
    pub ids: Option<String>,
}

pub async fn export_contents_csv(
    State(pool): State<SqlitePool>,
    Query(query): Query<ExportQuery>,
) -> Result<Response<String>, StatusCode> {
    // ids パラメータをパース
    let id_filter: Option<Vec<String>> = query.ids.map(|s| {
        s.split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    });

    // contents + sentences を結合して取得（動的フィルタ）
    let sql = r#"
        SELECT
            c.id,
            c.title,
            c.source,
            c.source_url,
            s.english_text,
            s.japanese_text
        FROM contents c
        LEFT JOIN sentences s ON s.content_id = c.id
        ORDER BY c.created_at DESC, s.sentence_index ASC
    "#;

    // sqlx のマクロはコンパイル時SQL検証なので動的クエリは query_as を使用
    #[derive(sqlx::FromRow)]
    struct ExportRow {
        id: String,
        title: String,
        source: Option<String>,
        source_url: Option<String>,
        english_text: Option<String>,
        japanese_text: Option<String>,
    }

    let all_rows = sqlx::query_as::<_, ExportRow>(sql)
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // IDフィルタ適用
    let rows: Vec<ExportRow> = if let Some(ref ids) = id_filter {
        all_rows.into_iter().filter(|r| ids.contains(&r.id)).collect()
    } else {
        all_rows
    };

    // content_id ごとに sentences をまとめる（順序維持）
    let mut content_order: Vec<String> = Vec::new();
    let mut content_map: std::collections::HashMap<
        String,
        (String, Option<String>, Option<String>, Vec<String>, Vec<Option<String>>),
    > = std::collections::HashMap::new();

    for row in rows {
        let id = row.id.clone();
        if !content_map.contains_key(&id) {
            content_order.push(id.clone());
            content_map.insert(
                id.clone(),
                (
                    row.title.clone(),
                    row.source.clone(),
                    row.source_url.clone(),
                    Vec::new(),
                    Vec::new(),
                ),
            );
        }
        if let Some(entry) = content_map.get_mut(&id) {
            if let Some(en) = row.english_text {
                entry.3.push(en);
                entry.4.push(row.japanese_text);
            }
        }
    }

    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    wtr.write_record(["id", "title", "source", "source_url", "english_text", "japanese_text"])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for id in &content_order {
        if let Some((title, source, source_url, en_sentences, ja_sentences)) =
            content_map.get(id)
        {
            let english_text = en_sentences.join(" ");
            let japanese_text = ja_sentences
                .iter()
                .filter_map(|j| j.as_deref())
                .collect::<Vec<_>>()
                .join(" ");
            wtr.write_record([
                id.as_str(),
                title.as_str(),
                source.as_deref().unwrap_or(""),
                source_url.as_deref().unwrap_or(""),
                &english_text,
                &japanese_text,
            ])
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    let data = wtr
        .into_inner()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let body = String::from_utf8(data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"lingua_forge_export.csv\"",
        )
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}

// ── Import ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ImportRequest {
    pub csv_text: String,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(serde::Serialize)]
pub struct PendingUpdate {
    pub line: usize,
    pub id: String,
    pub existing_title: String,
    pub new_title: String,
}

#[derive(serde::Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    /// confirmed=false のときのみ: 新規追加件数（まだDB未反映）
    pub to_create: usize,
    /// confirmed=false のときのみ: 更新候補
    pub pending_updates: Vec<PendingUpdate>,
}

pub async fn import_contents_csv(
    State(pool): State<SqlitePool>,
    Json(req): Json<ImportRequest>,
) -> Result<Json<ImportResult>, StatusCode> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(req.csv_text.as_bytes());

    // ── ヘッダー検証 ──────────────────────────────────────────────────────────
    let headers = rdr
        .headers()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .clone();

    fn col_index(headers: &csv::StringRecord, name: &str) -> Option<usize> {
        headers.iter().position(|h| h.trim() == name)
    }

    let col_id = col_index(&headers, "id");
    let col_title = col_index(&headers, "title");
    let col_english = col_index(&headers, "english_text");
    let col_source = col_index(&headers, "source");
    let col_source_url = col_index(&headers, "source_url");

    if col_title.is_none() || col_english.is_none() {
        return Ok(Json(ImportResult {
            imported: 0,
            updated: 0,
            skipped: 0,
            to_create: 0,
            pending_updates: vec![],
            errors: vec![format!(
                "ヘッダーに必須カラムがありません（必須: title, english_text / 検出されたヘッダー: {}）",
                headers.iter().collect::<Vec<_>>().join(", ")
            )],
        }));
    }

    let col_title = col_title.unwrap();
    let col_english = col_english.unwrap();

    // ── 全行パース ─────────────────────────────────────────────────────────────
    struct ParsedRow {
        line: usize,
        id: Option<String>,
        title: String,
        english_text: String,
        source_name: String,
        source_url: String,
    }

    let mut parsed_rows: Vec<ParsedRow> = Vec::new();
    let mut parse_errors: Vec<String> = Vec::new();
    let mut parse_skipped = 0usize;

    for (i, result) in rdr.records().enumerate() {
        let line = i + 2;
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                parse_errors.push(format!("行 {line}: CSVパースエラー: {e}"));
                parse_skipped += 1;
                continue;
            }
        };

        let id = col_id
            .and_then(|c| record.get(c))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let title = record.get(col_title).unwrap_or("").trim().to_string();
        let english_text = record.get(col_english).unwrap_or("").trim().to_string();
        let source_name = col_source.and_then(|c| record.get(c)).unwrap_or("").trim().to_string();
        let source_url = col_source_url.and_then(|c| record.get(c)).unwrap_or("").trim().to_string();

        if title.is_empty() {
            parse_errors.push(format!("行 {line}: title が空のためスキップ"));
            parse_skipped += 1;
            continue;
        }
        if english_text.is_empty() {
            parse_errors.push(format!("行 {line}: english_text が空のためスキップ"));
            parse_skipped += 1;
            continue;
        }

        parsed_rows.push(ParsedRow { line, id, title, english_text, source_name, source_url });
    }

    // ── プレビューモード（confirmed=false）────────────────────────────────────
    if !req.confirmed {
        let mut to_create = 0usize;
        let mut pending_updates: Vec<PendingUpdate> = Vec::new();
        let mut errors = parse_errors;

        for row in &parsed_rows {
            match &row.id {
                None => {
                    to_create += 1;
                }
                Some(id) => {
                    let existing = sqlx::query_scalar::<_, String>(
                        "SELECT title FROM contents WHERE id = ?",
                    )
                    .bind(id)
                    .fetch_optional(&pool)
                    .await
                    .unwrap_or(None);

                    match existing {
                        Some(existing_title) => {
                            pending_updates.push(PendingUpdate {
                                line: row.line,
                                id: id.clone(),
                                existing_title,
                                new_title: row.title.clone(),
                            });
                        }
                        None => {
                            errors.push(format!(
                                "行 {}: ID「{}」はDBに存在しません（スキップされます）",
                                row.line, id
                            ));
                        }
                    }
                }
            }
        }

        return Ok(Json(ImportResult {
            imported: 0,
            updated: 0,
            skipped: parse_skipped,
            to_create,
            pending_updates,
            errors,
        }));
    }

    // ── 実行モード（confirmed=true）───────────────────────────────────────────
    let mut imported = 0usize;
    let mut updated = 0usize;
    let mut skipped = parse_skipped;
    let mut errors = parse_errors;

    for row in &parsed_rows {
        // 出典マスタの解決
        let source_master_id: Option<String> = if row.source_name.is_empty() {
            None
        } else {
            let found = sqlx::query_scalar::<_, String>(
                "SELECT id FROM source_masters WHERE name = ?",
            )
            .bind(&row.source_name)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            if found.is_none() {
                errors.push(format!(
                    "行 {}: 出典「{}」は出典マスタに存在しないため未紐付けで登録",
                    row.line, row.source_name
                ));
            }
            found
        };

        let resolved_source = resolve_source_name(&pool, source_master_id.as_deref()).await;
        let now = Utc::now().to_rfc3339();

        match &row.id {
            // ── 新規追加 ──────────────────────────────────────────────────────
            None => {
                let content_id = Uuid::new_v4().to_string();

                let insert_result = sqlx::query(
                    "INSERT INTO contents (id, title, source, source_master_id, source_url, created_at) VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(&content_id)
                .bind(&row.title)
                .bind(&resolved_source)
                .bind(&source_master_id)
                .bind(if row.source_url.is_empty() { None } else { Some(row.source_url.clone()) })
                .bind(&now)
                .execute(&pool)
                .await;

                if let Err(e) = insert_result {
                    errors.push(format!("行 {}: DB挿入エラー: {e}", row.line));
                    skipped += 1;
                    continue;
                }

                let sentences = split_sentences(&row.english_text);
                if sentences.is_empty() {
                    errors.push(format!("行 {}: 文の分割結果が0件です", row.line));
                }
                for (index, sentence) in sentences.iter().enumerate() {
                    if let Err(e) = sqlx::query(
                        "INSERT INTO sentences (id, content_id, sentence_index, english_text, japanese_text, created_at) VALUES (?, ?, ?, ?, NULL, ?)",
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(&content_id)
                    .bind(index as i64)
                    .bind(sentence)
                    .bind(&now)
                    .execute(&pool)
                    .await
                    {
                        errors.push(format!("行 {}: 文({index})の挿入エラー: {e}", row.line));
                    }
                }
                imported += 1;
            }

            // ── 更新 ──────────────────────────────────────────────────────────
            Some(id) => {
                let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM contents WHERE id = ?")
                    .bind(id)
                    .fetch_one(&pool)
                    .await
                    .unwrap_or(0);

                if exists == 0 {
                    errors.push(format!("行 {}: ID「{id}」はDBに存在しないためスキップ", row.line));
                    skipped += 1;
                    continue;
                }

                if let Err(e) = sqlx::query(
                    "UPDATE contents SET title=?, source=?, source_master_id=?, source_url=? WHERE id=?",
                )
                .bind(&row.title)
                .bind(&resolved_source)
                .bind(&source_master_id)
                .bind(if row.source_url.is_empty() { None } else { Some(row.source_url.clone()) })
                .bind(id)
                .execute(&pool)
                .await
                {
                    errors.push(format!("行 {}: DB更新エラー: {e}", row.line));
                    skipped += 1;
                    continue;
                }

                // sentences を削除して再挿入
                if let Err(e) = sqlx::query("DELETE FROM sentences WHERE content_id = ?")
                    .bind(id)
                    .execute(&pool)
                    .await
                {
                    errors.push(format!("行 {}: sentences削除エラー: {e}", row.line));
                    skipped += 1;
                    continue;
                }

                let sentences = split_sentences(&row.english_text);
                for (index, sentence) in sentences.iter().enumerate() {
                    if let Err(e) = sqlx::query(
                        "INSERT INTO sentences (id, content_id, sentence_index, english_text, japanese_text, created_at) VALUES (?, ?, ?, ?, NULL, ?)",
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(id)
                    .bind(index as i64)
                    .bind(sentence)
                    .bind(&now)
                    .execute(&pool)
                    .await
                    {
                        errors.push(format!("行 {}: 文({index})の挿入エラー: {e}", row.line));
                    }
                }
                updated += 1;
            }
        }
    }

    Ok(Json(ImportResult { imported, updated, skipped, errors, to_create: 0, pending_updates: vec![] }))
}

// ── Bulk Delete ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BulkDeleteRequest {
    pub ids: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct BulkDeleteResult {
    pub deleted: usize,
}

pub async fn bulk_delete_contents(
    State(pool): State<SqlitePool>,
    Json(req): Json<BulkDeleteRequest>,
) -> Result<Json<BulkDeleteResult>, StatusCode> {
    if req.ids.is_empty() {
        return Ok(Json(BulkDeleteResult { deleted: 0 }));
    }

    let mut deleted = 0usize;
    for id in &req.ids {
        let result = sqlx::query("DELETE FROM contents WHERE id = ?")
            .bind(id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        deleted += result.rows_affected() as usize;
    }

    Ok(Json(BulkDeleteResult { deleted }))
}
