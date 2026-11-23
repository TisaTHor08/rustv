<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { TileMenuScreenProps } from '../../screens';

  // Menu « Options » style Roku : panneau violet collé à droite sur toute la hauteur, lignes à fond blanc quand ciblées.
  let { tile, items, index, hint, hover, activate, close }: TileMenuScreenProps = $props();
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 140 }}>
  <div class="panel" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} in:fly={{ x: 60, duration: 220 }}>
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
  .scrim { position: absolute; inset: 0; z-index: 12; background: rgba(0, 0, 0, 0.5); font-family: var(--font); }
  .panel { position: absolute; top: 0; right: 0; bottom: 0; width: 36rem; box-sizing: border-box; padding: 3rem 2rem; background: linear-gradient(180deg, #3b1259, #1b0a2c); color: #fff; box-shadow: -0.8rem 0 3rem rgba(0, 0, 0, 0.5); outline: 0; }
  h2 { margin: 0 0 2rem 0.8rem; font-size: 2rem; font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .it { display: flex; justify-content: space-between; align-items: center; width: 100%; padding: 0.9rem 1.2rem; border: 0; border-radius: 0.4rem; background: transparent; color: inherit; font: inherit; font-size: 1.5rem; font-weight: 500; text-align: left; cursor: pointer; }
  .it.on { background: #fff; color: #2b0f45; font-weight: 700; }
  .v { display: flex; align-items: center; gap: 0.4rem; font-size: 1.15rem; opacity: 0.85; }
  .v s { text-decoration: none; font-size: 1.5rem; }
  .v i { width: 1.2rem; height: 1.2rem; border-radius: 50%; box-shadow: 0 0 0 0.1rem rgba(255, 255, 255, 0.6); }
  p { margin: 2rem 0.8rem 0; font-size: 1rem; opacity: 0.6; }
</style>
