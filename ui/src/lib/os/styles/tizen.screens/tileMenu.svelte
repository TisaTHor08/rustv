<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fade, scale } from 'svelte/transition';
  import type { TileMenuScreenProps } from '../../screens';

  // Menu de tuile style Tizen : carte flottante sombre bordée de clair au centre, lignes blanches à la sélection (menu contextuel One UI).
  let { tile, items, index, hint, icon, hover, activate, close }: TileMenuScreenProps = $props();
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 140 }}>
  <div class="panel" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} in:scale={{ start: 0.94, duration: 240, easing: cubicOut }}>
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
  .scrim { position: absolute; inset: 0; z-index: 12; display: grid; place-items: center; background: rgba(4, 5, 14, 0.62); font-family: var(--font); }
  .panel { width: 36rem; padding: 1.6rem; border-radius: 2.4rem; background: linear-gradient(160deg, rgba(60, 62, 110, 0.7), rgba(22, 23, 44, 0.96)); color: #f4f5fa; box-shadow: inset 0 0 0 0.1rem rgba(255, 255, 255, 0.3), 0 2rem 5rem rgba(0, 0, 0, 0.55); outline: 0; backdrop-filter: blur(1.6rem); }
  .head { display: flex; align-items: center; gap: 1.1rem; padding: 0.4rem 0.8rem 1.1rem; }
  .head b { font-size: 1.6rem; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .mini { flex: none; width: 3.6rem; height: 3.6rem; border-radius: 1.1rem; display: grid; place-items: center; font-weight: 700; color: #fff; overflow: hidden; }
  .mini img { width: 70%; height: 70%; object-fit: contain; }
  .it { display: flex; justify-content: space-between; align-items: center; width: 100%; padding: 0.95rem 1.5rem; border: 0; border-radius: 1.4rem; background: transparent; color: inherit; font: inherit; font-size: 1.4rem; text-align: left; cursor: pointer; transition: background 0.15s; }
  .it.on { background: #fff; color: #10131c; }
  .v { display: flex; align-items: center; gap: 0.5rem; font-size: 1.15rem; opacity: 0.75; }
  .v s { text-decoration: none; font-size: 1.5rem; }
  .v i { width: 1.2rem; height: 1.2rem; border-radius: 50%; box-shadow: 0 0 0 0.1rem rgba(255, 255, 255, 0.6); }
  p { margin: 0.8rem 0.8rem 0; font-size: 0.95rem; color: rgba(255, 255, 255, 0.55); }
</style>
