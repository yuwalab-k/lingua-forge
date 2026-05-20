# Lingua Forge

英語学習用の静的サイト生成ツール。

## 構造

```
texts/{source}/{episode}.txt    ← 英語原稿
json/{source}/{episode}.json    ← 翻訳作業ファイル（japanese_text を Claude に埋めてもらう）
docs/{source}/{episode}.html    ← 公開HTML（GitHub Pages）
docs/index.html                 ← カタログ（全教材一覧）
docs/index.json                 ← カタログ用データ
assets/                         ← 共有フォントなど
```

## ワークフロー

### 新しい教材を追加する

1. `texts/{source}/{episode}.txt` に英語テキストを置く
   ```
   title: Episode Title
   url: https://...
   ---
   English body text here.
   ```

2. JSON に変換（センテンス分割）
   ```bash
   node scripts/import.js texts/syntax/1005.txt
   ```

3. `json/{source}/{episode}.json` を Claude に渡して `japanese_text` を埋めてもらう

4. HTML を生成
   ```bash
   node scripts/gen-html.js json/syntax/1005.json
   ```

5. カタログを更新（教材追加時のみ）
   ```bash
   node scripts/gen-catalog.js
   ```

6. デプロイ
   ```bash
   git add docs/ && git commit -m "add syntax/1005" && git push
   ```

### 既存教材の日本語を更新する

```bash
# JSON を更新後
node scripts/gen-html.js json/{source}/{episode}.json
git add docs/{source}/{episode}.html && git commit && git push
```

## スクリプト

| スクリプト | 役割 |
|---|---|
| `scripts/import.js` | txt → JSON（センテンス分割） |
| `scripts/gen-html.js` | JSON → HTML |
| `scripts/gen-catalog.js` | docs/index.html を再生成 |
