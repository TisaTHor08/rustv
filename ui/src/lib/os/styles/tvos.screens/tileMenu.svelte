<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import type { TileMenuScreenProps } from '../../screens';

  // Menu contextuel style tvOS : feuille centrée en verre dépoli aux grands angles, titre, lignes dont la ciblée devient une pastille blanche.
  let { tile, items, index, hint, icon, hover, activate, close }: TileMenuScreenProps = $props();
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 180 }}>
  <div class="sheet" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} in:scale={{ start: 0.92, duration: 220 }}>
    <header>
      <span class="ic" style:--a={tile.color} style:--b={tile.color2}>{#if icon}<img src={icon} alt="" />{:else}{tile.name[0]}{/if}</span>
      <h2>{tile.name}</h2>
    </header>
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
  .scrim { position: absolute; inset: 0; z-index: 12; display: grid; place-items: center; background: rgba(0, 0, 0, 0.5); backdrop-filter: blur(1.4rem); font-family: var(--font); }
  .sheet { width: 40rem; box-sizing: border-box; padding: 2.2rem 1.8rem 1.6rem; border-radius: 2.8rem; background: rgba(58, 58, 62, 0.82); backdrop-filter: blur(3rem) saturate(1.8); color: #fff; box-shadow: 0 2rem 5rem rgba(0, 0, 0, 0.55), inset 0 0 0 0.06rem rgba(255, 255, 255, 0.14); outline: 0; max-height: calc(100% - 6rem); overflow: hidden auto; scrollbar-width: none; }
  header { display: grid; justify-items: center; gap: 1rem; margin-bottom: 1.4rem; }
  .ic { display: grid; place-items: center; width: 9rem; height: 5.6rem; border-radius: 1.2rem; background: linear-gradient(145deg, var(--a), var(--b)); font-size: 2.4rem; font-weight: 700; overflow: hidden; box-shadow: 0 0.8rem 1.8rem rgba(0, 0, 0, 0.4); }
  .ic img { width: 3rem; height: 3rem; object-fit: contain; }
  h2 { margin: 0; font-size: 1.9rem; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 100%; }
  .it { display: flex; justify-content: space-between; align-items: center; width: 100%; min-height: 4.6rem; padding: 0 1.8rem; margin-bottom: 0.3rem; border: 0; border-radius: 1.6rem; background: transparent; color: inherit; font: inherit; font-size: 1.55rem; font-weight: 500; text-align: left; cursor: pointer; transition: background 0.15s, transform 0.15s var(--ease); }
  .it.on { background: rgba(255, 255, 255, 0.95); color: #111; transform: scale(1.03); }
  .v { display: flex; align-items: center; gap: 0.4rem; font-size: 1.25rem; opacity: 0.75; }
  .v s { text-decoration: none; font-size: 1.7rem; }
  .v i { width: 1.2rem; height: 1.2rem; border-radius: 50%; box-shadow: 0 0 0 0.1rem rgba(255, 255, 255, 0.6); }
  p { margin: 1rem 0 0; text-align: center; font-size: 1rem; color: var(--muted); }
</style>
