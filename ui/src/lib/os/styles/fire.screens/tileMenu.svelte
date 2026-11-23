<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { TileMenuScreenProps } from '../../screens';

  // Menu « Options » style Fire TV : fenêtre sombre en bas à droite, titre de l'appli, lignes à contour blanc quand ciblées.
  let { tile, items, index, hint, icon, hover, activate, close }: TileMenuScreenProps = $props();
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 140 }}>
  <div class="panel" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} in:fly={{ y: 40, duration: 200 }}>
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
  .scrim { position: absolute; inset: 0; z-index: 12; background: linear-gradient(0deg, rgba(0, 0, 0, 0.75), rgba(0, 0, 0, 0.35)); font-family: var(--font); }
  .panel { position: absolute; right: 4rem; bottom: 3rem; width: 38rem; max-height: calc(100% - 6rem); overflow: hidden auto; scrollbar-width: none; box-sizing: border-box; padding: 2rem 1.6rem 1.6rem; border-radius: var(--r); background: #242424; color: #fff; box-shadow: 0 1rem 3rem rgba(0, 0, 0, 0.6); outline: 0; }
  header { display: flex; align-items: center; gap: 1.2rem; margin: 0 0.4rem 1.4rem; }
  .ic { display: grid; place-items: center; width: 4.8rem; height: 2.7rem; border-radius: 0.2rem; background: linear-gradient(135deg, var(--a), var(--b)); font-size: 1.4rem; font-weight: 700; overflow: hidden; }
  .ic img { width: 1.8rem; height: 1.8rem; object-fit: contain; }
  h2 { margin: 0; font-size: 1.7rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .it { display: flex; justify-content: space-between; align-items: center; width: 100%; min-height: 4.2rem; padding: 0 1.2rem; margin-bottom: 0.2rem; border: 0; border-radius: var(--r); background: transparent; color: inherit; font: inherit; font-size: 1.45rem; text-align: left; cursor: pointer; }
  .it.on { background: #3a3a3a; box-shadow: 0 0 0 0.18rem #fff; }
  .v { display: flex; align-items: center; gap: 0.4rem; font-size: 1.15rem; color: rgba(255, 255, 255, 0.8); }
  .v s { text-decoration: none; font-size: 1.6rem; }
  .v i { width: 1.2rem; height: 1.2rem; border-radius: 50%; box-shadow: 0 0 0 0.1rem rgba(255, 255, 255, 0.6); }
  p { margin: 1.2rem 0.4rem 0; font-size: 1rem; color: rgba(255, 255, 255, 0.5); }
</style>
