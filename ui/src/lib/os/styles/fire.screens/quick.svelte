<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { QuickScreenProps } from '../../screens';

  // Réglages rapides style Fire TV : bandeau en haut avec une rangée de cartes (icône ronde, nom, valeur) ; le volume a sa barre intégrée.
  let { items, index, time, title, hint, hover, activate, adjust, close }: QuickScreenProps = $props();
  const ICONS: Record<string, string> = {
    volume: 'M4 9.5v5h3.5l4.5 4v-13l-4.5 4Zm10 .5a3.5 3.5 0 0 1 0 4M16.5 7.5a7 7 0 0 1 0 9',
    output: 'M5 4h14v16H5zM12 9a2 2 0 1 0 0 4 2 2 0 0 0 0-4Zm0 6.5a3 3 0 0 0-3 3h6a3 3 0 0 0-3-3Z',
    display: 'M3 5h18v12H3zM8 21h8M12 17v4',
    phones: 'M8 3h8v18H8zM11 18h2',
    settings: 'M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7ZM12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M18.4 5.6l-2.1 2.1M7.7 16.3l-2.1 2.1',
    exit: 'M12 3v8M7 6.5a8 8 0 1 0 10 0',
  };
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 140 }}>
  <div class="band" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} in:fly={{ y: -50, duration: 220 }}>
    <header><h2>{title}</h2><span>{time}</span></header>
    <div class="cards">
      {#each items as it, i}
        <div class="c" class:on={i === index} role="presentation" onmouseenter={() => hover(i)} onclick={() => activate(i)}>
          <span class="ic"><svg viewBox="0 0 24 24"><path d={ICONS[it.id] ?? ICONS.settings} /></svg></span>
          <b>{it.label}</b>
          {#if it.kind === 'slider'}
            <span class="sl"><button class="st" onclick={(e) => (e.stopPropagation(), adjust(i, -1))}>-</button><span class="bar"><i style:width="{it.muted ? 0 : (it.level ?? 0)}%"></i></span><button class="st" onclick={(e) => (e.stopPropagation(), adjust(i, 1))}>+</button></span>
          {:else if it.value}<small>{it.value}</small>{/if}
        </div>
      {/each}
    </div>
    <p>{hint}</p>
  </div>
</div>

<style>
  .scrim { position: absolute; inset: 0; z-index: 12; background: linear-gradient(180deg, rgba(0, 0, 0, 0.35), rgba(0, 0, 0, 0.78)); font-family: var(--font); }
  .band { position: absolute; top: 0; left: 0; right: 0; box-sizing: border-box; padding: 2.6rem 5.2rem 2.2rem; background: #1c1c1c; color: #fff; outline: 0; box-shadow: 0 1rem 3rem rgba(0, 0, 0, 0.6); }
  header { display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 1.6rem; }
  h2 { margin: 0; font-size: 1.3rem; font-weight: 700; letter-spacing: 0.22em; text-transform: uppercase; }
  header span { font-size: 1.4rem; color: rgba(255, 255, 255, 0.75); }
  .cards { display: flex; gap: 0.8rem; padding: 0.5rem; margin: -0.5rem; overflow: hidden; }
  .c { flex: none; display: grid; justify-items: center; align-content: start; gap: 0.6rem; width: 13.6rem; height: 11rem; padding: 1.4rem 1rem 0; box-sizing: border-box; border-radius: var(--r); background: #2a2a2a; text-align: center; cursor: pointer; transition: transform 0.15s var(--ease); }
  .c.on { background: #363636; box-shadow: 0 0 0 0.2rem #fff; transform: scale(1.05); position: relative; z-index: 2; }
  .ic { display: grid; place-items: center; width: 3.6rem; height: 3.6rem; border-radius: 50%; background: #444; }
  .ic svg { width: 2rem; height: 2rem; fill: none; stroke: #fff; stroke-width: 1.7; stroke-linecap: round; stroke-linejoin: round; }
  b { font-size: 1.25rem; font-weight: 500; }
  small { font-size: 1rem; color: rgba(255, 255, 255, 0.7); max-width: 100%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sl { display: flex; align-items: center; gap: 0.4rem; width: 100%; }
  .bar { flex: 1; height: 0.5rem; border-radius: 0.3rem; background: rgba(255, 255, 255, 0.25); overflow: hidden; }
  .bar i { display: block; height: 100%; background: #fff; }
  .st { width: 1.8rem; height: 1.8rem; padding: 0; border: 0; border-radius: 50%; background: #555; color: #fff; font: inherit; font-size: 1.2rem; line-height: 1; cursor: pointer; }
  p { margin: 1.6rem 0 0; font-size: 1rem; color: rgba(255, 255, 255, 0.5); }
</style>
