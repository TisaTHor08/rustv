<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fade, scale } from 'svelte/transition';
  import type { TileMenuScreenProps } from '../../screens';

  // Menu de tuile style webOS : panneau sombre à coins peu arrondis, lignes rectangulaires, ligne ciblée en blanc (menu d'options LG).
  let { tile, items, index, hint, icon, hover, activate, close }: TileMenuScreenProps = $props();
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 140 }}>
  <div class="panel" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} in:scale={{ start: 0.95, duration: 220, easing: cubicOut }}>
    <div class="head">
      <span class="mini" style:background="linear-gradient(135deg, {tile.color}, {tile.color2})">{#if icon}<img src={icon} alt="" />{:else}{tile.name[0]}{/if}</span>
      <b>{tile.name}</b>
    </div>
    {#each items as it, i}
      <button class="it" class:on={i === index} role="menuitem" onmouseenter={() => hover(i)} onclick={() => activate(i)}>
        <span>{it.label}</span>
        <span class="v">{#if it.swatch}<i style:background={it.swatch}></i>{/if}{#if it.arrows}<s>&lsaquo;</s>{/if}{it.value ?? ''}{#if it.arrows}<s>&rsaquo;</s>{/if}</span>
      </button>
    {/each}
    <p>{hint}</p>
  </div>
</div>

<style>
  .scrim { position: absolute; inset: 0; z-index: 12; display: grid; place-items: center; background: rgba(0, 0, 0, 0.65); font-family: var(--font); }
  .panel { width: 36rem; padding: 1.4rem; border-radius: 1rem; background: #1b1e27; color: #f4f7fb; box-shadow: 0 2rem 5rem rgba(0, 0, 0, 0.6); outline: 0; }
  .head { display: flex; align-items: center; gap: 1.1rem; padding: 0.2rem 0.6rem 1rem; }
  .head b { font-size: 1.6rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .mini { flex: none; width: 3.6rem; height: 3.6rem; border-radius: 0.8rem; display: grid; place-items: center; font-weight: 700; color: #fff; overflow: hidden; }
  .mini img { width: 70%; height: 70%; object-fit: contain; }
  .it { display: flex; justify-content: space-between; align-items: center; width: 100%; margin-top: 0.3rem; padding: 0.9rem 1.4rem; border: 0; border-radius: 0.7rem; background: #262a35; color: inherit; font: inherit; font-size: 1.4rem; text-align: left; cursor: pointer; transition: background 0.15s, box-shadow 0.15s; }
  .it.on { background: #fff; color: #11172b; }
  .v { display: flex; align-items: center; gap: 0.5rem; font-size: 1.15rem; opacity: 0.75; }
  .v s { text-decoration: none; font-size: 1.5rem; }
  .v i { width: 1.2rem; height: 1.2rem; border-radius: 50%; box-shadow: 0 0 0 0.1rem rgba(255, 255, 255, 0.6); }
  p { margin: 0.8rem 0.6rem 0; font-size: 0.95rem; color: rgba(255, 255, 255, 0.5); }
</style>
