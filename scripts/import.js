#!/usr/bin/env node
/**
 * テキストファイルを 1教材の JSON に変換するスクリプト
 *
 * ファイル名規則:  {source}-#{episode}.txt
 *   例: syntax-#1005.txt   → json/syntax/1005.json  /  docs/syntax/1005.html
 *       U001-#001.txt      → json/U001/001.json     /  docs/U001/001.html
 *
 * ─── ヘッダー付きテキストファイル ───
 *   title: Episode Title
 *   source: Podcast Name   ← 省略時はファイル名から自動抽出
 *   url: https://...
 *   ---
 *   English text here.
 *
 *   Usage: node scripts/import.js texts/syntax-#1005.txt
 *
 * 生成物:
 *   json/{source}/{episode}.json   （japanese_text は null）
 *   docs/index.json                （更新）
 */

const fs = require('fs');
const path = require('path');

const JSON_DIR = path.join(__dirname, '../json');

// ファイルパスから source / episode / id を抽出
//
// フォルダ構造（推奨）:
//   texts/syntax/1005.txt  → { source: 'syntax', episode: '1005', id: 'syntax/1005' }
//   texts/U001/001.txt     → { source: 'U001',   episode: '001',  id: 'U001/001' }
//
// ファイル名パターン（旧形式・フォールバック）:
//   syntax-#1005.txt       → { source: 'syntax', episode: '1005', id: 'syntax/1005' }
//
function parseFilename(filePath) {
  const absPath = path.resolve(filePath);
  const textsDir = path.resolve(path.join(__dirname, '../texts'));
  const rel = path.relative(textsDir, absPath);
  const parts = rel.split(path.sep);

  // texts/{source}/{episode}.txt 形式
  if (parts.length === 2) {
    const source = parts[0];
    const episode = path.basename(parts[1], path.extname(parts[1]));
    return { source, episode, id: `${source}/${episode}` };
  }

  // フォールバック: ファイル名から {source}-#{episode} を抽出
  const filename = path.basename(filePath, path.extname(filePath));
  const m = filename.match(/^(.+)-#(\d+)$/);
  if (m) {
    return { source: m[1], episode: m[2], id: `${m[1]}/${m[2]}` };
  }

  return { source: null, episode: null, id: filename };
}

// --- センテンス分割（Rust版と同じロジック）---
const ABBREVIATIONS = new Set([
  'mr', 'mrs', 'ms', 'dr', 'prof', 'sr', 'jr', 'vs', 'etc', 'inc', 'ltd', 'corp',
  'dept', 'est', 'approx', 'e.g', 'i.e', 'fig', 'vol', 'no', 'pp', 'ed',
  'jan', 'feb', 'mar', 'apr', 'jun', 'jul', 'aug', 'sep', 'oct', 'nov', 'dec',
  'u.s', 'u.k', 'u.n',
]);

function isAbbreviation(word) {
  const lower = word.toLowerCase();
  if (lower.length === 1 && /[a-z]/.test(lower)) return true;
  return ABBREVIATIONS.has(lower);
}

function splitSentences(text) {
  const sentences = [];
  let current = '';
  const chars = [...text];
  const len = chars.length;

  for (let i = 0; i < len; i++) {
    const ch = chars[i];
    current += ch;

    if ('.!?'.includes(ch)) {
      if (ch === '.') {
        const beforeDot = current.slice(0, -1).trimEnd();
        const lastWord = beforeDot.split(/[^a-zA-Z0-9.]/).pop() || '';
        if (isAbbreviation(lastWord)) continue;

        const rest = chars.slice(i + 1).join('');
        const match = rest.match(/\S/);
        if (match && /[a-z]/.test(match[0])) continue;
      }

      const trimmed = current.trim();
      if (trimmed) sentences.push(trimmed);
      current = '';
    }
  }

  const remaining = current.trim();
  if (remaining) sentences.push(remaining);

  return sentences;
}

// --- ヘッダー付きファイルのパース ---
function parseFile(filePath) {
  const raw = fs.readFileSync(filePath, 'utf8');
  const sepIdx = raw.indexOf('\n---\n');
  if (sepIdx === -1) {
    return { title: null, source: null, url: null, body: raw };
  }

  const headerBlock = raw.slice(0, sepIdx);
  const body = raw.slice(sepIdx + 5);

  const meta = { title: null, source: null, url: null };
  for (const line of headerBlock.split('\n')) {
    const m = line.match(/^(\w+):\s*(.+)$/);
    if (m) meta[m[1].toLowerCase()] = m[2].trim();
  }

  return { ...meta, body };
}

// --- 引数パース ---
const args = process.argv.slice(2);
if (args.length === 0) {
  console.error('Usage: node scripts/import.js texts/syntax-#1005.txt');
  process.exit(1);
}

const filePath = args[0];
if (!fs.existsSync(filePath)) {
  console.error(`ERROR: ファイルが見つかりません: ${filePath}`);
  process.exit(1);
}

const { source: fileSource, episode, id } = parseFilename(filePath);
const parsed = parseFile(filePath);

const title = parsed.title || id;
const source = parsed.source || fileSource || null;
const sourceUrl = parsed.url || null;
const textBody = parsed.body;

// --- 処理 ---
const sentences = splitSentences(textBody.trim());
if (sentences.length === 0) {
  console.error('ERROR: センテンスが見つかりませんでした');
  process.exit(1);
}

const contentJson = {
  id,
  title,
  source,
  source_url: sourceUrl,
  sentences: sentences.map((text, i) => ({
    sentence_index: i,
    english_text: text,
    japanese_text: null,
  })),
};

// json/{source}/{episode}.json
const jsonOutDir = episode ? path.join(JSON_DIR, fileSource) : JSON_DIR;
const jsonFilename = episode ? `${episode}.json` : `${id}.json`;
fs.mkdirSync(jsonOutDir, { recursive: true });
const contentPath = path.join(jsonOutDir, jsonFilename);
fs.writeFileSync(contentPath, JSON.stringify(contentJson, null, 2));

console.log(`Done: "${title}" を ${sentences.length} センテンスに分割しました`);
console.log(`  ${path.relative(process.cwd(), contentPath)}`);
console.log('');
console.log('次のステップ:');
console.log(`  1. ${path.relative(process.cwd(), contentPath)} を Claude に渡して japanese_text を埋めてもらう`);
console.log(`  2. node scripts/gen-html.js ${path.relative(process.cwd(), contentPath)}`);
console.log('  3. node scripts/gen-catalog.js');
console.log('  4. git add docs/ && git commit && git push');
