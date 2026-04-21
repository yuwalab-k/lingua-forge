<script>
  import { API_BASE } from './config.js';
  let { sourceMasters, onClose, onChanged } = $props();

  let editingId = $state(null); // 編集中の出典ID
  let isNew = $state(false);    // 新規追加モード
  let name = $state('');
  let isSaving = $state(false);
  let isDeleting = $state(false);
  let error = $state('');

  function startEdit(sm) {
    editingId = sm.id;
    isNew = false;
    name = sm.name;
    error = '';
  }

  function startNew() {
    editingId = null;
    isNew = true;
    name = '';
    error = '';
  }

  function cancelEdit() {
    editingId = null;
    isNew = false;
    name = '';
    error = '';
  }

  async function handleSave() {
    if (!name.trim()) { error = '名前は必須です'; return; }
    error = '';
    isSaving = true;
    try {
      const url = isNew
        ? `${API_BASE}/api/sources`
        : `${API_BASE}/api/sources/${editingId}`;
      const res = await fetch(url, {
        method: isNew ? 'POST' : 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: name.trim() }),
      });
      if (!res.ok) throw new Error('保存に失敗しました');
      onChanged();
      cancelEdit();
    } catch (e) {
      error = e?.message || 'エラーが発生しました';
    } finally {
      isSaving = false;
    }
  }

  async function handleDelete(sm) {
    if (!confirm(`「${sm.name}」を削除しますか？\n紐づく教材の出典設定は解除されます。`)) return;
    isDeleting = true;
    try {
      await fetch(`${API_BASE}/api/sources/${sm.id}`, { method: 'DELETE' });
      onChanged();
      if (editingId === sm.id) cancelEdit();
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
      <div class="flex items-center gap-3">
        <h2 class="text-base font-semibold text-stone-800">出典管理</h2>
        <button
          onclick={startNew}
          disabled={isNew}
          class="flex items-center gap-1 text-xs text-stone-500 hover:text-stone-800 transition-colors disabled:opacity-40"
        >
          <span class="material-symbols-rounded text-[14px]">add</span>
          新規追加
        </button>
      </div>
      <button
        onclick={onClose}
        class="w-7 h-7 flex items-center justify-center rounded-md text-stone-400 hover:bg-stone-100 transition-colors"
      >
        <span class="material-symbols-rounded text-[18px]">close</span>
      </button>
    </div>

    <!-- List -->
    <div class="px-6 py-4 space-y-1 max-h-72 overflow-y-auto">
      {#if sourceMasters.length === 0 && !isNew}
        <p class="text-xs text-stone-400 text-center py-4">出典がありません</p>
      {/if}

      {#each sourceMasters as sm (sm.id)}
        <div class="group">
          {#if editingId === sm.id}
            <!-- インライン編集 -->
            <div class="flex gap-2 py-1">
              <input
                type="text"
                bind:value={name}
                onkeydown={(e) => { if (e.key === 'Enter') handleSave(); if (e.key === 'Escape') cancelEdit(); }}
                class="flex-1 px-3 py-1.5 text-sm border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-stone-400"
              />
              <button
                onclick={handleSave}
                disabled={isSaving || !name.trim()}
                class="px-3 py-1.5 text-xs bg-stone-800 text-white rounded-md hover:bg-stone-700 transition-colors disabled:opacity-40"
              >
                {isSaving ? '保存中...' : '保存'}
              </button>
              <button
                onclick={cancelEdit}
                class="px-3 py-1.5 text-xs border border-stone-200 text-stone-500 rounded-md hover:bg-stone-50 transition-colors"
              >
                キャンセル
              </button>
            </div>
          {:else}
            <!-- 通常表示 -->
            <div class="flex items-center px-3 py-2 rounded-md hover:bg-stone-50 transition-colors">
              <span class="flex-1 text-sm text-stone-700">{sm.name}</span>
              <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onclick={() => startEdit(sm)}
                  class="w-7 h-7 flex items-center justify-center rounded text-stone-400 hover:text-stone-700 hover:bg-stone-200 transition-colors"
                  title="編集"
                >
                  <span class="material-symbols-rounded text-[15px]">edit</span>
                </button>
                <button
                  onclick={() => handleDelete(sm)}
                  disabled={isDeleting}
                  class="w-7 h-7 flex items-center justify-center rounded text-stone-400 hover:text-rose-500 hover:bg-rose-50 transition-colors disabled:opacity-40"
                  title="削除"
                >
                  <span class="material-symbols-rounded text-[15px]">delete</span>
                </button>
              </div>
            </div>
          {/if}
        </div>
      {/each}

      <!-- 新規追加フォーム -->
      {#if isNew}
        <div class="flex gap-2 pt-1">
          <input
            type="text"
            bind:value={name}
            placeholder="新しい出典名"
            onkeydown={(e) => { if (e.key === 'Enter') handleSave(); if (e.key === 'Escape') cancelEdit(); }}
            class="flex-1 px-3 py-1.5 text-sm border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-stone-400"
          />
          <button
            onclick={handleSave}
            disabled={isSaving || !name.trim()}
            class="px-3 py-1.5 text-xs bg-stone-800 text-white rounded-md hover:bg-stone-700 transition-colors disabled:opacity-40"
          >
            {isSaving ? '追加中...' : '追加'}
          </button>
          <button
            onclick={cancelEdit}
            class="px-3 py-1.5 text-xs border border-stone-200 text-stone-500 rounded-md hover:bg-stone-50 transition-colors"
          >
            キャンセル
          </button>
        </div>
      {/if}
    </div>

    {#if error}
      <div class="px-6 py-3 border-t border-stone-100">
        <p class="text-xs text-rose-500">{error}</p>
      </div>
    {/if}
  </div>
</div>
