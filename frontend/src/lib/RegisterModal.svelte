<script>
  let { onClose, onSave, sourceMasters = [], content = null } = $props();

  const isEdit = content !== null;

  let title = $state(content?.title ?? '');
  let sourceMasterId = $state(content?.source_master_id ?? '');
  let englishText = $state('');
  let isSubmitting = $state(false);
  let error = $state('');

  $effect(() => {
    if (isEdit && content?.sentences?.length) {
      englishText = content.sentences.map(s => s.english_text).join(' ');
    }
  });

  async function handleSubmit() {
    if (!title.trim()) { error = 'タイトルは必須です'; return; }
    if (!englishText.trim()) { error = '英文は必須です'; return; }
    error = '';
    isSubmitting = true;

    try {
      const url = isEdit
        ? `http://localhost:3001/api/contents/${content.id}`
        : 'http://localhost:3001/api/contents';

      const res = await fetch(url, {
        method: isEdit ? 'PUT' : 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          title: title.trim(),
          source_master_id: sourceMasterId || null,
          english_text: englishText.trim(),
        }),
      });

      if (!res.ok) throw new Error(isEdit ? '更新に失敗しました' : '登録に失敗しました');
      onSave(await res.json());
    } catch (e) {
      error = e?.message || 'エラーが発生しました';
    } finally {
      isSubmitting = false;
    }
  }
</script>

<div
  class="fixed inset-0 bg-black/30 backdrop-blur-sm z-50 flex items-center justify-center p-4"
  onclick={onClose}
  role="dialog"
  aria-modal="true"
>
  <div
    class="bg-white rounded-xl shadow-xl w-full max-w-2xl max-h-[90vh] overflow-y-auto"
    onclick={(e) => e.stopPropagation()}
    role="presentation"
  >
    <div class="px-6 py-5 border-b border-stone-200 flex items-center justify-between">
      <h2 class="text-base font-semibold text-stone-800">
        {isEdit ? '教材を編集' : '教材を追加'}
      </h2>
      <button
        onclick={onClose}
        class="w-7 h-7 flex items-center justify-center rounded-md text-stone-400 hover:bg-stone-100 transition-colors"
      >
        <span class="material-symbols-rounded text-[18px]">close</span>
      </button>
    </div>

    <div class="px-6 py-5 space-y-4">
      <div>
        <label class="block text-xs font-medium text-stone-600 mb-1.5">タイトル *</label>
        <input
          type="text"
          bind:value={title}
          placeholder="例: #123 Podcast Title"
          class="w-full px-3 py-2 text-sm border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-stone-400 focus:border-transparent"
        />
      </div>

      <!-- 出典（ドロップダウン） -->
      <div>
        <label class="block text-xs font-medium text-stone-600 mb-1.5">出典</label>
        <select
          bind:value={sourceMasterId}
          class="w-full px-3 py-2 text-sm border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-stone-400 focus:border-transparent bg-white"
        >
          <option value="">未設定</option>
          {#each sourceMasters as sm (sm.id)}
            <option value={sm.id}>{sm.name}</option>
          {/each}
        </select>
        {#if sourceMasters.length === 0}
          <p class="mt-1 text-xs text-stone-400">
            出典はサイドバーの
            <span class="material-symbols-rounded text-[12px] align-middle">category</span>
            から先に作成してください
          </p>
        {/if}
      </div>

      <div>
        <label class="block text-xs font-medium text-stone-600 mb-1.5">英文 *</label>
        <textarea
          bind:value={englishText}
          placeholder="英語のテキストをここに貼り付けてください。文単位に自動分割されます。"
          rows="10"
          class="w-full px-3 py-2 text-sm border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-stone-400 focus:border-transparent resize-none font-mono leading-relaxed"
        ></textarea>
      </div>

      {#if isEdit}
        <p class="text-xs text-amber-600">
          <span class="material-symbols-rounded text-[12px] align-middle">warning</span>
          英文を変更すると既存の日本語訳は削除されます
        </p>
      {:else}
        <p class="text-xs text-stone-400">
          <span class="material-symbols-rounded text-[12px] align-middle">info</span>
          日本語訳は登録後に「AI翻訳」ボタンで自動生成できます
        </p>
      {/if}

      {#if error}
        <p class="text-sm text-rose-500">{error}</p>
      {/if}
    </div>

    <div class="px-6 py-4 border-t border-stone-200 flex justify-end gap-2">
      <button
        onclick={onClose}
        class="px-4 py-2 text-sm text-stone-600 hover:bg-stone-100 rounded-md transition-colors"
      >
        キャンセル
      </button>
      <button
        onclick={handleSubmit}
        disabled={isSubmitting}
        class="px-4 py-2 text-sm bg-stone-800 text-white rounded-md hover:bg-stone-700 transition-colors disabled:opacity-50"
      >
        {isSubmitting ? (isEdit ? '更新中...' : '登録中...') : (isEdit ? '更新する' : '登録する')}
      </button>
    </div>
  </div>
</div>
