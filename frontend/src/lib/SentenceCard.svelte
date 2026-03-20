<script>
  let { sentence, showJapanese, isActive, onClick, onUpdate } = $props();

  let isPlaying = $state(false);
  let speechResult = $state('');
  let isListening = $state(false);
  let matchScore = $state(null);
  let isEditing = $state(false);
  let editText = $state('');

  function startEdit() {
    editText = sentence.japanese_text ?? '';
    isEditing = true;
  }

  function cancelEdit() {
    isEditing = false;
  }

  async function saveEdit() {
    try {
      const res = await fetch(`http://localhost:3001/api/sentences/${sentence.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ japanese_text: editText.trim() || null }),
      });
      if (!res.ok) throw new Error('保存に失敗しました');
      const updated = await res.json();
      onUpdate(updated);
      isEditing = false;
    } catch (e) {
      alert(e?.message || '保存に失敗しました');
    }
  }

  function handleEditKeydown(e) {
    if (e.key === 'Escape') cancelEdit();
  }

  function getVoices() {
    return new Promise(resolve => {
      const voices = window.speechSynthesis.getVoices();
      if (voices.length) { resolve(voices); return; }
      window.speechSynthesis.onvoiceschanged = () => resolve(window.speechSynthesis.getVoices());
    });
  }

  async function playTTS() {
    if (isPlaying) {
      window.speechSynthesis.cancel();
      isPlaying = false;
      return;
    }
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

  function startSpeech() {
    const SpeechRecognition = window.SpeechRecognition || window['webkitSpeechRecognition'];
    if (!SpeechRecognition) {
      alert('このブラウザは音声認識に対応していません');
      return;
    }
    const recognition = new SpeechRecognition();
    recognition.lang = 'en-US';
    recognition.interimResults = false;
    recognition.maxAlternatives = 1;
    recognition.onstart = () => { isListening = true; speechResult = ''; matchScore = null; };
    recognition.onend = () => { isListening = false; };
    recognition.onerror = () => { isListening = false; };
    recognition.onresult = (event) => {
      const transcript = event.results[0][0].transcript.toLowerCase().trim();
      speechResult = transcript;
      const original = sentence.english_text.toLowerCase().replace(/[.,!?]/g, '').trim();
      matchScore = calcScore(original, transcript);
    };
    recognition.start();
  }

  function calcScore(original, spoken) {
    const origWords = original.split(/\s+/);
    const spokenWords = spoken.split(/\s+/);
    let matches = 0;
    for (const word of spokenWords) {
      if (origWords.includes(word)) matches++;
    }
    return Math.round((matches / origWords.length) * 100);
  }
</script>

<div
  class="group py-5 px-6 border-b border-stone-100 cursor-pointer transition-colors {isActive ? 'bg-amber-50 border-l-2 border-l-amber-400' : 'hover:bg-stone-50'}"
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
      <p class="text-stone-800 leading-relaxed text-[15px]">{sentence.english_text}</p>

      {#if showJapanese}
        {#if isEditing}
          <!-- インライン編集 -->
          <div class="mt-2" onclick={(e) => e.stopPropagation()} role="presentation">
            <textarea
              bind:value={editText}
              onkeydown={handleEditKeydown}
              rows="2"
              class="w-full px-2 py-1.5 text-sm border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-stone-400 resize-none leading-relaxed text-stone-700"
              placeholder="日本語訳を入力..."
            ></textarea>
            <div class="flex gap-1.5 mt-1">
              <button
                onclick={(e) => { e.stopPropagation(); saveEdit(); }}
                class="text-xs px-2.5 py-1 bg-stone-800 text-white rounded-md hover:bg-stone-700 transition-colors"
              >
                保存
              </button>
              <button
                onclick={(e) => { e.stopPropagation(); cancelEdit(); }}
                class="text-xs px-2.5 py-1 border border-stone-300 text-stone-500 rounded-md hover:bg-stone-50 transition-colors"
              >
                キャンセル
              </button>
            </div>
          </div>
        {:else if sentence.japanese_text}
          <div class="mt-1.5 flex items-start gap-1 group/jp">
            <p class="text-stone-400 text-sm leading-relaxed flex-1">{sentence.japanese_text}</p>
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

      {#if speechResult}
        <div class="mt-2 text-xs">
          <span class="text-stone-400">認識結果: </span>
          <span class="text-stone-600">{speechResult}</span>
          {#if matchScore !== null}
            <span class="ml-2 font-semibold {matchScore >= 80 ? 'text-emerald-500' : matchScore >= 50 ? 'text-amber-500' : 'text-rose-500'}">
              {matchScore}%
            </span>
          {/if}
        </div>
      {/if}
    </div>

    <div class="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
      <button
        onclick={(e) => { e.stopPropagation(); playTTS(); }}
        class="w-8 h-8 flex items-center justify-center rounded-md text-stone-400 hover:bg-stone-200 hover:text-stone-700 transition-colors {isPlaying ? 'bg-stone-200 text-stone-700' : ''}"
        title="音声再生"
      >
        <span class="material-symbols-rounded text-[18px]">{isPlaying ? 'stop' : 'play_arrow'}</span>
      </button>
      <button
        onclick={(e) => { e.stopPropagation(); startSpeech(); }}
        class="w-8 h-8 flex items-center justify-center rounded-md text-stone-400 hover:bg-stone-200 hover:text-stone-700 transition-colors {isListening ? 'bg-rose-100 text-rose-500' : ''}"
        title="音読チェック"
      >
        <span class="material-symbols-rounded text-[18px]">mic</span>
      </button>
    </div>
  </div>
</div>
