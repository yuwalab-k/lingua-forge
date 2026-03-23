use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use chrono::Utc;
use indexmap::IndexMap;
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

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
    let id_filter: Option<Vec<String>> = query.ids.map(|s| {
        s.split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    });

    let sql = r#"
        SELECT
            c.id,
            c.title,
            c.source,
            c.source_url,
            s.sentence_index,
            s.english_text,
            s.japanese_text
        FROM contents c
        JOIN sentences s ON s.content_id = c.id
        ORDER BY c.created_at DESC, s.sentence_index ASC
    "#;

    #[derive(sqlx::FromRow)]
    struct ExportRow {
        id: String,
        title: String,
        source: Option<String>,
        source_url: Option<String>,
        sentence_index: i64,
        english_text: String,
        japanese_text: Option<String>,
    }

    let all_rows = sqlx::query_as::<_, ExportRow>(sql)
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows: Vec<ExportRow> = if let Some(ref ids) = id_filter {
        all_rows.into_iter().filter(|r| ids.contains(&r.id)).collect()
    } else {
        all_rows
    };

    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    wtr.write_record(["content_id", "content_title", "source", "source_url", "sentence_index", "english_text", "japanese_text"])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for row in &rows {
        wtr.write_record([
            row.id.as_str(),
            row.title.as_str(),
            row.source.as_deref().unwrap_or(""),
            row.source_url.as_deref().unwrap_or(""),
            &row.sentence_index.to_string(),
            row.english_text.as_str(),
            row.japanese_text.as_deref().unwrap_or(""),
        ])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    pub to_create: usize,
    pub pending_updates: Vec<PendingUpdate>,
}

struct ParsedSentence {
    sentence_index: i64,
    english_text: String,
    japanese_text: Option<String>,
}

struct ContentGroup {
    first_line: usize,
    content_id: Option<String>,
    title: String,
    source_name: String,
    source_url: String,
    sentences: Vec<ParsedSentence>,
}

pub async fn import_contents_csv(
    State(pool): State<SqlitePool>,
    Json(req): Json<ImportRequest>,
) -> Result<Json<ImportResult>, StatusCode> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(req.csv_text.as_bytes());

    let headers = rdr
        .headers()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .clone();

    fn col_index(headers: &csv::StringRecord, name: &str) -> Option<usize> {
        headers.iter().position(|h| h.trim() == name)
    }

    let col_content_id = col_index(&headers, "content_id");
    let col_title = col_index(&headers, "content_title");
    let col_english = col_index(&headers, "english_text");
    let col_japanese = col_index(&headers, "japanese_text");
    let col_source = col_index(&headers, "source");
    let col_source_url = col_index(&headers, "source_url");
    let col_sentence_index = col_index(&headers, "sentence_index");

    if col_title.is_none() || col_english.is_none() {
        return Ok(Json(ImportResult {
            imported: 0,
            updated: 0,
            skipped: 0,
            to_create: 0,
            pending_updates: vec![],
            errors: vec![format!(
                "ヘッダーに必須カラムがありません（必須: content_title, english_text / 検出されたヘッダー: {}）",
                headers.iter().collect::<Vec<_>>().join(", ")
            )],
        }));
    }

    let col_title = col_title.unwrap();
    let col_english = col_english.unwrap();

    // ── 全行パース → ContentGroup にまとめる ──────────────────────────────
    // Key: content_id があればその値、なければ "NEW::" + title でグループ化
    let mut groups: IndexMap<String, ContentGroup> = IndexMap::new();
    let mut parse_errors: Vec<String> = Vec::new();
    let mut parse_skipped = 0usize;
    let mut sentence_counter: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

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

        let content_id = col_content_id
            .and_then(|c| record.get(c))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let title = record.get(col_title).unwrap_or("").trim().to_string();
        let english_text = record.get(col_english).unwrap_or("").trim().to_string();
        let japanese_text = col_japanese
            .and_then(|c| record.get(c))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let source_name = col_source.and_then(|c| record.get(c)).unwrap_or("").trim().to_string();
        let source_url = col_source_url.and_then(|c| record.get(c)).unwrap_or("").trim().to_string();

        if title.is_empty() {
            parse_errors.push(format!("行 {line}: content_title が空のためスキップ"));
            parse_skipped += 1;
            continue;
        }
        if english_text.is_empty() {
            parse_errors.push(format!("行 {line}: english_text が空のためスキップ"));
            parse_skipped += 1;
            continue;
        }

        let group_key = content_id.clone().unwrap_or_else(|| format!("NEW::{}", title));

        let sentence_index = col_sentence_index
            .and_then(|c| record.get(c))
            .and_then(|s| s.trim().parse::<i64>().ok())
            .unwrap_or_else(|| {
                let count = sentence_counter.entry(group_key.clone()).or_insert(0);
                let idx = *count;
                *count += 1;
                idx
            });

        groups.entry(group_key.clone()).or_insert_with(|| ContentGroup {
            first_line: line,
            content_id: content_id.clone(),
            title: title.clone(),
            source_name: source_name.clone(),
            source_url: source_url.clone(),
            sentences: Vec::new(),
        });

        if let Some(group) = groups.get_mut(&group_key) {
            group.sentences.push(ParsedSentence { sentence_index, english_text, japanese_text });
        }
    }

    // 各グループのセンテンスをインデックス順にソート
    for group in groups.values_mut() {
        group.sentences.sort_by_key(|s| s.sentence_index);
    }

    // ── プレビューモード（confirmed=false）────────────────────────────────────
    if !req.confirmed {
        let mut to_create = 0usize;
        let mut pending_updates: Vec<PendingUpdate> = Vec::new();
        let errors = parse_errors;

        for group in groups.values() {
            match &group.content_id {
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
                                line: group.first_line,
                                id: id.clone(),
                                existing_title,
                                new_title: group.title.clone(),
                            });
                        }
                        None => {
                            to_create += 1;
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

    for group in groups.values() {
        let source_master_id: Option<String> = if group.source_name.is_empty() {
            None
        } else {
            let found = sqlx::query_scalar::<_, String>(
                "SELECT id FROM source_masters WHERE name = ?",
            )
            .bind(&group.source_name)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            if found.is_none() {
                errors.push(format!(
                    "行 {}: 出典「{}」は出典マスタに存在しないため未紐付けで登録",
                    group.first_line, group.source_name
                ));
            }
            found
        };

        let resolved_source = resolve_source_name(&pool, source_master_id.as_deref()).await;
        let now = Utc::now().to_rfc3339();

        match &group.content_id {
            // ── 新規追加 ──────────────────────────────────────────────────────
            None => {
                let content_id = Uuid::new_v4().to_string();

                let insert_result = sqlx::query(
                    "INSERT INTO contents (id, title, source, source_master_id, source_url, created_at) VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(&content_id)
                .bind(&group.title)
                .bind(&resolved_source)
                .bind(&source_master_id)
                .bind(if group.source_url.is_empty() { None } else { Some(group.source_url.clone()) })
                .bind(&now)
                .execute(&pool)
                .await;

                if let Err(e) = insert_result {
                    errors.push(format!("行 {}: DB挿入エラー: {e}", group.first_line));
                    skipped += 1;
                    continue;
                }

                for sentence in &group.sentences {
                    if let Err(e) = sqlx::query(
                        "INSERT INTO sentences (id, content_id, sentence_index, english_text, japanese_text, created_at) VALUES (?, ?, ?, ?, ?, ?)",
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(&content_id)
                    .bind(sentence.sentence_index)
                    .bind(&sentence.english_text)
                    .bind(&sentence.japanese_text)
                    .bind(&now)
                    .execute(&pool)
                    .await
                    {
                        errors.push(format!("行 {}: 文({})の挿入エラー: {e}", group.first_line, sentence.sentence_index));
                    }
                }
                imported += 1;
            }

            // ── 更新（DBに存在しない場合は新規作成） ─────────────────────────
            Some(id) => {
                let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM contents WHERE id = ?")
                    .bind(id)
                    .fetch_one(&pool)
                    .await
                    .unwrap_or(0);

                if exists == 0 {
                    // DBに存在しない → 指定されたIDで新規作成
                    let insert_result = sqlx::query(
                        "INSERT INTO contents (id, title, source, source_master_id, source_url, created_at) VALUES (?, ?, ?, ?, ?, ?)",
                    )
                    .bind(id)
                    .bind(&group.title)
                    .bind(&resolved_source)
                    .bind(&source_master_id)
                    .bind(if group.source_url.is_empty() { None } else { Some(group.source_url.clone()) })
                    .bind(&now)
                    .execute(&pool)
                    .await;

                    if let Err(e) = insert_result {
                        errors.push(format!("行 {}: DB挿入エラー: {e}", group.first_line));
                        skipped += 1;
                        continue;
                    }

                    for sentence in &group.sentences {
                        if let Err(e) = sqlx::query(
                            "INSERT INTO sentences (id, content_id, sentence_index, english_text, japanese_text, created_at) VALUES (?, ?, ?, ?, ?, ?)",
                        )
                        .bind(Uuid::new_v4().to_string())
                        .bind(id)
                        .bind(sentence.sentence_index)
                        .bind(&sentence.english_text)
                        .bind(&sentence.japanese_text)
                        .bind(&now)
                        .execute(&pool)
                        .await
                        {
                            errors.push(format!("行 {}: 文({})の挿入エラー: {e}", group.first_line, sentence.sentence_index));
                        }
                    }
                    imported += 1;
                    continue;
                }

                if let Err(e) = sqlx::query(
                    "UPDATE contents SET title=?, source=?, source_master_id=?, source_url=? WHERE id=?",
                )
                .bind(&group.title)
                .bind(&resolved_source)
                .bind(&source_master_id)
                .bind(if group.source_url.is_empty() { None } else { Some(group.source_url.clone()) })
                .bind(id)
                .execute(&pool)
                .await
                {
                    errors.push(format!("行 {}: DB更新エラー: {e}", group.first_line));
                    skipped += 1;
                    continue;
                }

                if let Err(e) = sqlx::query("DELETE FROM sentences WHERE content_id = ?")
                    .bind(id)
                    .execute(&pool)
                    .await
                {
                    errors.push(format!("行 {}: sentences削除エラー: {e}", group.first_line));
                    skipped += 1;
                    continue;
                }

                for sentence in &group.sentences {
                    if let Err(e) = sqlx::query(
                        "INSERT INTO sentences (id, content_id, sentence_index, english_text, japanese_text, created_at) VALUES (?, ?, ?, ?, ?, ?)",
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(id)
                    .bind(sentence.sentence_index)
                    .bind(&sentence.english_text)
                    .bind(&sentence.japanese_text)
                    .bind(&now)
                    .execute(&pool)
                    .await
                    {
                        errors.push(format!("行 {}: 文({})の挿入エラー: {e}", group.first_line, sentence.sentence_index));
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
