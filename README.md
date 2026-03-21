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

### 1. コンテナ起動

```bash
docker compose up --build
```

### 2. 翻訳モデルをダウンロード（初回のみ）

```bash
docker compose exec ollama ollama pull mitmul/plamo-2-translate
```

URL:http://localhost:5174 