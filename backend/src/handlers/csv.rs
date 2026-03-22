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
    wtr.write_record(["title", "source", "source_url", "english_text", "japanese_text"])
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
}

#[derive(serde::Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
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

    let col_title = col_index(&headers, "title");
    let col_english = col_index(&headers, "english_text");
    let col_source = col_index(&headers, "source");
    let col_source_url = col_index(&headers, "source_url");

    if col_title.is_none() || col_english.is_none() {
        return Ok(Json(ImportResult {
            imported: 0,
            skipped: 0,
            errors: vec![format!(
                "ヘッダーに必須カラムがありません（必須: title, english_text / 検出されたヘッダー: {}）",
                headers.iter().collect::<Vec<_>>().join(", ")
            )],
        }));
    }

    let col_title = col_title.unwrap();
    let col_english = col_english.unwrap();

    // ── 行ごとにインポート ─────────────────────────────────────────────────────
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for (i, result) in rdr.records().enumerate() {
        let line = i + 2; // 1行目=ヘッダーなので2始まり
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                errors.push(format!("行 {line}: CSVパースエラー: {e}"));
                skipped += 1;
                continue;
            }
        };

        let title = record.get(col_title).unwrap_or("").trim().to_string();
        let english_text = record.get(col_english).unwrap_or("").trim().to_string();
        let source_name = col_source
            .and_then(|c| record.get(c))
            .unwrap_or("")
            .trim()
            .to_string();
        let source_url = col_source_url
            .and_then(|c| record.get(c))
            .unwrap_or("")
            .trim()
            .to_string();

        if title.is_empty() {
            errors.push(format!("行 {line}: title が空のためスキップ"));
            skipped += 1;
            continue;
        }
        if english_text.is_empty() {
            errors.push(format!("行 {line}: english_text が空のためスキップ"));
            skipped += 1;
            continue;
        }

        // 出典マスタの解決
        let source_master_id: Option<String> = if source_name.is_empty() {
            None
        } else {
            let found = sqlx::query_scalar::<_, String>(
                "SELECT id FROM source_masters WHERE name = ?",
            )
            .bind(&source_name)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            if found.is_none() {
                errors.push(format!(
                    "行 {line}: 出典「{source_name}」は出典マスタに存在しないため未紐付けで登録"
                ));
            }
            found
        };

        let resolved_source = resolve_source_name(&pool, source_master_id.as_deref()).await;
        let content_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let insert_result = sqlx::query(
            "INSERT INTO contents (id, title, source, source_master_id, source_url, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&content_id)
        .bind(&title)
        .bind(&resolved_source)
        .bind(&source_master_id)
        .bind(if source_url.is_empty() { None } else { Some(source_url.clone()) })
        .bind(&now)
        .execute(&pool)
        .await;

        if let Err(e) = insert_result {
            errors.push(format!("行 {line}: DB挿入エラー: {e}"));
            skipped += 1;
            continue;
        }

        let sentences = split_sentences(&english_text);
        if sentences.is_empty() {
            errors.push(format!("行 {line}: 文の分割結果が0件です（english_text を確認してください）"));
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
                errors.push(format!("行 {line}: 文({index})の挿入エラー: {e}"));
            }
        }

        imported += 1;
    }

    Ok(Json(ImportResult { imported, skipped, errors }))
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
