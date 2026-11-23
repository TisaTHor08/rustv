<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { QuickScreenProps } from '../../screens';

  // Panneau rapide style VIDAA : colonne sombre à gauche (le « Home Panel » de VIDAA s'ouvre à gauche), lignes à cadre blanc.
  let { items, index, time, hint, title, hover, activate, adjust, close }: QuickScreenProps = $props();
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 140 }}>
  <aside role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} in:fly={{ x: -60, duration: 220 }}>
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
  aside { position: absolute; top: 0; left: 0; bottom: 0; width: 38rem; box-sizing: border-box; padding: 3rem 2rem; background: #161925; color: #fff; box-shadow: 0.8rem 0 3rem rgba(0, 0, 0, 0.5); outline: 0; }
  header { display: flex; justify-content: space-between; align-items: baseline; margin: 0 0.6rem 2rem; }
  h2 { margin: 0; font-size: 2rem; font-weight: 300; letter-spacing: 0.04em; }
  header span { font-size: 1.4rem; opacity: 0.75; }
  .it { display: flex; justify-content: space-between; align-items: center; gap: 1rem; padding: 0.95rem 1.2rem; border: 0.2rem solid transparent; border-radius: 0.3rem; font-size: 1.45rem; cursor: pointer; }
  .it.on { border-color: #fff; background: rgba(255, 255, 255, 0.1); }
  .v { font-size: 1.15rem; color: #7fe3f0; max-width: 16rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sl { display: flex; align-items: center; gap: 0.7rem; }
  .bar { width: 13rem; height: 0.5rem; border-radius: 0.3rem; background: rgba(255, 255, 255, 0.22); overflow: hidden; }
  .bar i { display: block; height: 100%; background: #2fb5c8; }
  .st { width: 2.2rem; height: 2.2rem; border: 0; border-radius: 50%; background: rgba(255, 255, 255, 0.15); color: inherit; font: inherit; cursor: pointer; }
  p { margin: 2rem 0.6rem 0; font-size: 1rem; opacity: 0.55; }
</style>
