<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import type { TileMenuScreenProps } from '../../screens';

  // Menu de tuile style Titan : fenêtre centrale bleu nuit translucide, ligne ciblée sur barre bleue.
  let { tile, items, index, hint, hover, activate, close }: TileMenuScreenProps = $props();
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 140 }}>
  <div class="panel" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} in:scale={{ start: 0.95, duration: 180 }}>
    <h2>{tile.name}</h2>
    {#each items as it, i}
      <button class="it" class:on={i === index} role="menuitem" onmouseenter={() => hover(i)} onclick={() => activate(i)}>
        <span>{it.label}</span>
        <span class="v">{#if it.swatch}<i style:background={it.swatch}></i>{/if}{#if it.arrows}<s>‹</s>{/if}{it.value ?? ''}{#if it.arrows}<s>›</s>{/if}</span>
      </button>
    {/each}
    <p>{hint}</p>
  </div>
</div>

<style>
  .scrim { position: absolute; inset: 0; z-index: 12; display: grid; place-items: center; background: rgba(2, 3, 10, 0.72); font-family: var(--font); }
  .panel { width: 42rem; max-height: 80%; box-sizing: border-box; padding: 2.2rem 1.6rem 1.4rem; border-radius: 0.8rem; background: linear-gradient(180deg, rgba(24, 38, 70, 0.98), rgba(14, 22, 44, 0.98)); color: #fff; box-shadow: 0 1.2rem 4rem rgba(0, 0, 0, 0.6); outline: 0; overflow: hidden auto; scrollbar-width: none; }
  h2 { margin: 0 0 1.4rem 1rem; font-size: 2rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .it { display: flex; justify-content: space-between; align-items: center; width: 100%; padding: 0.9rem 1rem; border: 0; border-radius: 0.3rem; background: transparent; color: inherit; font: inherit; font-size: 1.45rem; font-weight: 500; text-align: left; cursor: pointer; }
  .it.on { background: #1d7be5; }
  .v { display: flex; align-items: center; gap: 0.4rem; font-size: 1.2rem; opacity: 0.9; }
  .v s { text-decoration: none; font-size: 1.5rem; }
  .v i { width: 1.2rem; height: 1.2rem; border-radius: 50%; box-shadow: 0 0 0 0.1rem rgba(255, 255, 255, 0.6); }
  p { margin: 1.2rem 1rem 0; font-size: 1rem; opacity: 0.55; }
</style>
