use sqlx::{SqlitePool, sqlite::{SqliteConnectOptions, SqlitePoolOptions}};
use std::str::FromStr;

pub async fn create_pool(database_url: &str) -> SqlitePool {
    let options = SqliteConnectOptions::from_str(database_url)
        .expect("Invalid database URL")
        .create_if_missing(true);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .expect("Failed to connect to SQLite")
}

pub async fn migrate(pool: &SqlitePool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS contents (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            source TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sentences (
            id TEXT PRIMARY KEY,
            content_id TEXT NOT NULL,
            sentence_index INTEGER NOT NULL,
            english_text TEXT NOT NULL,
            japanese_text TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (content_id) REFERENCES contents(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to run migrations");

    // summary カラムを既存テーブルに追加（存在する場合はエラーを無視）
    let _ = sqlx::query("ALTER TABLE contents ADD COLUMN summary TEXT")
        .execute(pool)
        .await;

    // 出典マスタテーブル
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS source_masters (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            translate_prompt TEXT,
            created_at TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .expect("Failed to create source_masters table");

    // contents に source_master_id カラムを追加（存在する場合はエラーを無視）
    let _ = sqlx::query("ALTER TABLE contents ADD COLUMN source_master_id TEXT")
        .execute(pool)
        .await;
}
