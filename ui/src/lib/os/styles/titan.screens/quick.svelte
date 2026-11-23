<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { QuickScreenProps } from '../../screens';

  // Menu rapide style Titan : volet bleu nuit à droite (menu rapide de la touche Menu chez Philips), lignes sur barre bleue.
  let { items, index, time, hint, title, hover, activate, adjust, close }: QuickScreenProps = $props();
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 140 }}>
  <aside role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} in:fly={{ x: 60, duration: 220 }}>
    <header><h2>{title}</h2><span>{time}</span></header>
    {#each items as it, i}
      <div class="it" class:on={i === index} role="presentation" onmouseenter={() => hover(i)} onclick={() => activate(i)}>
        <span class="l">{it.label}</span>
        {#if it.kind === 'slider'}
          <span class="sl"><button class="st" onclick={(e) => (e.stopPropagation(), adjust(i, -1))}>-</button><span class="bar"><i style:width="{it.muted ? 0 : (it.level ?? 0)}%"></i></span><button class="st" onclick={(e) => (e.stopPropagation(), adjust(i, 1))}>+</button></span>
        {:else if it.value}<span class="v">{it.value}</span>{/if}
      </div>
    {/each}
    <p>{hint}</p>
  </aside>
</div>

<style>
  .scrim { position: absolute; inset: 0; z-index: 12; background: rgba(0, 0, 0, 0.45); font-family: var(--font); }
  aside { position: absolute; top: 0; right: 0; bottom: 0; width: 40rem; box-sizing: border-box; padding: 2.6rem 1.6rem; background: linear-gradient(180deg, rgba(22, 36, 66, 0.98), rgba(10, 16, 34, 0.98)); color: #fff; box-shadow: -0.8rem 0 3rem rgba(0, 0, 0, 0.5); outline: 0; }
  header { display: flex; justify-content: space-between; align-items: baseline; margin: 0 1rem 2.4rem; }
  h2 { margin: 0; font-size: 2rem; font-weight: 500; }
  header span { font-size: 1.4rem; opacity: 0.75; }
  .it { display: flex; justify-content: space-between; align-items: center; gap: 1rem; padding: 0.9rem 1rem; border-radius: 0.3rem; font-size: 1.45rem; font-weight: 500; cursor: pointer; }
  .it.on { background: #1d7be5; }
  .v { font-size: 1.15rem; opacity: 0.9; max-width: 16rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sl { display: flex; align-items: center; gap: 0.7rem; }
  .bar { width: 13rem; height: 0.6rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.25); overflow: hidden; }
  .bar i { display: block; height: 100%; background: #fff; }
  .st { width: 2.2rem; height: 2.2rem; border: 0; border-radius: 50%; background: rgba(255, 255, 255, 0.18); color: inherit; font: inherit; cursor: pointer; }
  p { margin: 2rem 1rem 0; font-size: 1rem; opacity: 0.55; }
</style>
