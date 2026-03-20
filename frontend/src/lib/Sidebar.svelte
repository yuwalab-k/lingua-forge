<script>
  let { contents, selectedId, onSelect, onAdd } = $props();

  // 出典別にグループ化
  const grouped = $derived((() => {
    const groups = {};
    for (const content of contents) {
      const key = content.source || 'その他';
      if (!groups[key]) groups[key] = [];
      groups[key].push(content);
    }
    return Object.entries(groups).sort(([a], [b]) => {
      if (a === 'その他') return 1;
      if (b === 'その他') return -1;
      return a.localeCompare(b, 'ja');
    });
  })());

  let collapsed = $state({});

  function toggleGroup(key) {
    collapsed[key] = !collapsed[key];
  }
</script>

<aside class="w-64 bg-white border-r border-stone-200 flex flex-col h-full">
  <!-- Header -->
  <div class="px-4 py-4 border-b border-stone-200 flex items-center justify-between shrink-0">
    <h1 class="text-sm font-semibold text-stone-800 tracking-wide">Lingua Forge</h1>
    <button
      onclick={onAdd}
      class="w-7 h-7 flex items-center justify-center rounded-md text-stone-500 hover:bg-stone-100 hover:text-stone-800 transition-colors"
      title="教材を追加"
    >
      <span class="material-symbols-rounded text-[18px]">add</span>
    </button>
  </div>

  <!-- Content list -->
  <div class="flex-1 overflow-y-auto py-2">
    {#if contents.length === 0}
      <p class="px-4 py-6 text-xs text-stone-400 text-center leading-relaxed">
        教材がありません<br />
        <span class="text-stone-300">+ ボタンから追加してください</span>
      </p>
    {:else}
      {#each grouped as [source, items] (source)}
        <!-- Group header -->
        <button
          onclick={() => toggleGroup(source)}
          class="w-full flex items-center gap-1 px-4 py-1.5 text-left hover:bg-stone-50 transition-colors"
        >
          <span class="material-symbols-rounded text-[14px] text-stone-400 transition-transform {collapsed[source] ? '' : 'rotate-90'}">
            chevron_right
          </span>
          <span class="text-xs font-medium text-stone-400 uppercase tracking-wider truncate">{source}</span>
          <span class="ml-auto text-xs text-stone-300">{items.length}</span>
        </button>

        <!-- Group items -->
        {#if !collapsed[source]}
          <nav class="mb-1">
            {#each items as content (content.id)}
              <button
                onclick={() => onSelect(content.id)}
                class="w-full text-left pl-8 pr-3 py-2 transition-colors {selectedId === content.id
                  ? 'bg-stone-100 text-stone-900 font-medium'
                  : 'text-stone-600 hover:bg-stone-50 hover:text-stone-800'}"
              >
                <span class="block truncate text-sm">{content.title}</span>
              </button>
            {/each}
          </nav>
        {/if}
      {/each}
    {/if}
  </div>
</aside>
