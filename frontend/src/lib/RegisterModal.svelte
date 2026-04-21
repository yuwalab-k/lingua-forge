<script>
  import { API_BASE, TRANSCRIBE_BASE } from './config.js';
  let { onClose, onSave, sourceMasters = [], content = null } = $props();

  const isEdit = content !== null;

  let title = $state(content?.title ?? '');
  let sourceMasterId = $state(content?.source_master_id ?? '');
  let sourceUrl = $state(content?.source_url ?? '');
  let englishText = $state('');
  let isSubmitting = $state(false);
  let error = $state('');

  // 文字起こし
  let transcribeUrl = $state('');
  let selectedFile = $state(null);
  let isTranscribing = $state(false);
  let transcribeError = $state('');
  let transcribeController = $state(null);
  let fileInput;

  $effect(() => {
    if (isEdit && content?.sentences?.length) {
      englishText = content.sentences.map(s => s.english_text).join(' ');
    }
  });


  function handleClose() {
    if (isTranscribing) {
      if (!confirm('文字起こし中です。中止してモーダルを閉じますか？')) return;
      transcribeController?.abort();
    }
    onClose();
  }

  async function handleTranscribe() {
    if (!transcribeUrl && !selectedFile) return;
    isTranscribing = true;
    transcribeError = '';
    const controller = new AbortController();
    transcribeController = controller;
    try {
      let res;
      if (transcribeUrl) {
        res = await fetch(`${TRANSCRIBE_BASE}/transcribe/url`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ url: transcribeUrl }),
          signal: controller.signal,
        });
      } else {
        const formData = new FormData();
        formData.append('file', selectedFile);
        res = await fetch(`${TRANSCRIBE_BASE}/transcribe/file`, {
          method: 'POST',
          body: formData,
          signal: controller.signal,
        });
      }
      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.detail || '文字起こしに失敗しました');
      }
      const data = await res.json();
      englishText = data.text;
      if (data.title && !title.trim()) title = data.title;
      if (transcribeUrl && !sourceUrl.trim()) sourceUrl = transcribeUrl;
      transcribeUrl = '';
      selectedFile = null;
    } catch (e) {
      if (e?.name !== 'AbortError') transcribeError = e?.message || 'エラーが発生しました';
    } finally {
      isTranscribing = false;
      transcribeController = null;
    }
  }

  async function handleSubmit() {
    if (!title.trim()) { error = 'タイトルは必須です'; return; }
    if (!sourceMasterId) { error = '出典は必須です'; return; }
    if (!isEdit && !englishText.trim()) { error = '英文は必須です'; return; }
    error = '';
    isSubmitting = true;

    try {
      const url = isEdit
        ? `${API_BASE}/api/contents/${content.id}`
        : `${API_BASE}/api/contents`;

      const body = isEdit
        ? { title: title.trim(), source_master_id: sourceMasterId || null, source_url: sourceUrl.trim() || null }
        : { title: title.trim(), source_master_id: sourceMasterId || null, source_url: sourceUrl.trim() || null, english_text: englishText.trim() };

      const res = await fetch(url, {
        method: isEdit ? 'PUT' : 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
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
        onclick={handleClose}
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

      <div>
        <label class="block text-xs font-medium text-stone-600 mb-1.5">出典 *</label>
        <select
          bind:value={sourceMasterId}
          class="w-full px-3 py-2 text-sm border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-stone-400 focus:border-transparent bg-white"
        >
          <option value="">選択してください</option>
          {#each sourceMasters as sm (sm.id)}
            <option value={sm.id}>{sm.name}</option>
          {/each}
        </select>
      </div>

      <div>
        <label class="block text-xs font-medium text-stone-600 mb-1.5">参照元URL</label>
        <input
          type="url"
          bind:value={sourceUrl}
          placeholder="https://..."
          class="w-full px-3 py-2 text-sm border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-stone-400 focus:border-transparent"
        />
      </div>

      <!-- 文字起こし -->
      {#if !isEdit}
        <div class="rounded-lg border border-stone-200 p-3 space-y-2 bg-stone-50">
          <p class="text-xs font-medium text-stone-500">文字起こしで英文を入力</p>
          <div class="flex gap-2">
            <input
              type="url"
              bind:value={transcribeUrl}
              placeholder="YouTube / Podcast URL"
              disabled={isTranscribing || !!selectedFile}
              class="flex-1 px-3 py-1.5 text-xs border border-stone-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-stone-400 disabled:opacity-50"
              oninput={() => { selectedFile = null; }}
            />
            <span class="text-xs text-stone-300 self-center">または</span>
            <input
              type="file"
              accept="audio/*,video/*,.mp3,.mp4,.m4a,.wav"
              class="hidden"
              bind:this={fileInput}
              onchange={(e) => { selectedFile = e.currentTarget.files?.[0] ?? null; transcribeUrl = ''; }}
            />
            <button
              type="button"
              onclick={() => fileInput.click()}
              disabled={isTranscribing || !!transcribeUrl}
              class="text-xs px-3 py-1.5 rounded-md border border-stone-300 bg-white text-stone-600 hover:border-stone-400 transition-colors disabled:opacity-50 whitespace-nowrap"
            >
              {selectedFile ? selectedFile.name.slice(0, 12) + '…' : 'MP3 / 動画'}
            </button>
            <button
              type="button"
              onclick={handleTranscribe}
              disabled={isTranscribing || (!transcribeUrl && !selectedFile)}
              class="flex items-center gap-1 text-xs px-3 py-1.5 rounded-md border transition-colors whitespace-nowrap
                {isTranscribing
                  ? 'bg-stone-100 text-stone-400 border-stone-200 cursor-wait'
                  : 'bg-stone-800 text-white border-stone-800 hover:bg-stone-700 disabled:opacity-40'}"
            >
              {#if isTranscribing}
                <span class="material-symbols-rounded text-[13px] animate-spin">progress_activity</span>
                処理中...
              {:else}
                <span class="material-symbols-rounded text-[13px]">mic</span>
                文字起こし
              {/if}
            </button>
          </div>
          {#if isTranscribing}
            <p class="text-xs text-stone-400">長い音声は数分かかります...</p>
          {/if}
          {#if transcribeError}
            <p class="text-xs text-rose-500">{transcribeError}</p>
          {/if}
        </div>
      {/if}

      {#if !isEdit}
        <div>
          <label class="block text-xs font-medium text-stone-600 mb-1.5">英文 *</label>
          <textarea
            bind:value={englishText}
            placeholder="英語のテキストをここに貼り付けてください。文単位に自動分割されます。"
            rows="10"
            class="w-full px-3 py-2 text-sm border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-stone-400 focus:border-transparent resize-none font-mono leading-relaxed"
          ></textarea>
        </div>
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
        onclick={handleClose}
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
