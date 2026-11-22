<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from './api';

  const params = new URLSearchParams(location.search);
  const index = Number(params.get('i') ?? 0);
  const count = Number(params.get('n') ?? 1);

  let name = $state('');
  let active = $state(document.hasFocus());

  function onKey(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft') api.pickerMove(-1);
    else if (e.key === 'ArrowRight') api.pickerMove(1);
    else if (e.key === 'Enter') api.pickerChoose(index);
    else if (e.key === 'Escape' || e.key === 'Backspace') api.tvExit();
    else return;
    e.preventDefault();
  }

  onMount(() => {
    api.state().then((s) => (name = s.monitors[index]?.name?.replace(/^\\\\\.\\/, '') ?? ''));
    const on = () => (active = true);
    const off = () => (active = false);
    window.addEventListener('focus', on);
    window.addEventListener('blur', off);
    return () => {
      window.removeEventListener('focus', on);
      window.removeEventListener('blur', off);
    };
  });
</script>

<svelte:window onkeydown={onKey} />

<button class="screen" class:active onclick={() => api.pickerChoose(index)}>
  <div class="ring"></div>
  <span class="num">{index + 1}</span>
  <span class="title">Écran {index + 1}{name ? ` · ${name}` : ''}</span>
  <span class="hint">
    {#if active}
      <kbd>OK</kbd> pour utiliser cet écran comme TV
    {:else}
      ◀ ▶ pour aller sur cet écran
    {/if}
  </span>
  {#if count > 1}<span class="dots">{#each Array(count) as _, i}<i class:on={i === index}></i>{/each}</span>{/if}
</button>

<style>
  .screen {
    position: fixed;
    inset: 0;
    width: 100%;
    border: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1.2rem;
    background: radial-gradient(60rem 40rem at 50% 40%, #1b2230, #05070a);
    color: var(--text);
    font-family: inherit;
    transition: filter 0.5s;
    filter: brightness(0.45) saturate(0.6);
  }
  .screen.active { filter: none; }
  .ring {
    position: absolute;
    width: 30rem;
    height: 30rem;
    border-radius: 50%;
    border: 0.25rem solid rgba(255, 106, 26, 0);
    transition: border-color 0.4s, transform 0.6s cubic-bezier(0.2, 0.8, 0.2, 1);
    transform: scale(0.85);
  }
  .active .ring { border-color: #ff6a1a; transform: scale(1); box-shadow: 0 0 5rem rgba(255, 106, 26, 0.35), inset 0 0 5rem rgba(255, 106, 26, 0.15); }
  .num { font-size: 16rem; font-weight: 800; line-height: 1; letter-spacing: -0.04em; }
  .title { font-size: 2.2rem; font-weight: 600; }
  .hint { font-size: 1.5rem; color: var(--muted); }
  kbd { font: inherit; padding: 0.15rem 0.8rem; border-radius: 0.5rem; background: #fff; color: #111; font-weight: 700; }
  .dots { position: absolute; bottom: 4rem; display: flex; gap: 0.8rem; }
  .dots i { width: 0.8rem; height: 0.8rem; border-radius: 50%; background: rgba(255, 255, 255, 0.25); }
  .dots i.on { background: #fff; }
</style>
