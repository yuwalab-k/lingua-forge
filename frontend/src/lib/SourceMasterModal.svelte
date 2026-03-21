<script>
  let { sourceMasters, onClose, onChanged } = $props();

  let selected = $state(null);
  let isNew = $state(false);
  let name = $state('');
  let isSaving = $state(false);
  let isDeleting = $state(false);
  let error = $state('');

  function selectSource(sm) {
    selected = sm;
    isNew = false;
    name = sm.name;
    error = '';
  }

  function startNew() {
    selected = null;
    isNew = true;
    name = '';
    error = '';
  }

  async function handleSave() {
    if (!name.trim()) { error = '名前は必須です'; return; }
    error = '';
    isSaving = true;
    try {
      const url = isNew
        ? 'http://localhost:3001/api/sources'
        : `http://localhost:3001/api/sources/${selected.id}`;
      const res = await fetch(url, {
        method: isNew ? 'POST' : 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: name.trim() }),
      });
      if (!res.ok) throw new Error('保存に失敗しました');
      const saved = await res.json();
      onChanged();
      selectSource(saved);
    } catch (e) {
      error = e?.message || 'エラーが発生しました';
    } finally {
      isSaving = false;
    }
  }

  async function handleDelete() {
    if (!selected) return;
    if (!confirm(`「${selected.name}」を削除しますか？\n紐づく教材の出典設定は解除されます。`)) return;
    isDeleting = true;
    try {
      await fetch(`http://localhost:3001/api/sources/${selected.id}`, { method: 'DELETE' });
      onChanged();
      selected = null;
      isNew = false;
      name = '';
    } catch {
      error = '削除に失敗しました';
    } finally {
      isDeleting = false;
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
    class="bg-white rounded-xl shadow-xl w-full max-w-md"
    onclick={(e) => e.stopPropagation()}
    role="presentation"
  >
    <!-- Header -->
    <div class="px-6 py-4 border-b border-stone-200 flex items-center justify-between">
      <h2 class="text-base font-semibold text-stone-800">出典管理</h2>
      <button
        onclick={onClose}
        class="w-7 h-7 flex items-center justify-center rounded-md text-stone-400 hover:bg-stone-100 transition-colors"
      >
        <span class="material-symbols-rounded text-[18px]">close</span>
      </button>
    </div>

    <!-- List -->
    <div class="px-6 py-4 space-y-1 max-h-60 overflow-y-auto">
      {#each sourceMasters as sm (sm.id)}
        <button
          onclick={() => selectSource(sm)}
          class="w-full text-left px-3 py-2 rounded-md text-sm transition-colors {selected?.id === sm.id && !isNew
            ? 'bg-stone-100 text-stone-900 font-medium'
            : 'text-stone-600 hover:bg-stone-50'}"
        >
          {sm.name}
        </button>
      {/each}
      {#if sourceMasters.length === 0 && !isNew}
        <p class="text-xs text-stone-400 text-center py-4">出典がありません</p>
      {/if}
    </div>

    <!-- Form -->
    <div class="px-6 pb-6 border-t border-stone-100 pt-4 space-y-3">
      <div class="flex gap-2">
        <input
          type="text"
          bind:value={name}
          placeholder={isNew ? '新しい出典名' : '出典名を編集'}
          class="flex-1 px-3 py-2 text-sm border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-stone-400 focus:border-transparent"
        />
        <button
          onclick={handleSave}
          disabled={isSaving || !name.trim()}
          class="px-4 py-2 text-sm bg-stone-800 text-white rounded-md hover:bg-stone-700 transition-colors disabled:opacity-40"
        >
          {isSaving ? '保存中...' : (isNew ? '追加' : '保存')}
        </button>
      </div>

      <div class="flex items-center justify-between">
        <button
          onclick={startNew}
          class="flex items-center gap-1 text-xs text-stone-400 hover:text-stone-600 transition-colors"
        >
          <span class="material-symbols-rounded text-[14px]">add</span>
          新規追加
        </button>
        {#if selected && !isNew}
          <button
            onclick={handleDelete}
            disabled={isDeleting}
            class="flex items-center gap-1 text-xs text-rose-400 hover:text-rose-600 transition-colors disabled:opacity-50"
          >
            <span class="material-symbols-rounded text-[14px]">delete</span>
            削除
          </button>
        {/if}
      </div>

      {#if error}
        <p class="text-xs text-rose-500">{error}</p>
      {/if}
    </div>
  </div>
</div>
