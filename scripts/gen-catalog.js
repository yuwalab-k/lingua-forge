#!/usr/bin/env node
/**
 * json/ を全スキャンして docs/index.html を生成する
 * データはビルド時に HTML へ直接埋め込む（fetch 不要、サーバー不要）
 *
 * Usage: node scripts/gen-catalog.js
 */

const fs = require('fs');
const path = require('path');

const JSON_DIR = path.join(__dirname, '../json');
const OUT_PATH = path.join(__dirname, '../docs/index.html');

// json/{source}/{episode}.json を全走査してメタデータを収集
const contents = [];
for (const source of fs.readdirSync(JSON_DIR).sort()) {
  const sourceDir = path.join(JSON_DIR, source);
  if (!fs.statSync(sourceDir).isDirectory()) continue;

  for (const file of fs.readdirSync(sourceDir).sort()) {
    if (!file.endsWith('.json')) continue;
    const episode = path.basename(file, '.json');
    const id = `${source}/${episode}`;
    try {
      const c = JSON.parse(fs.readFileSync(path.join(sourceDir, file), 'utf8'));
      contents.push({
        id,
        title: c.title || id,
        source: c.source || source,
        source_url: c.source_url || null,
        sentence_count: Array.isArray(c.sentences) ? c.sentences.length : 0,
      });
    } catch {
      console.warn(`  skip: ${id} (parse error)`);
    }
  }
}

const embeddedData = JSON.stringify({ contents });

const html = `<!DOCTYPE html>
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Lingua Forge</title>
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

    body {
      font-family: -apple-system, "Hiragino Sans", "Hiragino Kaku Gothic ProN",
                   "Yu Gothic UI", sans-serif;
      background: #fafaf9;
      color: #1c1917;
      min-height: 100svh;
    }

    header {
      position: sticky;
      top: 0;
      z-index: 10;
      background: #fff;
      border-bottom: 1px solid #e7e5e4;
      padding: 14px 16px 12px;
    }

    .header-top {
      display: flex;
      align-items: center;
      gap: 10px;
      margin-bottom: 10px;
    }

    h1 {
      font-size: 15px;
      font-weight: 700;
      letter-spacing: .04em;
      color: #292524;
      flex: 1;
    }

    #total { font-size: 12px; color: #a8a29e; }

    #search {
      width: 100%;
      padding: 8px 12px;
      border: 1px solid #d6d3d1;
      border-radius: 8px;
      font-size: 14px;
      background: #fafaf9;
      outline: none;
      transition: border-color .15s;
    }
    #search:focus { border-color: #78716c; background: #fff; }

    #filter-source {
      width: 100%;
      margin-top: 8px;
      padding: 7px 10px;
      border: 1px solid #d6d3d1;
      border-radius: 8px;
      font-size: 13px;
      color: #57534e;
      background: #fff;
      outline: none;
      cursor: pointer;
    }
    #filter-source:focus { border-color: #78716c; }

    main { padding: 0 16px 32px; }

    .section { margin-top: 20px; }
    .section.hidden { display: none; }

    .section-header {
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 6px 0 8px;
      border-bottom: 1px solid #e7e5e4;
      margin-bottom: 2px;
    }

    .section-name {
      font-size: 11px;
      font-weight: 600;
      color: #a8a29e;
      text-transform: uppercase;
      letter-spacing: .08em;
      flex: 1;
    }

    .section-count { font-size: 11px; color: #d6d3d1; }

    .item {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 10px 0;
      border-bottom: 1px solid #f5f5f4;
    }
    .item.hidden { display: none; }

    .item-title {
      flex: 1;
      font-size: 14px;
      color: #1c1917;
      line-height: 1.4;
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .item-meta { font-size: 11px; color: #a8a29e; flex-shrink: 0; }

    .item-link {
      flex-shrink: 0;
      padding: 5px 12px;
      border-radius: 6px;
      background: #f5f5f4;
      color: #57534e;
      font-size: 12px;
      font-weight: 500;
      text-decoration: none;
      transition: background .15s;
    }
    .item-link:hover { background: #e7e5e4; }

    #empty {
      display: none;
      text-align: center;
      padding: 48px 16px;
      color: #a8a29e;
      font-size: 14px;
    }
  </style>
</head>
<body>
  <header>
    <div class="header-top">
      <h1>Lingua Forge</h1>
      <span id="total"></span>
    </div>
    <input type="search" id="search" placeholder="タイトルで絞り込み..." autocomplete="off">
    <select id="filter-source"></select>
  </header>

  <main id="main"></main>
  <div id="empty">教材が見つかりませんでした</div>

  <script>
    const DATA = ${embeddedData};
    let activeSource = '__all__';

    (function init() {
      const { contents } = DATA;
      document.getElementById('total').textContent = contents.length + ' 教材';

      const sources = ['__all__', ...new Set(contents.map(c => c.source || 'その他'))];
      const sel = document.getElementById('filter-source');
      sources.forEach(src => {
        const opt = document.createElement('option');
        opt.value = src;
        opt.textContent = src === '__all__' ? '全カテゴリ' : src;
        sel.appendChild(opt);
      });
      sel.addEventListener('change', () => {
        activeSource = sel.value;
        applyFilter();
      });

      const groups = {};
      for (const c of contents) {
        const key = c.source || 'その他';
        if (!groups[key]) groups[key] = [];
        groups[key].push(c);
      }

      const mainEl = document.getElementById('main');
      for (const [source, items] of Object.entries(groups)) {
        items.sort((a, b) => {
          const na = parseInt((a.title.match(/#(\d+)/) || [])[1] || 0);
          const nb = parseInt((b.title.match(/#(\d+)/) || [])[1] || 0);
          return nb - na;
        });

        const section = document.createElement('section');
        section.className = 'section';
        section.dataset.source = source;
        section.innerHTML = \`
          <div class="section-header">
            <span class="section-name">\${esc(source)}</span>
            <span class="section-count">\${items.length}</span>
          </div>
        \`;

        for (const c of items) {
          const div = document.createElement('div');
          div.className = 'item';
          div.dataset.title = c.title.toLowerCase();
          div.dataset.source = source;
          div.innerHTML = \`
            <span class="item-title">\${esc(c.title)}</span>
            <span class="item-meta">\${c.sentence_count}文</span>
            <a class="item-link" href="./\${c.id}.html">開く</a>
          \`;
          section.appendChild(div);
        }
        mainEl.appendChild(section);
      }

      applyFilter();
    })();

    function applyFilter() {
      const query = document.getElementById('search').value.toLowerCase().trim();
      let anyVisible = false;
      document.querySelectorAll('.section').forEach(sec => {
        const srcMatch = activeSource === '__all__' || sec.dataset.source === activeSource;
        if (!srcMatch) { sec.classList.add('hidden'); return; }
        let secHasVisible = false;
        sec.querySelectorAll('.item').forEach(item => {
          const ok = !query || item.dataset.title.includes(query);
          item.classList.toggle('hidden', !ok);
          if (ok) secHasVisible = true;
        });
        sec.classList.toggle('hidden', !secHasVisible);
        if (secHasVisible) anyVisible = true;
      });
      document.getElementById('empty').style.display = anyVisible ? 'none' : 'block';
    }

    document.getElementById('search').addEventListener('input', applyFilter);

    function esc(s) {
      return (s || '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    }
  </script>
</body>
</html>`;

fs.writeFileSync(OUT_PATH, html);
console.log(`docs/index.html を生成しました (${contents.length} 教材)`);
