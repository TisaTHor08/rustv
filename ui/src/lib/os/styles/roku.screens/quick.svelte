<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { QuickScreenProps } from '../../screens';

  // Réglages rapides style Roku : panneau violet à droite, liste verticale, curseur de volume sur sa ligne.
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
  .scrim { position: absolute; inset: 0; z-index: 12; background: rgba(0, 0, 0, 0.5); font-family: var(--font); }
  aside { position: absolute; top: 0; right: 0; bottom: 0; width: 40rem; box-sizing: border-box; padding: 3rem 2rem; background: linear-gradient(180deg, #3b1259, #1b0a2c); color: #fff; box-shadow: -0.8rem 0 3rem rgba(0, 0, 0, 0.5); outline: 0; }
  header { display: flex; justify-content: space-between; align-items: baseline; margin: 0 0.8rem 2rem; }
  h2 { margin: 0; font-size: 2rem; font-weight: 700; }
  header span { font-size: 1.5rem; opacity: 0.8; }
  .it { display: flex; justify-content: space-between; align-items: center; gap: 1rem; padding: 0.9rem 1.2rem; border-radius: 0.4rem; font-size: 1.5rem; font-weight: 500; cursor: pointer; }
  .it.on { background: #fff; color: #2b0f45; font-weight: 700; }
  .v { font-size: 1.15rem; opacity: 0.85; max-width: 18rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sl { display: flex; align-items: center; gap: 0.7rem; }
  .bar { width: 14rem; height: 0.7rem; border-radius: 0.2rem; background: rgba(128, 128, 128, 0.45); overflow: hidden; }
  .bar i { display: block; height: 100%; background: currentColor; }
  .st { width: 2.2rem; height: 2.2rem; border: 0; border-radius: 0.3rem; background: rgba(128, 128, 128, 0.35); color: inherit; font: inherit; cursor: pointer; }
  p { margin: 2rem 0.8rem 0; font-size: 1rem; opacity: 0.6; }
</style>
