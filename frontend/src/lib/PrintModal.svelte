<script>
  import fontUrl from '../assets/BIZUDPGothic-Regular.ttf';

  let { content, onClose } = $props();

  let fontSizeKey = $state('large');
  let showJapanese = $state(true);

  const sizeOptions = [
    { key: 'normal',  label: '小',  basePt: 6,  titlePt: 11, gap: 12 },
    { key: 'large',   label: '中',  basePt: 13, titlePt: 18, gap: 20 },
    { key: 'xlarge',  label: '大',  basePt: 20, titlePt: 27, gap: 32 },
  ];

  const currentSize = $derived(sizeOptions.find(s => s.key === fontSizeKey));

  function toRubyHtml(text) {
    if (!text) return '';
    return text.replace(/《([^|》]+)\|([^》]+)》/g, '<ruby>$1<rt>$2</rt></ruby>');
  }

  function escapeHtml(str) {
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function doPrint() {
    const s = currentSize;
    const jpnPt = Math.round(s.basePt * 0.65);

    const sentencesHtml = content.sentences.map((sentence) => {
      const eng = escapeHtml(sentence.english_text);
      const jpn = sentence.japanese_text ? toRubyHtml(sentence.japanese_text) : '';
      return `
        <div style="margin-bottom:${s.gap}pt; page-break-inside:avoid;">
          <div style="font-size:${s.basePt}pt; line-height:1.6; color:#111;">${eng}</div>
          ${showJapanese && jpn ? `<div style="font-size:${jpnPt}pt; line-height:2.4; color:#888; margin-top:1pt;">${jpn}</div>` : ''}
        </div>`;
    }).join('');

    const title = escapeHtml(content.title);
    const source = content.source ? `<p style="font-size:${Math.round(s.basePt * 0.75)}pt; color:#666; margin:0 0 ${s.basePt}pt;">${escapeHtml(content.source)}</p>` : '';

    const html = `<!DOCTYPE html>
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <title>${title}</title>
  <style>
    @font-face {
      font-family: 'BIZUDP';
      src: url('${new URL(fontUrl, window.location.href).href}') format('truetype');
    }
    @page {
      size: A4;
      margin: 18mm 30mm;
      @bottom-center {
        font-family: 'BIZUDP', sans-serif;
        font-size: 8pt;
        color: #aaa;
        content: counter(page);
      }
    }
    body {
      font-family: 'BIZUDP', "Hiragino Sans", "Hiragino Kaku Gothic ProN",
                   "Yu Gothic UI", "Yu Gothic", "Meiryo", sans-serif;
      margin: 0; padding: 0; color: #111;
      -webkit-print-color-adjust: exact;
    }
    h1 {
      font-size: ${s.titlePt}pt;
      margin: 0 0 ${Math.round(s.basePt * 0.6)}pt;
      padding-bottom: ${Math.round(s.basePt * 0.4)}pt;
      border-bottom: 2px solid #ccc;
      line-height: 1.4;
    }
    ruby { ruby-align: center; }
    rt { font-size: 0.52em; }
  </style>
</head>
<body>
  <h1>${title}</h1>
  ${source}
  ${sentencesHtml}
</body>
</html>`;

    const win = window.open('', '_blank', 'width=860,height=700');
    if (!win) { alert('ポップアップがブロックされました。ブラウザの設定でポップアップを許可してください。'); return; }
    win.document.write(html);
    win.document.close();
    win.addEventListener('load', () => {
      win.document.fonts.ready.then(() => win.print());
    });
  }
</script>

<!-- Backdrop -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
  role="dialog"
  aria-modal="true"
>
  <div class="absolute inset-0" onclick={onClose}></div>

  <!-- Panel -->
  <div class="relative bg-white rounded-2xl shadow-2xl w-[380px] overflow-hidden">
    <!-- Header -->
    <div class="flex items-center justify-between px-6 py-4 border-b border-stone-100">
      <div class="flex items-center gap-2">
        <span class="material-symbols-rounded text-[18px] text-stone-500">print</span>
        <h3 class="text-sm font-semibold text-stone-800">印刷設定</h3>
      </div>
      <button
        onclick={onClose}
        class="w-7 h-7 flex items-center justify-center rounded-full text-stone-400 hover:bg-stone-100 hover:text-stone-700 transition-colors"
      >
        <span class="material-symbols-rounded text-[18px]">close</span>
      </button>
    </div>

    <!-- Body -->
    <div class="px-6 py-5 space-y-5">
      <!-- Font size -->
      <div>
        <p class="text-xs font-medium text-stone-500 mb-2.5">文字サイズ</p>
        <div class="flex gap-2">
          {#each sizeOptions as opt}
            <button
              onclick={() => { fontSizeKey = opt.key; }}
              class="flex-1 flex flex-col items-center gap-1.5 py-3 rounded-xl border-2 transition-all
                {fontSizeKey === opt.key
                  ? 'border-stone-700 bg-stone-50'
                  : 'border-stone-200 hover:border-stone-400 bg-white'}"
            >
              <span class="font-semibold leading-none text-stone-800" style="font-size: {opt.basePt * 0.75}px;">あ</span>
              <span class="text-[11px] text-stone-500">{opt.label}</span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Japanese toggle -->
      <div>
        <p class="text-xs font-medium text-stone-500 mb-2.5">内容</p>
        <div class="flex gap-2">
          <button
            onclick={() => { showJapanese = false; }}
            class="flex-1 py-2.5 rounded-xl border-2 text-sm transition-all
              {!showJapanese ? 'border-stone-700 bg-stone-50 font-medium text-stone-800' : 'border-stone-200 hover:border-stone-400 text-stone-500'}"
          >英語のみ</button>
          <button
            onclick={() => { showJapanese = true; }}
            class="flex-1 py-2.5 rounded-xl border-2 text-sm transition-all
              {showJapanese ? 'border-stone-700 bg-stone-50 font-medium text-stone-800' : 'border-stone-200 hover:border-stone-400 text-stone-500'}"
          >英語＋日本語</button>
        </div>
      </div>

      <!-- Preview -->
      <div>
        <p class="text-xs font-medium text-stone-500 mb-2.5">プレビュー</p>
        <div class="rounded-xl border border-stone-200 bg-stone-50 p-4 max-h-44 overflow-hidden">
          {#each content.sentences.slice(0, 3) as sentence}
            <div class="mb-3 last:mb-0" style="font-size: {currentSize.basePt * 0.62}px; line-height: 1.5;">
              <div class="text-stone-800">{sentence.english_text}</div>
              {#if showJapanese && sentence.japanese_text}
                <div class="text-stone-400" style="font-size: {currentSize.basePt * 0.62 * 0.65}px; line-height: 2;">
                  {@html toRubyHtml(sentence.japanese_text)}
                </div>
              {/if}
            </div>
          {/each}
          {#if content.sentences.length > 3}
            <p class="text-[10px] text-stone-400 mt-1">… 全 {content.sentences.length} 文</p>
          {/if}
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="px-6 pb-5 space-y-3">
      <button
        onclick={doPrint}
        class="w-full flex items-center justify-center gap-2 py-3 rounded-xl bg-stone-800 text-white text-sm font-medium hover:bg-stone-700 active:bg-stone-900 transition-colors"
      >
        <span class="material-symbols-rounded text-[18px]">print</span>
        印刷する
      </button>
    </div>
  </div>
</div>

<style>
  ruby { ruby-align: center; }
  rt { font-size: 0.52em; }
</style>
