<script>
  import SentenceCard from './SentenceCard.svelte';

  let { content, onDelete, onEdit, onUpdate, globalProcessing, onGlobalProcessingChange } = $props();

  const _isThisPage = () => globalProcessing?.contentId === content.id;

  let showJapanese = $state(true);
  let activeSentenceId = $state(null);
  let isTranslating = $state(_isThisPage() && globalProcessing?.type === 'translate');
  let translateTotal = $state(0);
  let aiError = $state('');
  let translateController = $state(
    _isThisPage() && globalProcessing?.type === 'translate' ? globalProcessing.controller : null
  );

  const isOtherPageProcessing = $derived(
    globalProcessing !== null && globalProcessing.contentId !== content.id
  );

  async function translate(force = false) {
    isTranslating = true;
    translateTotal = force
      ? content.sentences.length
      : content.sentences.filter(s => !s.japanese_text).length;
    aiError = '';
    const controller = new AbortController();
    translateController = controller;
    onGlobalProcessingChange({ contentId: content.id, type: 'translate', controller });
    try {
      const url = `http://localhost:3001/api/contents/${content.id}/translate${force ? '?force=true' : ''}`;
      const res = await fetch(url, { method: 'POST', signal: controller.signal });
      if (!res.ok) throw new Error(await res.text() || '翻訳に失敗しました');
      onUpdate(await res.json());
    } catch (e) {
      if (e?.name !== 'AbortError') aiError = e?.message || 'エラーが発生しました';
    } finally {
      isTranslating = false;
      translateTotal = 0;
      translateController = null;
      onGlobalProcessingChange(null);
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
      {#if content.source}
        <p class="text-xs text-stone-400 mt-0.5">{content.source}</p>
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
        class="text-xs px-3 py-1.5 rounded-md border transition-colors {showJapanese
          ? 'bg-stone-800 text-white border-stone-800'
          : 'bg-white text-stone-600 border-stone-300 hover:border-stone-400'}"
      >
        日本語 {showJapanese ? 'ON' : 'OFF'}
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
        onclick={() => globalProcessing?.controller?.abort()}
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
        AI が翻訳処理中です（{translateTotal}文）
      </p>
      <button
        onclick={() => translateController?.abort()}
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
        isActive={activeSentenceId === sentence.id}
        onClick={() => { activeSentenceId = sentence.id; }}
        onUpdate={handleSentenceUpdate}
      />
    {/each}
  </div>
</div>
