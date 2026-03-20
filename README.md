lingua-forge

------------------------------------------------------------
# 1. コンテナ起動
docker compose up --build

# 2. 別ターミナルでモデルをダウンロード（初回のみ・数分かかる）
docker compose exec ollama ollama pull qwen2.5:7b