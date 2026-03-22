<script>
  import { onMount } from 'svelte';
  import { API_BASE } from './lib/config.js';
  import Sidebar from './lib/Sidebar.svelte';
  import ContentView from './lib/ContentView.svelte';
  import RegisterModal from './lib/RegisterModal.svelte';
  import SourceMasterModal from './lib/SourceMasterModal.svelte';
  import DataManager from './lib/DataManager.svelte';

  let contents = $state([]);
  let sourceMasters = $state([]);
  let selectedContent = $state(null);
  let showModal = $state(false);
  let editingContent = $state(null);
  let showSourceModal = $state(false);
  let showDataManager = $state(false);
  // { contentId: string, type: 'translate'|'summarize', controller: AbortController } | null
  let globalProcessing = $state(null);

  onMount(async () => {
    await Promise.all([loadContents(), loadSourceMasters()]);
  });

  async function loadContents() {
    try {
      const res = await fetch(`${API_BASE}/api/contents`);
      contents = await res.json();
    } catch (e) {
      console.error('Failed to load contents', e);
    }
  }

  async function loadSourceMasters() {
    try {
      const res = await fetch(`${API_BASE}/api/sources`);
      sourceMasters = await res.json();
    } catch (e) {
      console.error('Failed to load sources', e);
    }
  }

  async function selectContent(id) {
    try {
      const res = await fetch(`${API_BASE}/api/contents/${id}`);
      selectedContent = await res.json();
    } catch (e) {
      console.error('Failed to load content', e);
    }
  }

  async function deleteContent() {
    if (!selectedContent) return;
    if (!confirm(`「${selectedContent.title}」を削除しますか？`)) return;
    try {
      await fetch(`${API_BASE}/api/contents/${selectedContent.id}`, {
        method: 'DELETE',
      });
      selectedContent = null;
      await loadContents();
    } catch (e) {
      console.error('Failed to delete content', e);
    }
  }

  function handleSave(saved) {
    showModal = false;
    editingContent = null;
    loadContents();
    selectedContent = saved;
  }

  function openEdit() {
    editingContent = selectedContent;
    showModal = true;
  }

  async function handleSourcesChanged() {
    await Promise.all([loadSourceMasters(), loadContents()]);
    // 表示中のコンテンツも更新
    if (selectedContent) {
      await selectContent(selectedContent.id);
    }
  }
</script>

<div class="flex h-screen overflow-hidden">
  <Sidebar
    {contents}
    selectedId={selectedContent?.id}
    onSelect={selectContent}
    onAdd={() => { editingContent = null; showModal = true; }}
    onManageSources={() => { showSourceModal = true; }}
    onManageData={() => { showDataManager = true; }}
  />

  <main class="flex-1 overflow-hidden bg-stone-50">
    {#if selectedContent}
      {#key selectedContent.id}
        <ContentView
          content={selectedContent}
          onDelete={deleteContent}
          onEdit={openEdit}
          onUpdate={(updated) => { selectedContent = updated; }}
          {globalProcessing}
          onGlobalProcessingChange={(v) => { globalProcessing = v; }}
        />
      {/key}
    {:else}
      <div class="flex flex-col items-center justify-center h-full text-stone-300 select-none">
        <span class="material-symbols-rounded text-[56px] mb-4">auto_stories</span>
        <p class="text-sm">左のサイドバーから教材を選んでください</p>
        <button
          onclick={() => { editingContent = null; showModal = true; }}
          class="mt-4 text-xs text-stone-400 hover:text-stone-600 underline underline-offset-2 transition-colors"
        >
          または教材を新規追加する
        </button>
      </div>
    {/if}
  </main>
</div>

{#if showModal}
  <RegisterModal
    {sourceMasters}
    content={editingContent}
    onClose={() => { showModal = false; editingContent = null; }}
    onSave={handleSave}
  />
{/if}

{#if showSourceModal}
  <SourceMasterModal
    {sourceMasters}
    onClose={() => { showSourceModal = false; }}
    onChanged={handleSourcesChanged}
  />
{/if}

{#if showDataManager}
  <DataManager
    onClose={async () => { showDataManager = false; await loadContents(); }}
  />
{/if}
