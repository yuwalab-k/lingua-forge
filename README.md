

## 構成

| 役割 | 技術 |
|---|---|
| フロントエンド | Svelte + Vite |
| バックエンド | Rust / Axum |
| 翻訳エンジン | PLaMo-2-translate (Ollama 経由) |
| DB | SQLite |

## セットアップ

### 1. コンテナ起動

```bash
docker compose up --build
```


### 2. 別ターミナルでモデルをダウンロード（初回のみ・数分かかる）

```bash
docker compose exec ollama ollama pull mitmul/plamo-2-translate
```

http://localhost:5174