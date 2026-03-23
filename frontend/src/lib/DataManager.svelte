<script>
  import { API_BASE } from './config.js';

  let { onClose } = $props();

  // ── 教材一覧 ──────────────────────────────────────────────────────────────
  let contents = $state([]);
  let loadError = $state('');
  let selectedIds = $state(new Set());
  let filterSource = $state('');  // '' = 全件

  // 出典の選択肢（重複なし）
  const sourceOptions = $derived(
    [...new Set(contents.map((c) => c.source ?? '').filter(Boolean))].sort((a, b) =>
      a.localeCompare(b, 'ja')
    )
  );

  // フィルタ適用後の一覧
  const filteredContents = $derived(
    filterSource === '' ? contents : contents.filter((c) => (c.source ?? '') === filterSource)
  );

  const allChecked = $derived(
    filteredContents.length > 0 && filteredContents.every((c) => selectedIds.has(c.id))
  );
  const someChecked = $derived(filteredContents.some((c) => selectedIds.has(c.id)));

  async function loadContents() {
    try {
      const res = await fetch(`${API_BASE}/api/contents`);
      contents = await res.json();
    } catch {
      loadError = '教材の読み込みに失敗しました';
    }
  }

  // ── チェックボックス ───────────────────────────────────────────────────────
  function toggleSelect(id) {
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selectedIds = next;
  }

  function toggleAll() {
    const visibleIds = filteredContents.map((c) => c.id);
    const allSelected = visibleIds.every((id) => selectedIds.has(id));
    const next = new Set(selectedIds);
    if (allSelected) {
      visibleIds.forEach((id) => next.delete(id));
    } else {
      visibleIds.forEach((id) => next.add(id));
    }
    selectedIds = next;
  }

  // ── 一括削除 ──────────────────────────────────────────────────────────────
  let deleting = $state(false);
  let deleteResult = $state('');

  async function bulkDelete() {
    if (selectedIds.size === 0) return;
    if (!confirm(`選択した ${selectedIds.size} 件を削除しますか？この操作は取り消せません。`)) return;

    deleting = true;
    deleteResult = '';
    try {
      const res = await fetch(`${API_BASE}/api/contents/bulk-delete`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ ids: [...selectedIds] }),
      });
      const data = await res.json();
      deleteResult = `${data.deleted} 件を削除しました`;
      selectedIds = new Set();
      await loadContents();
    } catch {
      deleteResult = '削除に失敗しました';
    } finally {
      deleting = false;
    }
  }

  // ── エクスポート ───────────────────────────────────────────────────────────
  let exporting = $state(false);

  async function exportCsv() {
    if (selectedIds.size === 0) return;
    exporting = true;
    try {
      const ids = [...selectedIds].join(',');
      const res = await fetch(`${API_BASE}/api/export/contents.csv?ids=${encodeURIComponent(ids)}`);
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'lingua_forge_export.csv';
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      alert('エクスポートに失敗しました');
    } finally {
      exporting = false;
    }
  }

  // ── インポート ─────────────────────────────────────────────────────────────
  let fileInput = $state(null);
  let importing = $state(false);
  let importResult = $state(null);
  let pendingImport = $state(null);  // プレビュー結果
  let pendingCsvText = $state('');   // 確認後に再送するCSVテキスト

  async function handleFileChange(e) {
    const file = e.target.files[0];
    if (!file) return;

    importing = true;
    importResult = null;
    pendingImport = null;
    try {
      const text = await file.text();
      pendingCsvText = text;
      const res = await fetch(`${API_BASE}/api/import/contents`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ csv_text: text, confirmed: false }),
      });
      if (!res.ok) {
        importResult = { imported: 0, updated: 0, skipped: 0, errors: [`サーバーエラー: HTTP ${res.status}`] };
      } else {
        const data = await res.json();
        if (data.to_create > 0 || data.pending_updates.length > 0) {
          pendingImport = data;
        } else {
          // エラーのみの場合はそのまま表示
          importResult = { ...data, imported: 0, updated: 0 };
        }
      }
    } catch {
      importResult = { imported: 0, updated: 0, skipped: 0, errors: ['インポートに失敗しました（サーバーに接続できません）'] };
    } finally {
      importing = false;
      if (fileInput) fileInput.value = '';
    }
  }

  async function executeImport() {
    if (!pendingCsvText) return;
    importing = true;
    try {
      const res = await fetch(`${API_BASE}/api/import/contents`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ csv_text: pendingCsvText, confirmed: true }),
      });
      if (!res.ok) {
        importResult = { imported: 0, updated: 0, skipped: 0, errors: [`サーバーエラー: HTTP ${res.status}`] };
      } else {
        importResult = await res.json();
        await loadContents();
      }
    } catch {
      importResult = { imported: 0, updated: 0, skipped: 0, errors: ['インポートに失敗しました'] };
    } finally {
      importing = false;
      pendingImport = null;
      pendingCsvText = '';
    }
  }

  function cancelImport() {
    pendingImport = null;
    pendingCsvText = '';
  }

  // ── 初期化 ────────────────────────────────────────────────────────────────
  import { onMount } from 'svelte';
  onMount(loadContents);
</script>

<!-- オーバーレイ -->
<div
  class="fixed inset-0 bg-black/40 z-40 flex items-center justify-center"
  role="dialog"
  aria-modal="true"
>
  <div class="bg-white rounded-xl shadow-2xl w-[900px] max-w-[95vw] max-h-[90vh] flex flex-col">
    <!-- ヘッダー -->
    <div class="flex items-center justify-between px-6 py-4 border-b border-stone-200 shrink-0">
      <h2 class="text-base font-semibold text-stone-800">データ管理</h2>
      <button
        onclick={onClose}
        class="w-7 h-7 flex items-center justify-center rounded-md text-stone-400 hover:bg-stone-100 hover:text-stone-700 transition-colors"
      >
        <span class="material-symbols-rounded text-[18px]">close</span>
      </button>
    </div>

    <div class="flex-1 overflow-y-auto p-6 flex flex-col gap-6">


      <!-- インポート -->
      <section class="border border-stone-200 rounded-lg p-4">
        <h3 class="text-sm font-semibold text-stone-700 mb-1">インポート</h3>
        <p class="text-xs text-stone-400 mb-3">
          CSV ファイルから教材を一括登録します。センテンス1行ずつのフォーマットです。<br />
          必須カラム：<code class="bg-stone-100 px-1 rounded">content_title</code>、<code class="bg-stone-100 px-1 rounded">english_text</code>
          ／任意：<code class="bg-stone-100 px-1 rounded">content_id</code>（指定時は既存データを更新）、<code class="bg-stone-100 px-1 rounded">sentence_index</code>、<code class="bg-stone-100 px-1 rounded">japanese_text</code>、<code class="bg-stone-100 px-1 rounded">source</code>、<code class="bg-stone-100 px-1 rounded">source_url</code>
        </p>
        <label class="flex items-center gap-2 cursor-pointer">
          <input
            bind:this={fileInput}
            type="file"
            accept=".csv,text/csv"
            onchange={handleFileChange}
            disabled={importing}
            class="hidden"
          />
          <span
            class="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-stone-100 text-stone-700 rounded-md hover:bg-stone-200 disabled:opacity-50 transition-colors select-none"
          >
            <span class="material-symbols-rounded text-[16px]">upload</span>
            {importing ? 'インポート中...' : 'CSVファイルを選択'}
          </span>
        </label>

        <!-- 確認ダイアログ -->
        {#if pendingImport}
          <div class="mt-3 rounded-md border border-amber-200 bg-amber-50 p-3 text-xs">
            <p class="font-semibold text-amber-800 mb-2">インポート内容を確認してください</p>
            <ul class="text-amber-700 space-y-0.5 mb-3">
              {#if pendingImport.to_create > 0}
                <li>・新規追加：{pendingImport.to_create} 件</li>
              {/if}
              {#if pendingImport.pending_updates.length > 0}
                <li>・更新：{pendingImport.pending_updates.length} 件</li>
              {/if}
            </ul>
            {#if pendingImport.pending_updates.length > 0}
              <div class="border border-amber-200 rounded overflow-hidden mb-3">
                <div class="flex gap-3 px-3 py-1.5 bg-amber-100 text-amber-700 font-medium">
                  <span class="w-6 shrink-0">行</span>
                  <span class="flex-1">現在のタイトル</span>
                  <span class="flex-1">新しいタイトル</span>
                </div>
                <div class="max-h-40 overflow-y-auto divide-y divide-amber-100">
                  {#each pendingImport.pending_updates as u}
                    <div class="flex gap-3 px-3 py-1.5 text-amber-700">
                      <span class="w-6 shrink-0">{u.line}</span>
                      <span class="flex-1 truncate text-stone-500">{u.existing_title}</span>
                      <span class="flex-1 truncate font-medium">{u.new_title}</span>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
            {#if pendingImport.errors.length > 0}
              <ul class="text-amber-600 space-y-0.5 mb-3">
                {#each pendingImport.errors as err}
                  <li>{err}</li>
                {/each}
              </ul>
            {/if}
            <div class="flex gap-2">
              <button
                onclick={executeImport}
                disabled={importing}
                class="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-stone-700 text-white rounded-md hover:bg-stone-600 disabled:opacity-50 transition-colors"
              >
                <span class="material-symbols-rounded text-[16px]">check</span>
                {importing ? '実行中...' : '確認して実行'}
              </button>
              <button
                onclick={cancelImport}
                disabled={importing}
                class="px-3 py-1.5 text-sm text-stone-500 hover:text-stone-700 transition-colors"
              >
                キャンセル
              </button>
            </div>
          </div>
        {/if}

        {#if importResult}
          <div class="mt-3 text-xs rounded-md p-3 {importResult.errors.length > 0 ? 'bg-amber-50 border border-amber-200' : 'bg-green-50 border border-green-200'}">
            <p class="font-medium {importResult.errors.length > 0 ? 'text-amber-700' : 'text-green-700'}">
              {#if importResult.imported > 0}新規追加 {importResult.imported} 件{/if}
              {#if importResult.imported > 0 && importResult.updated > 0}／{/if}
              {#if importResult.updated > 0}更新 {importResult.updated} 件{/if}
              {#if importResult.imported === 0 && importResult.updated === 0}インポート完了（0件）{/if}
              {#if importResult.skipped > 0}／{importResult.skipped} 件スキップ{/if}
            </p>
            {#if importResult.errors.length > 0}
              <ul class="mt-1 text-amber-600 space-y-0.5">
                {#each importResult.errors as err}
                  <li>{err}</li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}
      </section>

      <!-- 一括削除 -->
      <section class="border border-stone-200 rounded-lg p-4">
        <div class="flex items-center justify-between mb-3">
          <div>
            <h3 class="text-sm font-semibold text-stone-700">エクスポート / 一括削除</h3>
            <p class="text-xs text-stone-400 mt-0.5">教材にチェックを入れてCSV出力または削除できます</p>
          </div>
          {#if selectedIds.size > 0}
            <div class="flex items-center gap-2">
              <button
                onclick={exportCsv}
                disabled={exporting}
                class="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-stone-700 text-white rounded-md hover:bg-stone-600 disabled:opacity-50 transition-colors"
              >
                <span class="material-symbols-rounded text-[16px]">download</span>
                {exporting ? 'エクスポート中...' : `${selectedIds.size} 件をCSV出力`}
              </button>
              <button
                onclick={bulkDelete}
                disabled={deleting}
                class="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-50 transition-colors"
              >
                <span class="material-symbols-rounded text-[16px]">delete</span>
                {deleting ? '削除中...' : `${selectedIds.size} 件を削除`}
              </button>
            </div>
          {/if}
        </div>

        {#if deleteResult}
          <p class="text-xs text-green-700 bg-green-50 border border-green-200 rounded px-3 py-2 mb-3">{deleteResult}</p>
        {/if}

        {#if loadError}
          <p class="text-xs text-red-600">{loadError}</p>
        {:else if contents.length === 0}
          <p class="text-xs text-stone-400 text-center py-6">教材がありません</p>
        {:else}
          <!-- 出典フィルタ -->
          <div class="flex items-center gap-2 mb-2">
            <span class="text-xs text-stone-500 shrink-0">出典で絞り込み：</span>
            <select
              bind:value={filterSource}
              class="text-xs border border-stone-200 rounded px-2 py-1 bg-white text-stone-700 focus:outline-none focus:ring-1 focus:ring-stone-400"
            >
              <option value="">すべて</option>
              {#each sourceOptions as src}
                <option value={src}>{src}</option>
              {/each}
            </select>
            {#if filterSource !== ''}
              <button
                onclick={() => { filterSource = ''; }}
                class="text-xs text-stone-400 hover:text-stone-600 transition-colors"
              >クリア</button>
            {/if}
          </div>

          <div class="border border-stone-200 rounded-md overflow-hidden">
            <!-- テーブルヘッダー -->
            <div class="flex items-center gap-3 px-3 py-2 bg-stone-50 border-b border-stone-200">
              <input
                type="checkbox"
                checked={allChecked}
                indeterminate={!allChecked && someChecked}
                onchange={toggleAll}
                class="w-3.5 h-3.5 rounded border-stone-300 text-stone-700 cursor-pointer"
              />
              <span class="text-xs font-medium text-stone-500 flex-1">タイトル</span>
              <span class="text-xs font-medium text-stone-500 w-32 shrink-0">出典</span>
              <span class="text-xs font-medium text-stone-500 w-36 shrink-0">登録日</span>
            </div>
            <!-- テーブルボディ -->
            <div class="max-h-[320px] overflow-y-auto divide-y divide-stone-100">
              {#each filteredContents as content (content.id)}
                <label class="flex items-center gap-3 px-3 py-2 hover:bg-stone-50 cursor-pointer transition-colors">
                  <input
                    type="checkbox"
                    checked={selectedIds.has(content.id)}
                    onchange={() => toggleSelect(content.id)}
                    class="w-3.5 h-3.5 rounded border-stone-300 text-stone-700 cursor-pointer shrink-0"
                  />
                  <span class="text-sm text-stone-700 truncate flex-1">{content.title}</span>
                  <span class="text-xs text-stone-400 w-32 shrink-0 truncate">{content.source ?? '—'}</span>
                  <span class="text-xs text-stone-400 w-36 shrink-0">
                    {new Date(content.created_at).toLocaleDateString('ja-JP')}
                  </span>
                </label>
              {/each}
              {#if filteredContents.length === 0}
                <p class="text-xs text-stone-400 text-center py-6">該当する教材がありません</p>
              {/if}
            </div>
          </div>
          <p class="mt-2 text-xs text-stone-400 text-right">
            {#if filterSource !== ''}
              {filteredContents.length} 件表示 / 全 {contents.length} 件
            {:else}
              全 {contents.length} 件
            {/if}
          </p>
        {/if}
      </section>

    </div>
  </div>
</div>
