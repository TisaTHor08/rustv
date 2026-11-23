<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import type { QuickScreenProps } from '../../screens';

  // Réglages rapides style Google TV : panneau en haut à droite, date et heure, bouton « Tous les réglages », tuiles sur deux colonnes.
  let { items, index, time, date, hint, hover, activate, adjust, close }: QuickScreenProps = $props();
  const settings = $derived(items.findIndex((x) => x.id === 'settings'));
  const tiles = $derived(items.map((it, i) => ({ it, i })).filter((x) => x.it.id !== 'settings'));
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 160 }}>
  <aside role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} in:fly={{ x: 40, y: -10, duration: 280, easing: cubicOut }}>
    <header>
      <div><small>{date}</small><b>{time}</b></div>
      {#if settings >= 0}
        <button class="gear" class:on={index === settings} onmouseenter={() => hover(settings)} onclick={() => activate(settings)}>
          <span><svg viewBox="0 0 24 24"><path d="M19.4 13a7.7 7.7 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1l-.4-2.7h-4l-.4 2.7a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L4.6 11a7.7 7.7 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1c.5.4 1.1.7 1.7 1l.4 2.7h4l.4-2.7c.6-.3 1.2-.6 1.7-1l2.5 1 2-3.5ZM12 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z" /></svg></span>
          <em>{items[settings].label}</em>
        </button>
      {/if}
    </header>
    <div class="grid">
      {#each tiles as { it, i }}
        <div class="tile" class:wide={it.kind === 'slider'} class:on={i === index} onmouseenter={() => hover(i)} onclick={() => activate(i)} role="presentation">
          <b>{it.label}</b>
          {#if it.kind === 'slider'}
            <span class="sl"><button class="st" onclick={(e) => (e.stopPropagation(), adjust(i, -1))}>-</button><span class="bar"><i style:width="{it.muted ? 0 : (it.level ?? 0)}%"></i></span><button class="st" onclick={(e) => (e.stopPropagation(), adjust(i, 1))}>+</button><em>{it.value}</em></span>
          {:else if it.value}<small>{it.value}</small>{/if}
        </div>
      {/each}
    </div>
    <p>{hint}</p>
  </aside>
</div>

<style>
  .scrim { position: absolute; inset: 0; z-index: 12; background: rgba(0, 0, 0, 0.62); font-family: var(--font); }
  aside { position: absolute; top: 2.4rem; right: 3.4rem; width: 46rem; box-sizing: border-box; padding: 2.2rem 2.2rem 1.6rem; border-radius: 2.6rem; background: #2b2f3a; color: #e8eaed; box-shadow: 0 2rem 5rem rgba(0, 0, 0, 0.55); outline: 0; }
  header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1.6rem; }
  header small { display: block; font-size: 1.25rem; color: rgba(232, 234, 237, 0.75); }
  header small::first-letter { text-transform: uppercase; }
  header b { font-size: 3.2rem; font-weight: 400; }
  .gear { display: grid; justify-items: center; gap: 0.5rem; border: 0; background: transparent; color: inherit; font: inherit; cursor: pointer; }
  .gear span { width: 4.6rem; height: 4.6rem; border-radius: 50%; display: grid; place-items: center; background: rgba(255, 255, 255, 0.12); }
  .gear svg { width: 2.4rem; height: 2.4rem; fill: currentColor; }
  .gear.on span { background: #e8eaed; color: #1f1f1f; box-shadow: 0 0 0 0.2rem #8ab4f8; }
  .gear em { font-size: 1.1rem; font-style: normal; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.8rem; }
  .tile { display: flex; flex-direction: column; justify-content: center; gap: 0.3rem; min-height: 6.6rem; padding: 0.9rem 1.5rem; border-radius: 1.6rem; background: rgba(255, 255, 255, 0.08); transition: background 0.15s, color 0.15s; cursor: pointer; }
  .tile.wide { grid-column: 1 / -1; }
  .tile b { font-size: 1.4rem; font-weight: 400; }
  .tile small { font-size: 1.05rem; color: rgba(232, 234, 237, 0.65); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tile.on { background: #e8eaed; color: #1f1f1f; box-shadow: 0 0 0 0.2rem #8ab4f8; }
  .tile.on small { color: #555; }
  .sl { display: flex; align-items: center; gap: 0.8rem; }
  .bar { flex: 1; height: 0.7rem; border-radius: 99rem; background: rgba(128, 128, 128, 0.4); overflow: hidden; }
  .bar i { display: block; height: 100%; background: #8ab4f8; transition: width 0.12s; }
  .tile.on .bar i { background: #1a56db; }
  .sl em { min-width: 3rem; text-align: right; font-style: normal; font-size: 1.2rem; }
  .st { width: 2.2rem; height: 2.2rem; border: 0; border-radius: 50%; background: rgba(128, 128, 128, 0.35); color: inherit; font: inherit; cursor: pointer; }
  p { margin: 1.2rem 0.4rem 0; font-size: 0.9rem; color: rgba(232, 234, 237, 0.55); }
</style>
