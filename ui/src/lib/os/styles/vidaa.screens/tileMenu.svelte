<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import type { TileMenuScreenProps } from '../../screens';

  // Menu de tuile style VIDAA : fenêtre centrale sombre, titre, lignes à cadre blanc quand ciblées.
  let { tile, items, index, hint, hover, activate, close }: TileMenuScreenProps = $props();
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 140 }}>
  <div class="panel" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} in:scale={{ start: 0.94, duration: 180 }}>
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
  .scrim { position: absolute; inset: 0; z-index: 12; display: grid; place-items: center; background: rgba(5, 6, 10, 0.7); font-family: var(--font); }
  .panel { width: 44rem; max-height: 80%; box-sizing: border-box; padding: 2.4rem 2.4rem 1.6rem; border-radius: 0.6rem; background: #1b1e2b; color: #fff; box-shadow: 0 1.2rem 4rem rgba(0, 0, 0, 0.6); outline: 0; overflow: hidden auto; scrollbar-width: none; }
  h2 { margin: 0 0 1.4rem; font-size: 1.9rem; font-weight: 400; text-align: center; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .it { display: flex; justify-content: space-between; align-items: center; width: 100%; padding: 0.95rem 1.2rem; border: 0.2rem solid transparent; border-radius: 0.3rem; background: transparent; color: inherit; font: inherit; font-size: 1.45rem; text-align: left; cursor: pointer; box-sizing: border-box; }
  .it.on { border-color: #fff; background: rgba(255, 255, 255, 0.1); }
  .v { display: flex; align-items: center; gap: 0.4rem; font-size: 1.2rem; color: #7fe3f0; }
  .v s { text-decoration: none; font-size: 1.5rem; }
  .v i { width: 1.2rem; height: 1.2rem; border-radius: 50%; box-shadow: 0 0 0 0.1rem rgba(255, 255, 255, 0.6); }
  p { margin: 1.4rem 0 0; font-size: 1rem; text-align: center; opacity: 0.55; }
</style>
