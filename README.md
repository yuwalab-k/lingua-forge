lingua-forge

------------------------------------------------------------
初回セットアップ

1. フロントエンド依存ファイルのロック生成（初回のみ）

docker compose run --rm frontend sh -lc 'npm install --package-lock-only'

- package-lock.json を生成して、依存関係を固定します
- 初回のみ実行してください

2. Linux 環境で依存関係をクリーンインストール

docker compose run --rm frontend sh -lc 'npm ci'

- node_modules をクリーンにインストールします
- CI/CD や開発環境で確実に同じ依存を再現できます

------------------------------------------------------------
Docker ビルドと開発サーバー起動

# バックエンドとフロントエンドのビルドと起動
docker compose up --build -d

------------------------------------------------------------
クリーンアップ

# 停止
docker compose down

# キャッシュ削除して再ビルド
docker compose build --no-cache