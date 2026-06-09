#!/usr/bin/env node
/**
 * 教材 JSON から閲覧・印刷用 HTML を生成するスクリプト
 *
 * Usage:
 *   node scripts/gen-html.js json/syntax/1005.json
 *   node scripts/gen-html.js json/syntax/1005.json --no-japanese
 *
 * オプション:
 *   --no-japanese   日本語訳を含めない
 *
 * 出力: docs/{content.id}.html  (例: docs/syntax/1005.html)
 */

const fs = require('fs');
const path = require('path');

const FONT_SRC = path.join(__dirname, '../assets/BIZUDPGothic-Regular.ttf');
const FONT_SIZES = { 大: 20, 中: 13, 小: 10 };
const SIZE = { basePt: 13, gap: 20 };

const args = process.argv.slice(2);
const contentJsonPath = args.find(a => !a.startsWith('--'));
const showJapanese = !args.includes('--no-japanese');

if (!contentJsonPath) {
  console.error('Usage: node scripts/gen-html.js json/{source}/{episode}.json [--no-japanese]');
  process.exit(1);
}
if (!fs.existsSync(contentJsonPath)) {
  console.error(`ERROR: ファイルが見つかりません: ${contentJsonPath}`);
  process.exit(1);
}

function toRubyHtml(text) {
  if (!text) return '';
  return text.replace(/《([^|》]+)\|([^》]+)》/g, '<ruby>$1<rt>$2</rt></ruby>');
}

function escapeHtml(str) {
  return (str || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

const content = JSON.parse(fs.readFileSync(contentJsonPath, 'utf8'));

// フォントを docs/assets/ にコピー（なければ）
const docsAssetsDir = path.join(__dirname, '../docs/assets');
const fontDest = path.join(docsAssetsDir, 'BIZUDPGothic-Regular.ttf');
fs.mkdirSync(docsAssetsDir, { recursive: true });
if (!fs.existsSync(fontDest)) {
  fs.copyFileSync(FONT_SRC, fontDest);
}

const s = SIZE;
const bodyPt = FONT_SIZES[content.fontSize] ?? SIZE.basePt;
const jpnPt = Math.round(bodyPt * 0.65);

const titleText = content.source
  ? `【${escapeHtml(content.source)}】${escapeHtml(content.title)}`
  : escapeHtml(content.title);

const sentencesHtml = content.sentences.map(sentence => {
  const eng = escapeHtml(sentence.english_text);
  const jpn = showJapanese && sentence.japanese_text ? toRubyHtml(sentence.japanese_text) : '';
  return `
    <div style="margin-bottom:${s.gap}pt; page-break-inside:avoid;">
      <div style="font-size:${bodyPt}pt; line-height:1.6; color:#111;">${eng}</div>
      ${jpn ? `<div style="font-size:${jpnPt}pt; line-height:2.4; color:#888; margin-top:1pt;">${jpn}</div>` : ''}
    </div>`;
}).join('');

const html = `<!DOCTYPE html>
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <title>${titleText}</title>
  <style>
    @font-face {
      font-family: 'BIZUDP';
      src: url('../assets/BIZUDPGothic-Regular.ttf') format('truetype');
    }
    @page { size: A4; margin: 20mm 20mm 20mm 30mm; }
    body {
      font-family: 'BIZUDP', "Hiragino Sans", "Hiragino Kaku Gothic ProN",
                   "Yu Gothic UI", "Yu Gothic", "Meiryo", sans-serif;
      margin: 0; padding: 0; color: #111;
    }
    @media screen {
      body { padding: 28px 32px; max-width: 800px; margin: 0 auto; }
    }
    h1 {
      font-size: ${s.basePt + 2}pt;
      margin: 0 0 ${Math.round(s.basePt * 1.2)}pt;
      padding-bottom: ${Math.round(s.basePt * 0.4)}pt;
      border-bottom: 1px solid #ccc;
      line-height: 1.4;
      font-weight: bold;
    }
    ruby { ruby-align: center; }
    rt { font-size: 0.52em; }
  </style>
</head>
<body>
  <h1>${titleText}</h1>
  ${sentencesHtml}
</body>
</html>`;

// content.id をそのまま出力パスに使う (例: "syntax/1005" → docs/syntax/1005.html)
const outPath = path.join(__dirname, '../docs', `${content.id}.html`);
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, html);
console.log(`docs/${content.id}.html`);
