<script>
  import { API_BASE } from './config.js';
  import SentenceCard from './SentenceCard.svelte';

  let { content, onDelete, onEdit, onUpdate, globalProcessing, onGlobalProcessingChange } = $props();

  const _isThisPage = () => globalProcessing?.contentId === content.id;

  let showJapanese = $state(true);
  let reproductionMode = $state(false);
  let activeSentenceId = $state(null);
  let isTranslating = $state(_isThisPage() && globalProcessing?.type === 'translate');
  let translateTotal = $state(0);
  let translateDone = $state(0);
  let aiError = $state('');
  let translateController = $state(
    _isThisPage() && globalProcessing?.type === 'translate' ? globalProcessing.controller : null
  );
  // ユーザーが中止操作をしたか（$effect の再トリガー防止用）
  let userCancelled = $state(false);
  let pollInterval = null;

  function clearPoll() {
    if (pollInterval) { clearInterval(pollInterval); pollInterval = null; }
  }

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

  // リロード後にバックエンドで翻訳中の場合、ポーリングで状態を同期する
  $effect(() => {
    if (!content.is_translating || isTranslating || userCancelled) return;

    isTranslating = true;
    translateTotal = content.sentences.filter(s => !s.japanese_text).length;
    onGlobalProcessingChange({ contentId: content.id, type: 'translate', controller: null });

    const initialDone = content.sentences.filter(s => s.japanese_text).length;
    const interval = setInterval(async () => {
      try {
        const res = await fetch(`${API_BASE}/api/contents/${content.id}`);
        const updated = await res.json();
        onUpdate(updated);
        translateDone = Math.max(0, updated.sentences.filter(s => s.japanese_text).length - initialDone);
        if (!updated.is_translating) {
          clearInterval(interval);
          isTranslating = false;
          translateTotal = 0;
          translateDone = 0;
          onGlobalProcessingChange(null);
        }
      } catch {}
    }, 2000);

    return () => clearInterval(interval);
  });

  const isOtherPageProcessing = $derived(
    globalProcessing !== null && globalProcessing.contentId !== content.id
  );

  async function translate(force = false) {
    userCancelled = false;
    isTranslating = true;
    const initialDone = content.sentences.filter(s => s.japanese_text).length;
    translateTotal = force
      ? content.sentences.length
      : content.sentences.filter(s => !s.japanese_text).length;
    translateDone = 0;
    aiError = '';
    const controller = new AbortController();
    translateController = controller;
    onGlobalProcessingChange({ contentId: content.id, type: 'translate', controller });

    // 2秒ごとにポーリングして1文ずつリアルタイム反映
    pollInterval = setInterval(async () => {
      try {
        const res = await fetch(`${API_BASE}/api/contents/${content.id}`);
        const updated = await res.json();
        onUpdate(updated);
        const nowDone = updated.sentences.filter(s => s.japanese_text).length;
        translateDone = Math.max(0, nowDone - (force ? 0 : initialDone));
      } catch {}
    }, 2000);

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

  const hasUntranslated = $derived(content.sentences.some(s => !s.japanese_text));
  const hasAnyTranslated = $derived(content.sentences.some(s => s.japanese_text));

  const translateLabel = $derived(
    !hasUntranslated ? '再翻訳' : hasAnyTranslated ? '翻訳再開' : 'AI翻訳'
  );
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
      <!-- AI翻訳 -->
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

      <button
        onclick={() => { showJapanese = !showJapanese; }}
        class="flex items-center gap-1 text-xs px-3 py-1.5 rounded-md border transition-colors {showJapanese
          ? 'bg-stone-800 text-white border-stone-800'
          : 'bg-white text-stone-600 border-stone-300 hover:border-stone-400'}"
      >
        <span class="material-symbols-rounded text-[14px]">{showJapanese ? 'visibility' : 'visibility_off'}</span>
        日本語
      </button>

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

  {#if aiError}
    <div class="px-8 py-2 bg-rose-50 border-b border-rose-100 flex items-center gap-2 shrink-0">
      <span class="material-symbols-rounded text-[14px] text-rose-400">error</span>
      <p class="text-xs text-rose-500">{aiError}</p>
      <button onclick={() => { aiError = ''; }} class="ml-auto text-rose-400 hover:text-rose-600">
        <span class="material-symbols-rounded text-[14px]">close</span>
      </button>
    </div>
  {/if}

  <div class="flex-1 overflow-y-auto">
    {#each content.sentences as sentence (sentence.id)}
      <SentenceCard
        {sentence}
        {showJapanese}
        {reproductionMode}
        isActive={activeSentenceId === sentence.id}
        onClick={() => { activeSentenceId = sentence.id; }}
        onUpdate={handleSentenceUpdate}
      />
    {/each}
  </div>
</div>
