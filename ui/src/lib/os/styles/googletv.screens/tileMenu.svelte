<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import type { TileMenuScreenProps } from '../../screens';

  // Menu de tuile style Google TV : panneau contextuel compact, au centre-droit, lignes en pilules.
  let { tile, items, index, hint, icon, hover, activate, close }: TileMenuScreenProps = $props();
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 140 }}>
  <div class="panel" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} in:fly={{ y: 24, duration: 260, easing: cubicOut }}>
    <div class="head">
      <span class="mini" style:background="linear-gradient(135deg, {tile.color}, {tile.color2})">{#if icon}<img src={icon} alt="" />{:else}{tile.name[0]}{/if}</span>
      <b>{tile.name}</b>
    </div>
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
  .scrim { position: absolute; inset: 0; z-index: 12; display: grid; place-items: center end; padding-right: 7rem; background: rgba(0, 0, 0, 0.55); font-family: var(--font); }
  .panel { width: 30rem; padding: 1.4rem; border-radius: 2rem; background: #2d2f33; color: #e8eaed; box-shadow: 0 1.6rem 4rem rgba(0, 0, 0, 0.5); outline: 0; }
  .head { display: flex; align-items: center; gap: 1rem; padding: 0.4rem 0.6rem 1rem; }
  .head b { font-size: 1.5rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .mini { flex: none; width: 3.4rem; height: 3.4rem; border-radius: 0.8rem; display: grid; place-items: center; font-weight: 700; color: #fff; overflow: hidden; }
  .mini img { width: 70%; height: 70%; object-fit: contain; }
  .it { display: flex; justify-content: space-between; align-items: center; width: 100%; padding: 0.85rem 1.3rem; border: 0; border-radius: 99rem; background: transparent; color: inherit; font: inherit; font-size: 1.3rem; text-align: left; cursor: pointer; transition: background 0.15s; }
  .it.on { background: #e8eaed; color: #1f1f1f; }
  .v { display: flex; align-items: center; gap: 0.4rem; font-size: 1.1rem; opacity: 0.8; }
  .v s { text-decoration: none; font-size: 1.5rem; }
  .v i { width: 1.2rem; height: 1.2rem; border-radius: 50%; box-shadow: 0 0 0 0.1rem rgba(255, 255, 255, 0.6); }
  p { margin: 0.8rem 0.6rem 0; font-size: 0.9rem; color: rgba(232, 234, 237, 0.6); }
</style>
