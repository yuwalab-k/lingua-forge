# Lingua Forge

ローカルで動く英語学習ツール。英文を登録して AI 翻訳・学習できます。

## 構成

| 役割 | 技術 |
|---|---|
| フロントエンド | Svelte + Vite |
| バックエンド | Rust / Axum |
| 翻訳エンジン | PLaMo-2-translate (Ollama 経由) |
| 文字起こし | Whisper medium (faster-whisper) |
| DB | SQLite |

## セットアップ

### 1. コンテナ起動ß

```bash
docker compose up --build
```

### 2. 翻訳モデルをダウンロード（初回のみ）

```bash
docker compose exec ollama ollama pull mitmul/plamo-2-translate
```

URL:http://localhost:5174

## データベース

SQLite を使用しています。DBファイルは `backend/data/lingua.db` に自動生成されます。

マイグレーションファイルは `backend/migrations/` に管理されており、サーバー起動時に自動で適用されます。


### 手動で適用する場合

`sqlx-cli` を使うと手動でマイグレーションを管理できます。

```bash
# sqlx-cli のインストール
cargo install sqlx-cli --no-default-features --features sqlite

# マイグレーション適用
cd backend
sqlx migrate run --database-url sqlite:./data/lingua.db

# 適用状況の確認
sqlx migrate info --database-url sqlite:./data/lingua.db
```
