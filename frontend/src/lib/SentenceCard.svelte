<script>
  import { API_BASE } from './config.js';
  let { sentence, showJapanese, reproductionMode = false, isActive, onClick, onUpdate, onDelete } = $props();

  const PASS_THRESHOLD = 70;

  // ふりがな: 世界《せかい》→ <ruby>世界<rt>せかい</rt></ruby>
  function toRubyHtml(text) {
    if (!text) return '';
    return text.replace(/([^《》\s]+)《([^》]+)》/g, '<ruby>$1<rt>$2</rt></ruby>');
  }

  // --- リプロダクション（テキスト） ---
  let reproInput = $state('');
  let reproRevealed = $state(false);
  let reproScore = $state(null);
  const textPassed = $derived(sentence.text_completed || (reproScore !== null && reproScore >= PASS_THRESHOLD));

  // --- スピーキング ---
  let speechResult = $state('');
  let matchScore = $state(null);
  let isListening = $state(false);
  const speechPassed = $derived(sentence.speech_completed || (matchScore !== null && matchScore >= PASS_THRESHOLD));

  const completed = $derived(textPassed && speechPassed);

  $effect(() => { onPracticeComplete?.(sentence.id, completed); });

  $effect(() => {
    if (!reproductionMode) {
      reproInput = '';
      reproRevealed = false;
      reproScore = null;
      speechResult = '';
      matchScore = null;
    }
  });

  function calcScore(original, input) {
    const origWords = original.split(/\s+/);
    const inputWords = input.split(/\s+/);
    let matches = 0;
    for (const word of inputWords) {
      if (origWords.includes(word)) matches++;
    }
    return Math.round((matches / origWords.length) * 100);
  }

  function normalize(text) {
    return text.toLowerCase().replace(/[.,!?'"]/g, '').trim();
  }

  async function saveCompleted(field) {
    try {
      const res = await fetch(`${API_BASE}/api/sentences/${sentence.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ [field]: true }),
      });
      if (res.ok) onUpdate(await res.json());
    } catch {}
  }

  function submitRepro() {
    if (!reproInput.trim()) return;
    const score = calcScore(normalize(sentence.english_text), normalize(reproInput));
    reproScore = score;
    reproRevealed = true;
    if (score >= PASS_THRESHOLD && !sentence.text_completed) saveCompleted('text_completed');
  }

  function resetRepro() {
    reproInput = '';
    reproRevealed = false;
    reproScore = null;
  }

  function startSpeech() {
    const SpeechRecognition = window.SpeechRecognition || window['webkitSpeechRecognition'];
    if (!SpeechRecognition) { alert('このブラウザは音声認識に対応していません'); return; }
    const recognition = new SpeechRecognition();
    recognition.lang = 'en-US';
    recognition.interimResults = false;
    recognition.maxAlternatives = 1;
    recognition.onstart = () => { isListening = true; speechResult = ''; matchScore = null; };
    recognition.onend = () => { isListening = false; };
    recognition.onerror = () => { isListening = false; };
    recognition.onresult = (event) => {
      const transcript = event.results[0][0].transcript;
      speechResult = transcript;
      const score = calcScore(normalize(sentence.english_text), normalize(transcript));
      matchScore = score;
      if (score >= PASS_THRESHOLD && !sentence.speech_completed) saveCompleted('speech_completed');
    };
    recognition.start();
  }

  // --- 読み上げ ---
  let isPlaying = $state(false);

  function getVoices() {
    return new Promise(resolve => {
      const voices = window.speechSynthesis.getVoices();
      if (voices.length) { resolve(voices); return; }
      window.speechSynthesis.onvoiceschanged = () => resolve(window.speechSynthesis.getVoices());
    });
  }

  async function playTTS() {
    if (isPlaying) { window.speechSynthesis.cancel(); isPlaying = false; return; }
    const voices = await getVoices();
    const femaleNames = ['Samantha', 'Karen', 'Moira', 'Tessa', 'Victoria', 'Google US English', 'Microsoft Zira'];
    const voice = femaleNames.reduce((found, name) =>
      found || voices.find(v => v.name.includes(name) && v.lang.startsWith('en')) || null, null
    ) || voices.find(v => v.lang.startsWith('en')) || null;
    const utterance = new SpeechSynthesisUtterance(sentence.english_text);
    utterance.lang = 'en-US';
    utterance.rate = 0.9;
    if (voice) utterance.voice = voice;
    utterance.onend = () => { isPlaying = false; };
    utterance.onerror = () => { isPlaying = false; };
    isPlaying = true;
    window.speechSynthesis.speak(utterance);
  }

  // --- 翻訳インライン編集 ---
  let isEditing = $state(false);
  let editText = $state('');

  function startEdit() { editText = sentence.japanese_text ?? ''; isEditing = true; }
  function cancelEdit() { isEditing = false; }

  async function saveEdit() {
    try {
      const res = await fetch(`${API_BASE}/api/sentences/${sentence.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ japanese_text: editText.trim() || null }),
      });
      if (!res.ok) throw new Error('保存に失敗しました');
      onUpdate(await res.json());
      isEditing = false;
    } catch (e) {
      alert(e?.message || '保存に失敗しました');
    }
  }

  // --- 英文インライン編集 ---
  let isEditingEn = $state(false);
  let editEnText = $state('');

  function startEditEn() { editEnText = sentence.english_text; isEditingEn = true; }
  function cancelEditEn() { isEditingEn = false; }

  async function saveEditEn() {
    if (!editEnText.trim()) return;
    try {
      const res = await fetch(`${API_BASE}/api/sentences/${sentence.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ english_text: editEnText.trim() }),
      });
      if (!res.ok) throw new Error('保存に失敗しました');
      onUpdate(await res.json());
      isEditingEn = false;
    } catch (e) {
      alert(e?.message || '保存に失敗しました');
    }
  }
</script>

<div
  class="group py-5 px-6 border-b border-stone-100 cursor-pointer transition-colors
    {completed && reproductionMode ? 'border-l-2 border-l-emerald-400 bg-emerald-50/30' :
     isActive ? 'bg-amber-50 border-l-2 border-l-amber-400' : 'hover:bg-stone-50'}"
  onclick={onClick}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === 'Enter' && onClick()}
>
  <div class="flex items-start gap-4">
    <span class="text-xs text-stone-300 font-mono pt-1 w-5 shrink-0 text-right select-none">
      {sentence.sentence_index + 1}
    </span>

    <div class="flex-1 min-w-0">
      {#if reproductionMode}
        <!-- ===== 練習モード ===== -->
        <div class="flex items-start justify-between gap-2 mb-3" onclick={(e) => e.stopPropagation()} role="presentation">
          <div class="flex-1">
            {#if sentence.japanese_text}
              <p class="text-stone-600 text-sm leading-relaxed">{@html toRubyHtml(sentence.japanese_text)}</p>
            {:else}
              <p class="text-stone-300 text-sm">（日本語訳なし）</p>
            {/if}
          </div>
          <div class="flex items-center gap-1.5 shrink-0">
            {#if completed}
              <span class="flex items-center gap-1 text-xs font-medium text-emerald-600 bg-emerald-100 px-2 py-0.5 rounded-full">
                <span class="material-symbols-rounded text-[13px]">check_circle</span>
                完了
              </span>
            {/if}
            <button
              onclick={(e) => { e.stopPropagation(); playTTS(); }}
              class="w-7 h-7 flex items-center justify-center rounded-md text-stone-400 hover:bg-stone-200 transition-colors {isPlaying ? 'bg-stone-200 text-stone-700' : ''}"
              title="模範を聴く"
            >
              <span class="material-symbols-rounded text-[16px]">{isPlaying ? 'stop' : 'volume_up'}</span>
            </button>
          </div>
        </div>

        <!-- ① テキストタスク -->
        <div
          class="rounded-lg border p-3 mb-2 {textPassed ? 'border-emerald-200 bg-emerald-50/50' : 'border-stone-200 bg-stone-50'}"
          onclick={(e) => e.stopPropagation()}
          role="presentation"
        >
          <div class="flex items-center gap-1.5 mb-2">
            <span class="material-symbols-rounded text-[14px] {textPassed ? 'text-emerald-500' : 'text-stone-400'}">edit_note</span>
            <span class="text-xs font-medium {textPassed ? 'text-emerald-600' : 'text-stone-500'}">テキスト</span>
            {#if textPassed}
              <span class="material-symbols-rounded text-[14px] text-emerald-500">check_circle</span>
            {:else if reproScore !== null}
              <span class="text-xs font-semibold {reproScore >= 50 ? 'text-amber-500' : 'text-rose-500'}">{reproScore}%</span>
            {/if}
          </div>

          {#if reproRevealed}
            <p class="text-stone-700 text-sm leading-relaxed mb-1.5">{sentence.english_text}</p>
            <div class="flex items-center gap-2 text-xs text-stone-400 flex-wrap">
              <span>あなた: <span class="text-stone-600">{reproInput}</span></span>
              {#if reproScore !== null}
                <span class="font-semibold {reproScore >= PASS_THRESHOLD ? 'text-emerald-500' : reproScore >= 50 ? 'text-amber-500' : 'text-rose-500'}">{reproScore}%</span>
              {/if}
              <button onclick={resetRepro} class="flex items-center gap-0.5 text-stone-400 hover:text-stone-600">
                <span class="material-symbols-rounded text-[13px]">refresh</span>やり直し
              </button>
            </div>
          {:else}
            <textarea
              bind:value={reproInput}
              onkeydown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); submitRepro(); } }}
              rows="2"
              placeholder="英文を入力... (Enter で答え合わせ)"
              class="w-full px-2 py-1.5 text-sm border border-stone-200 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-300 resize-none leading-relaxed text-stone-800 bg-white"
            ></textarea>
            <button
              onclick={submitRepro}
              disabled={!reproInput.trim()}
              class="mt-1.5 text-xs px-3 py-1 bg-amber-500 text-white rounded-md hover:bg-amber-600 transition-colors disabled:opacity-40"
            >
              答え合わせ
            </button>
          {/if}
        </div>

        <!-- ② スピーキングタスク -->
        <div
          class="rounded-lg border p-3 {speechPassed ? 'border-emerald-200 bg-emerald-50/50' : 'border-stone-200 bg-stone-50'}"
          onclick={(e) => e.stopPropagation()}
          role="presentation"
        >
          <div class="flex items-center gap-1.5 mb-2">
            <span class="material-symbols-rounded text-[14px] {speechPassed ? 'text-emerald-500' : 'text-stone-400'}">mic</span>
            <span class="text-xs font-medium {speechPassed ? 'text-emerald-600' : 'text-stone-500'}">スピーキング</span>
            {#if speechPassed}
              <span class="material-symbols-rounded text-[14px] text-emerald-500">check_circle</span>
            {:else if matchScore !== null}
              <span class="text-xs font-semibold {matchScore >= 50 ? 'text-amber-500' : 'text-rose-500'}">{matchScore}%</span>
            {/if}
          </div>

          <div class="flex items-center gap-2 flex-wrap">
            <button
              onclick={startSpeech}
              class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md border transition-colors
                {isListening
                  ? 'bg-rose-50 border-rose-200 text-rose-600 animate-pulse'
                  : 'bg-white border-stone-200 text-stone-600 hover:border-stone-400'}"
            >
              <span class="material-symbols-rounded text-[14px]">{isListening ? 'radio_button_checked' : 'mic'}</span>
              {isListening ? '録音中...' : '録音する'}
            </button>
            {#if speechResult}
              <div class="flex items-center gap-1.5 text-xs text-stone-500 flex-1 min-w-0">
                <span class="truncate text-stone-600">"{speechResult}"</span>
                <span class="font-semibold shrink-0 {matchScore >= PASS_THRESHOLD ? 'text-emerald-500' : matchScore >= 50 ? 'text-amber-500' : 'text-rose-500'}">{matchScore}%</span>
                <button onclick={() => { speechResult = ''; matchScore = null; }} class="shrink-0 text-stone-300 hover:text-stone-500">
                  <span class="material-symbols-rounded text-[13px]">close</span>
                </button>
              </div>
            {/if}
          </div>
        </div>

      {:else}
        <!-- ===== 通常モード ===== -->
        {#if isEditingEn}
          <div onclick={(e) => e.stopPropagation()} role="presentation">
            <textarea
              bind:value={editEnText}
              onkeydown={(e) => { if (e.key === 'Escape') cancelEditEn(); }}
              rows="2"
              class="w-full px-2 py-1.5 text-sm border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-stone-400 resize-none leading-relaxed text-stone-800"
              placeholder="英文を入力..."
            ></textarea>
            <div class="flex gap-1.5 mt-1">
              <button onclick={(e) => { e.stopPropagation(); saveEditEn(); }} class="text-xs px-2.5 py-1 bg-stone-800 text-white rounded-md hover:bg-stone-700 transition-colors">保存</button>
              <button onclick={(e) => { e.stopPropagation(); cancelEditEn(); }} class="text-xs px-2.5 py-1 border border-stone-300 text-stone-500 rounded-md hover:bg-stone-50 transition-colors">キャンセル</button>
            </div>
          </div>
        {:else}
          <div class="flex items-start gap-1 group/en">
            <p class="text-stone-800 leading-relaxed text-[15px] flex-1">{sentence.english_text}</p>
            <button
              onclick={(e) => { e.stopPropagation(); startEditEn(); }}
              class="shrink-0 opacity-0 group-hover/en:opacity-100 transition-opacity mt-0.5 text-stone-300 hover:text-stone-500"
              title="英文を編集"
            >
              <span class="material-symbols-rounded text-[14px]">edit</span>
            </button>
          </div>
        {/if}

        {#if showJapanese}
          {#if isEditing}
            <div class="mt-2" onclick={(e) => e.stopPropagation()} role="presentation">
              <textarea
                bind:value={editText}
                onkeydown={(e) => { if (e.key === 'Escape') cancelEdit(); }}
                rows="2"
                class="w-full px-2 py-1.5 text-sm border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-stone-400 resize-none leading-relaxed text-stone-700"
                placeholder="日本語訳を入力..."
              ></textarea>
              <div class="flex gap-1.5 mt-1">
                <button onclick={(e) => { e.stopPropagation(); saveEdit(); }} class="text-xs px-2.5 py-1 bg-stone-800 text-white rounded-md hover:bg-stone-700 transition-colors">保存</button>
                <button onclick={(e) => { e.stopPropagation(); cancelEdit(); }} class="text-xs px-2.5 py-1 border border-stone-300 text-stone-500 rounded-md hover:bg-stone-50 transition-colors">キャンセル</button>
              </div>
            </div>
          {:else if sentence.japanese_text}
            <div class="mt-1.5 flex items-start gap-1 group/jp">
              <p class="text-stone-400 text-sm leading-relaxed flex-1">{@html toRubyHtml(sentence.japanese_text)}</p>
              <button
                onclick={(e) => { e.stopPropagation(); startEdit(); }}
                class="shrink-0 opacity-0 group-hover/jp:opacity-100 transition-opacity mt-0.5 text-stone-300 hover:text-stone-500"
                title="翻訳を編集"
              >
                <span class="material-symbols-rounded text-[14px]">edit</span>
              </button>
            </div>
          {:else}
            <button
              onclick={(e) => { e.stopPropagation(); startEdit(); }}
              class="mt-1.5 text-xs text-stone-300 hover:text-stone-500 transition-colors flex items-center gap-1"
            >
              <span class="material-symbols-rounded text-[13px]">add</span>
              翻訳を追加
            </button>
          {/if}
        {/if}
      {/if}
    </div>

    <!-- 通常モードのツールボタン -->
    {#if !reproductionMode}
      <div class="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
        <button
          onclick={(e) => { e.stopPropagation(); playTTS(); }}
          class="w-8 h-8 flex items-center justify-center rounded-md text-stone-400 hover:bg-stone-200 hover:text-stone-700 transition-colors {isPlaying ? 'bg-stone-200 text-stone-700' : ''}"
          title="音声再生"
        >
          <span class="material-symbols-rounded text-[18px]">{isPlaying ? 'stop' : 'play_arrow'}</span>
        </button>
        <button
          onclick={(e) => { e.stopPropagation(); if (confirm('このセンテンスを削除しますか？')) onDelete?.(sentence.id); }}
          class="w-8 h-8 flex items-center justify-center rounded-md text-stone-300 hover:bg-rose-50 hover:text-rose-400 transition-colors"
          title="削除"
        >
          <span class="material-symbols-rounded text-[18px]">delete</span>
        </button>
      </div>
    {/if}
  </div>
</div>
