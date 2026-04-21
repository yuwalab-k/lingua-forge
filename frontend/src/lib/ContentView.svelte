<script>
  import { untrack } from 'svelte';
  import { API_BASE } from './config.js';
  import SentenceCard from './SentenceCard.svelte';
  import PrintModal from './PrintModal.svelte';
  import { ttsRate } from './stores.js';

  let { content, onDelete, onEdit, onUpdate, globalProcessing, onGlobalProcessingChange, aiEnabled = false } = $props();

  const _isThisPage = () => globalProcessing?.contentId === content.id;

  let showJapanese = $state(true);
  let reproductionMode = $state(false);
  let activeSentenceId = $state(null);
  let isTranslating = $state(untrack(() => _isThisPage() && globalProcessing?.type === 'translate'));
  let translateTotal = $state(untrack(() => _isThisPage() ? (globalProcessing?.translateTotal ?? 0) : 0));
  let translateDone = $state(0);
  let aiError = $state('');
  let translateController = $state(untrack(() =>
    _isThisPage() && globalProcessing?.type === 'translate' ? globalProcessing.controller : null
  ));
  let userCancelled = $state(false);
  let pollInterval = null;

  function clearPoll() {
    if (pollInterval) { clearInterval(pollInterval); pollInterval = null; }
  }

  // 翻訳中は常にポーリング（通常翻訳・リロード・ページ遷移後復帰すべてに対応）
  $effect(() => {
    if (!isTranslating || !_isThisPage() || pollInterval !== null) return;

    const initDone = globalProcessing?.initialDone ?? 0;
    const isForce = globalProcessing?.force ?? false;

    pollInterval = setInterval(async () => {
      try {
        const res = await fetch(`${API_BASE}/api/contents/${content.id}`);
        const updated = await res.json();
        onUpdate(updated);
        const nowDone = updated.sentences.filter(s => s.japanese_text).length;
        translateDone = Math.max(0, nowDone - (isForce ? 0 : initDone));
        if (!updated.is_translating && !userCancelled) {
          clearPoll();
          isTranslating = false;
          translateTotal = 0;
          translateDone = 0;
          translateController = null;
          onGlobalProcessingChange(null);
        }
      } catch {}
    }, 2000);

    return () => clearPoll();
  });

  // リロード後にバックエンドで翻訳中の場合、状態を復元する
  $effect(() => {
    if (!content.is_translating || isTranslating || userCancelled) return;
    const untranslated = content.sentences.filter(s => !s.japanese_text).length;
    const alreadyDone = content.sentences.filter(s => s.japanese_text).length;
    isTranslating = true;
    translateTotal = untranslated;
    onGlobalProcessingChange({
      contentId: content.id,
      type: 'translate',
      controller: null,
      translateTotal: untranslated,
      initialDone: alreadyDone,
      force: false,
    });
  });

  const isOtherPageProcessing = $derived(
    globalProcessing !== null && globalProcessing.contentId !== content.id
  );

  async function cancelTranslate() {
    userCancelled = true;
    clearPoll();
    translateController?.abort();
    await fetch(`${API_BASE}/api/contents/${content.id}/translate`, { method: 'DELETE' });
    isTranslating = false;
    translateTotal = 0;
    translateDone = 0;
    translateController = null;
    onGlobalProcessingChange(null);
  }

  async function translate(force = false) {
    userCancelled = false;
    const initialDone = force ? 0 : content.sentences.filter(s => s.japanese_text).length;
    const total = force
      ? content.sentences.length
      : content.sentences.filter(s => !s.japanese_text).length;
    isTranslating = true;
    translateTotal = total;
    translateDone = 0;
    aiError = '';
    const controller = new AbortController();
    translateController = controller;
    onGlobalProcessingChange({
      contentId: content.id,
      type: 'translate',
      controller,
      translateTotal: total,
      initialDone,
      force,
    });

    try {
      const url = `${API_BASE}/api/contents/${content.id}/translate${force ? '?force=true' : ''}`;
      const res = await fetch(url, { method: 'POST', signal: controller.signal });
      if (!res.ok) throw new Error(await res.text() || '翻訳に失敗しました');
      onUpdate(await res.json());
    } catch (e) {
      if (e?.name !== 'AbortError') aiError = e?.message || 'エラーが発生しました';
    } finally {
      clearPoll();
      if (!userCancelled) {
        isTranslating = false;
        translateTotal = 0;
        translateDone = 0;
        translateController = null;
        onGlobalProcessingChange(null);
      }
    }
  }

  function handleSentenceUpdate(updated) {
    const sentences = content.sentences.map(s => s.id === updated.id ? updated : s);
    onUpdate({ ...content, sentences });
  }

  async function handleSentenceDelete(id) {
    try {
      const res = await fetch(`${API_BASE}/api/sentences/${id}`, { method: 'DELETE' });
      if (!res.ok) return;
      onUpdate(await res.json());
    } catch {}
  }

  let insertingAt = $state(null);
  let insertText = $state('');
  let inserting = $state(false);

  function openInsert(index) {
    insertingAt = index;
    insertText = '';
  }

  function closeInsert() {
    insertingAt = null;
    insertText = '';
  }

  async function submitInsert() {
    if (!insertText.trim() || inserting) return;
    inserting = true;
    try {
      const res = await fetch(`${API_BASE}/api/contents/${content.id}/sentences`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ insert_at: insertingAt, english_text: insertText.trim() }),
      });
      if (!res.ok) throw new Error();
      onUpdate(await res.json());
      closeInsert();
    } catch {
    } finally {
      inserting = false;
    }
  }

  const hasUntranslated = $derived(content.sentences.some(s => !s.japanese_text));
  const hasAnyTranslated = $derived(content.sentences.some(s => s.japanese_text));

  const translateLabel = $derived(
    !hasUntranslated ? '再翻訳' : hasAnyTranslated ? '翻訳再開' : 'AI翻訳'
  );

  const practiceTotal = $derived(content.sentences.length);
  const practiceCompletedCount = $derived(
    content.sentences.filter(s => s.text_completed && s.speech_completed).length
  );

  const showRetranslate = import.meta.env.VITE_SHOW_RETRANSLATE === 'true';

  let showPrintModal = $state(false);
</script>

<div class="flex flex-col h-full">
  <!-- Header -->
  <div class="px-8 py-4 border-b border-stone-200 bg-white flex items-center justify-between shrink-0">
    <div>
      <h2 class="text-base font-semibold text-stone-800">{content.title}</h2>
      {#if content.source || content.source_url}
        <p class="inline-flex items-center gap-0.5 text-xs text-stone-400 mt-0.5">
          {#if content.source}{content.source}{/if}
          {#if content.source_url}
            <a
              href={content.source_url}
              target="_blank"
              rel="noopener noreferrer"
              class="text-stone-400 hover:text-stone-600 underline underline-offset-2 ml-1"
            >{content.source_url.replace(/^https?:\/\//, '').slice(0, 40)}{content.source_url.length > 47 ? '…' : ''}
            </a>
            <span class="material-symbols-rounded text-[8px]">open_in_new</span>
          {/if}
        </p>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      <!-- AI翻訳（練習モード中・AI無効時は非表示） -->
      {#if !reproductionMode && aiEnabled && (hasUntranslated || showRetranslate)}
        <button
          onclick={() => translate(!hasUntranslated)}
          disabled={isTranslating || isOtherPageProcessing}
          class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md border transition-colors
            {isTranslating
              ? 'bg-stone-100 text-stone-400 border-stone-200 cursor-wait'
              : isOtherPageProcessing
                ? 'bg-stone-100 text-stone-300 border-stone-200 cursor-not-allowed opacity-50'
                : 'bg-white text-stone-600 border-stone-300 hover:border-stone-500 hover:text-stone-800'}"
        >
          {#if isTranslating}
            <span class="material-symbols-rounded text-[14px] animate-spin">progress_activity</span>
            翻訳中...
          {:else}
            <span class="material-symbols-rounded text-[14px]">translate</span>
            {translateLabel}
          {/if}
        </button>
      {/if}

      <!-- 日本語ON/OFF（練習モード中は非表示） -->
      {#if !reproductionMode}
      <button
        onclick={() => { showJapanese = !showJapanese; }}
        class="flex items-center gap-1 text-xs px-3 py-1.5 rounded-md border transition-colors {showJapanese
          ? 'bg-stone-800 text-white border-stone-800'
          : 'bg-white text-stone-600 border-stone-300 hover:border-stone-400'}"
      >
        <span class="material-symbols-rounded text-[14px]">{showJapanese ? 'visibility' : 'visibility_off'}</span>
        日本語
      </button>
      {/if}

      <button
        onclick={() => { reproductionMode = !reproductionMode; }}
        class="flex items-center gap-1 text-xs px-3 py-1.5 rounded-md border transition-colors {reproductionMode
          ? 'bg-amber-500 text-white border-amber-500'
          : 'bg-white text-stone-600 border-stone-300 hover:border-stone-400'}"
        title="日本語訳を見て英文を書く練習"
      >
        <span class="material-symbols-rounded text-[14px]">edit_note</span>
        練習
      </button>

      <!-- 読み上げ速度 -->
      <label class="flex items-center gap-1.5 text-xs text-stone-500">
        <span class="material-symbols-rounded text-[14px]">speed</span>
        <input
          type="range"
          min="0.3"
          max="1.5"
          step="0.1"
          bind:value={$ttsRate}
          class="w-20 accent-stone-600"
          title="読み上げ速度: {$ttsRate.toFixed(1)}x"
        />
        <span class="w-6 text-right">{$ttsRate.toFixed(1)}</span>
      </label>

      <button
        onclick={() => { showPrintModal = true; }}
        class="w-8 h-8 flex items-center justify-center rounded-md border border-stone-200 text-stone-400 hover:border-stone-400 hover:text-stone-700 transition-colors"
        title="印刷"
      >
        <span class="material-symbols-rounded text-[16px]">print</span>
      </button>

      <button
        onclick={onEdit}
        class="w-8 h-8 flex items-center justify-center rounded-md border border-stone-200 text-stone-400 hover:border-stone-400 hover:text-stone-700 transition-colors"
        title="編集"
      >
        <span class="material-symbols-rounded text-[16px]">edit</span>
      </button>

      <button
        onclick={onDelete}
        class="w-8 h-8 flex items-center justify-center rounded-md border border-stone-200 text-stone-400 hover:border-rose-300 hover:text-rose-500 transition-colors"
        title="削除"
      >
        <span class="material-symbols-rounded text-[16px]">delete</span>
      </button>
    </div>
  </div>

  <!-- 別ページ処理中バナー -->
  {#if isOtherPageProcessing}
    <div class="px-8 py-2.5 bg-amber-50 border-b border-amber-100 flex items-center gap-2.5 shrink-0">
      <span class="material-symbols-rounded text-[16px] text-amber-400 animate-spin">progress_activity</span>
      <p class="text-xs text-amber-600">別ページで AI翻訳 処理中のため操作できません</p>
      <button
        onclick={cancelTranslate}
        class="ml-auto flex items-center gap-1 text-xs px-2.5 py-1 rounded-md border border-amber-200 text-amber-500 hover:bg-amber-100 hover:border-amber-300 transition-colors"
      >
        <span class="material-symbols-rounded text-[14px]">stop_circle</span>
        中止
      </button>
    </div>
  {/if}

  <!-- 翻訳中バナー -->
  {#if isTranslating}
    <div class="px-8 py-2.5 bg-blue-50 border-b border-blue-100 flex items-center gap-2.5 shrink-0">
      <span class="material-symbols-rounded text-[16px] text-blue-400 animate-spin">progress_activity</span>
      <p class="text-xs text-blue-600">
        AI が翻訳処理中です（{translateDone}/{translateTotal}文）
      </p>
      <button
        onclick={cancelTranslate}
        class="ml-auto flex items-center gap-1 text-xs px-2.5 py-1 rounded-md border border-blue-200 text-blue-500 hover:bg-blue-100 hover:border-blue-300 transition-colors"
      >
        <span class="material-symbols-rounded text-[14px]">stop_circle</span>
        中止
      </button>
    </div>
  {/if}

  <!-- 練習モードプログレス -->
  {#if reproductionMode}
    <div class="px-8 py-2.5 bg-amber-50 border-b border-amber-100 flex items-center gap-3 shrink-0">
      <span class="material-symbols-rounded text-[16px] text-amber-500">edit_note</span>
      <span class="text-xs text-amber-700 font-medium">練習モード</span>
      <div class="flex items-center gap-2 ml-auto">
        <span class="text-xs text-amber-600">{practiceCompletedCount} / {practiceTotal} 完了</span>
        <div class="w-32 h-1.5 bg-amber-100 rounded-full overflow-hidden">
          <div
            class="h-full bg-emerald-400 rounded-full transition-all duration-500"
            style="width: {practiceTotal > 0 ? (practiceCompletedCount / practiceTotal) * 100 : 0}%"
          ></div>
        </div>
        {#if practiceCompletedCount === practiceTotal && practiceTotal > 0}
          <span class="flex items-center gap-1 text-xs font-medium text-emerald-600">
            <span class="material-symbols-rounded text-[14px]">celebration</span>
            全完了！
          </span>
        {/if}
      </div>
    </div>
  {/if}

  {#if aiError}
    <div class="px-8 py-2 bg-rose-50 border-b border-rose-100 flex items-center gap-2 shrink-0">
      <span class="material-symbols-rounded text-[14px] text-rose-400">error</span>
      <p class="text-xs text-rose-500">{aiError}</p>
      <button onclick={() => { aiError = ''; }} class="ml-auto text-rose-400 hover:text-rose-600">
        <span class="material-symbols-rounded text-[14px]">close</span>
      </button>
    </div>
  {/if}

  {#snippet InsertRow(index)}
    {#if insertingAt === index}
      <div class="px-8 py-2 flex items-center gap-2 bg-stone-50 border-b border-stone-200">
        <input
          autofocus
          type="text"
          bind:value={insertText}
          onkeydown={(e) => { if (e.key === 'Enter') submitInsert(); if (e.key === 'Escape') closeInsert(); }}
          placeholder="追加するセンテンスを入力..."
          class="flex-1 text-sm px-3 py-1.5 rounded border border-stone-300 focus:outline-none focus:border-stone-500 bg-white"
        />
        <button
          onclick={submitInsert}
          disabled={inserting || !insertText.trim()}
          class="text-xs px-3 py-1.5 rounded border border-stone-300 bg-white text-stone-600 hover:border-stone-500 hover:text-stone-800 disabled:opacity-40 transition-colors"
        >追加</button>
        <button
          onclick={closeInsert}
          class="text-stone-400 hover:text-stone-600"
        ><span class="material-symbols-rounded text-[16px]">close</span></button>
      </div>
    {:else}
      <div class="group flex justify-center py-0.5">
        <button
          onclick={() => openInsert(index)}
          class="opacity-0 group-hover:opacity-100 transition-opacity w-5 h-5 flex items-center justify-center rounded-full bg-sky-100 border border-sky-300 text-sky-500 hover:bg-sky-200 hover:border-sky-400"
        ><span class="material-symbols-rounded text-[12px]">add</span></button>
      </div>
    {/if}
  {/snippet}

  {#if showPrintModal}
    <PrintModal {content} onClose={() => { showPrintModal = false; }} />
  {/if}

  <div class="flex-1 overflow-y-auto">
    {#if !reproductionMode}
      {@render InsertRow(0)}
    {/if}
    {#each content.sentences as sentence (sentence.id)}
      <SentenceCard
        {sentence}
        {showJapanese}
        {reproductionMode}
        isActive={activeSentenceId === sentence.id}
        onClick={() => { activeSentenceId = sentence.id; }}
        onUpdate={handleSentenceUpdate}
        onDelete={handleSentenceDelete}
      />
      {#if !reproductionMode}
        {@render InsertRow(sentence.sentence_index + 1)}
      {/if}
    {/each}
  </div>
</div>
