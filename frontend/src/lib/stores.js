import { writable } from 'svelte/store';

function persistedWritable(key, defaultValue) {
  const stored = localStorage.getItem(key);
  const initial = stored !== null ? JSON.parse(stored) : defaultValue;
  const store = writable(initial);
  store.subscribe(value => localStorage.setItem(key, JSON.stringify(value)));
  return store;
}

export const ttsRate = persistedWritable('ttsRate', 0.6);
