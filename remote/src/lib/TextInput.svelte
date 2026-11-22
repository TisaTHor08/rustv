<script lang="ts">
  import { onMount } from 'svelte';
  import { send } from './connection.svelte.ts';

  let { onclose }: { onclose: () => void } = $props();
  let el: HTMLInputElement;
  let prev = '';

  onMount(() => el.focus());

  /** Calcule le delta entre l'ancien et le nouveau texte : retours arrière + caractères ajoutés. */
  function onInput() {
    const next = el.value;
    let i = 0;
    while (i < prev.length && i < next.length && prev[i] === next[i]) i++;
    const backspaces = [...prev.slice(i)].length;
    const text = next.slice(i);
    if (backspaces || text) send({ t: 'text', backspaces, text });
    prev = next;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      send({ t: 'key', k: 'enter' });
      el.value = '';
      prev = '';
    } else if (e.key === 'Backspace' && el.value === '') {
      send({ t: 'text', backspaces: 1, text: '' }); // champ vide côté téléphone mais texte existant côté PC
    }
  }
</script>

<div class="bar">
  <input
    bind:this={el}
    oninput={onInput}
    onkeydown={onKey}
    autocomplete="off"
    autocapitalize="off"
    spellcheck="false"
    enterkeyhint="send"
    placeholder="Saisie envoyée au PC…"
  />
  <button onclick={onclose}>✕</button>
</div>

<style>
  .bar {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    gap: 8px;
    padding: 10px 12px;
    background: var(--panel);
    box-shadow: 0 -4px 16px #0008;
  }
  input {
    flex: 1;
    background: var(--panel-2);
    color: var(--text);
    border: 0;
    border-radius: 12px;
    padding: 12px;
    font: inherit;
  }
  button {
    padding: 0 16px;
  }
</style>
