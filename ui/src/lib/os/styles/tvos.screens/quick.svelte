<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { QuickScreenProps } from '../../screens';

  // Réglages rapides style Centre de contrôle tvOS : panneau en haut à droite sous l'heure, grandes tuiles en verre (la tuile « Quitter » est
  // la grande à gauche, comme « Éteindre »), tuiles plus petites à droite, curseur de volume en pleine largeur. Tuile ciblée = blanche.
  let { items, index, time, title, hint, hover, activate, adjust, close }: QuickScreenProps = $props();
  const ICONS: Record<string, string> = {
    volume: 'M4 9.5v5h3.5l4.5 4v-13l-4.5 4Zm10 .5a3.5 3.5 0 0 1 0 4M16.5 7.5a7 7 0 0 1 0 9',
    output: 'M12 3a9 9 0 0 0-9 9v6h4v-6H5a7 7 0 0 1 14 0h-2v6h4v-6a9 9 0 0 0-9-9Z',
    display: 'M3 5h18v12H3zM8 21h8M12 17v4',
    phones: 'M8 3h8v18H8zM11 18h2',
    settings: 'M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7ZM12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M18.4 5.6l-2.1 2.1M7.7 16.3l-2.1 2.1',
    exit: 'M12 3v8M7 6.5a8 8 0 1 0 10 0',
  };
  const big = (id: string) => id === 'exit';
  const wide = (it: { kind: string }) => it.kind === 'slider';
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 160 }}>
  <div class="cc" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} in:fly={{ y: -24, duration: 240 }}>
    <header><span>{time}</span></header>
    <div class="grid" aria-label={title}>
      {#each items as it, i}
        <div class="t" class:big={big(it.id)} class:wide={wide(it)} class:on={i === index} role="presentation" onmouseenter={() => hover(i)} onclick={() => activate(i)}>
          <span class="ic"><svg viewBox="0 0 24 24"><path d={ICONS[it.id] ?? ICONS.settings} /></svg></span>
          <span class="tx"><b>{it.label}</b>{#if it.kind !== 'slider' && it.value}<small>{it.value}</small>{/if}</span>
          {#if it.kind === 'slider'}
            <span class="sl"><button class="st" onclick={(e) => (e.stopPropagation(), adjust(i, -1))}>-</button><span class="bar"><i style:width="{it.muted ? 0 : (it.level ?? 0)}%"></i></span><button class="st" onclick={(e) => (e.stopPropagation(), adjust(i, 1))}>+</button></span>
          {/if}
        </div>
      {/each}
    </div>
    <p>{hint}</p>
  </div>
</div>

<style>
  .scrim { position: absolute; inset: 0; z-index: 12; background: linear-gradient(200deg, rgba(0, 0, 0, 0.5), rgba(0, 0, 0, 0.12) 60%); font-family: var(--font); }
  .cc { position: absolute; top: 8rem; right: 5rem; width: 38rem; color: #fff; outline: 0; }
  header { display: none; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; grid-auto-rows: 5.8rem; gap: 0.8rem; padding: 0.8rem; margin: -0.8rem; }
  .t { display: flex; align-items: center; gap: 1rem; padding: 0 1.4rem; border-radius: 2.2rem; background: rgba(120, 120, 128, 0.42); backdrop-filter: blur(2.4rem) saturate(1.8); box-shadow: inset 0 0 0 0.06rem rgba(255, 255, 255, 0.14); cursor: pointer; transition: transform 0.2s var(--ease), background 0.2s, color 0.2s; }
  .t.big { order: -1; grid-row: span 2; flex-direction: column; justify-content: center; text-align: center; padding: 1.4rem; }
  .t.big .ic { width: 6rem; height: 6rem; background: transparent; }
  .t.big .ic svg { width: 5rem; height: 5rem; stroke-width: 1.4; }
  .t.wide { grid-column: span 2; flex-wrap: wrap; align-content: center; gap: 0.2rem 1rem; }
  .t.on { background: rgba(255, 255, 255, 0.95); color: #111; transform: scale(1.04); position: relative; z-index: 2; box-shadow: 0 1.2rem 2.4rem rgba(0, 0, 0, 0.4); }
  .ic { display: grid; place-items: center; flex: none; width: 3.2rem; height: 3.2rem; }
  .ic svg { width: 2.4rem; height: 2.4rem; fill: none; stroke: currentColor; stroke-width: 1.7; stroke-linecap: round; stroke-linejoin: round; }
  .tx { display: grid; line-height: 1.2; min-width: 0; }
  b { font-size: 1.35rem; font-weight: 600; }
  small { font-size: 1.05rem; opacity: 0.7; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sl { display: flex; align-items: center; gap: 0.6rem; flex: 1; min-width: 12rem; }
  .bar { flex: 1; height: 0.7rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.28); overflow: hidden; }
  .t.on .bar { background: rgba(0, 0, 0, 0.18); }
  .bar i { display: block; height: 100%; background: currentColor; }
  .st { width: 2.2rem; height: 2.2rem; padding: 0; border: 0; border-radius: 50%; background: rgba(255, 255, 255, 0.22); color: inherit; font: inherit; line-height: 1; cursor: pointer; }
  p { margin: 1.6rem 0.4rem 0; font-size: 1rem; color: var(--muted); text-align: right; text-shadow: 0 0.1rem 0.4rem rgba(0, 0, 0, 0.6); }
</style>
