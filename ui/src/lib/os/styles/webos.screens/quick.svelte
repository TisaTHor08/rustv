<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { QuickScreenProps } from '../../screens';

  // Réglages rapides style webOS : panneau sur toute la hauteur à gauche (la roue dentée de l'accueil l'ouvre), cartes sombres
  // arrondies sur deux colonnes avec icône, libellé et valeur, contour blanc sur la carte ciblée, curseur de volume en pleine largeur.
  let { items, index, time, date, hover, activate, adjust, close }: QuickScreenProps = $props();
  const GLYPH: Record<string, string> = {
    volume: 'M3 9v6h4l5 5V4L7 9H3Zm13.5 3a4.5 4.5 0 0 0-2.5-4v8a4.5 4.5 0 0 0 2.5-4Z',
    output: 'M20 3H4a2 2 0 0 0-2 2v11a2 2 0 0 0 2 2h6v2H8v2h8v-2h-2v-2h6a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2Zm0 13H4V5h16Z',
    display: 'M12 7a5 5 0 1 0 0 10 5 5 0 0 0 0-10Zm0-5h-1v3h2V2Zm0 17h-1v3h2v-3ZM2 11v2h3v-2Zm17 0v2h3v-2Z',
    phones: 'M17 1H7a2 2 0 0 0-2 2v18a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V3a2 2 0 0 0-2-2Zm0 18H7V5h10Z',
    settings: 'M19.4 13a7.7 7.7 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3.3h-4l-.4 2.6a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.7 7.7 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1a7.4 7.4 0 0 0 1.7 1l.4 2.6h4l.4-2.6a7.4 7.4 0 0 0 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z',
    exit: 'M13 3h-2v10h2Zm4.8 2.2-1.4 1.4A6 6 0 1 1 7.6 6.6L6.2 5.2a8 8 0 1 0 11.6 0Z',
  };
  const glyph = (id: string) => GLYPH[id] ?? GLYPH.settings;
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 160 }}>
  <aside role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} in:fly={{ x: -60, duration: 280, easing: cubicOut }}>
    <header><h1>{t('webos.quick.title')}</h1><span><b>{time}</b><small>{date}</small></span></header>
    <div class="grid">
      {#each items as it, i}
        {#if it.kind === 'slider'}
          <div class="card wide" class:on={i === index} onmouseenter={() => hover(i)} role="presentation">
            <svg viewBox="0 0 24 24"><path d={glyph(it.id)} /></svg>
            <button class="st" onclick={() => adjust(i, -1)} aria-label="-">-</button>
            <span class="bar"><i style:width="{it.muted ? 0 : (it.level ?? 0)}%"></i></span>
            <button class="st" onclick={() => adjust(i, 1)} aria-label="+">+</button>
            <em>{it.muted ? '0' : (it.level ?? 0)}</em>
          </div>
        {:else}
          <button class="card" class:on={i === index} onmouseenter={() => hover(i)} onclick={() => activate(i)}>
            <svg viewBox="0 0 24 24"><path d={glyph(it.id)} /></svg>
            <b>{it.label}</b>
            {#if it.value}<small>{it.value}</small>{/if}
          </button>
        {/if}
      {/each}
    </div>
  </aside>
</div>

<style>
  .scrim { position: absolute; inset: 0; z-index: 12; background: linear-gradient(90deg, rgba(0, 0, 0, 0.7), rgba(0, 0, 0, 0.25) 55%, transparent); font-family: var(--font); }
  aside { position: absolute; left: 0; top: 0; bottom: 0; width: 40rem; box-sizing: border-box; padding: 2.4rem 2.2rem; background: rgba(18, 20, 28, 0.94); color: #f4f7fb; box-shadow: 0 0 5rem rgba(0, 0, 0, 0.6); outline: 0; backdrop-filter: blur(1.4rem); overflow: hidden; }
  header { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 1.8rem; }
  h1 { margin: 0; font-size: 2.2rem; font-weight: 500; }
  header span { display: grid; text-align: right; }
  header b { font-size: 1.8rem; font-weight: 500; }
  header small { font-size: 1rem; color: rgba(255, 255, 255, 0.6); }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
  button { font-family: inherit; border: 0; color: inherit; cursor: pointer; }
  .card { display: flex; flex-direction: column; justify-content: center; gap: 0.5rem; min-height: 8.4rem; padding: 1.2rem 1.4rem; border-radius: 1rem; background: #262a35; text-align: left; transition: background 0.15s, box-shadow 0.15s; }
  .card svg { width: 2.6rem; height: 2.6rem; fill: currentColor; }
  .card b { font-size: 1.35rem; font-weight: 500; }
  .card small { font-size: 1.05rem; color: rgba(255, 255, 255, 0.6); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card.on { background: #323847; box-shadow: 0 0 0 0.26rem #fff; }
  .wide { grid-column: 1 / -1; flex-direction: row; align-items: center; min-height: 6.4rem; gap: 1rem; }
  .bar { flex: 1; height: 0.7rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.22); overflow: hidden; }
  .bar i { display: block; height: 100%; background: #3a8cf0; transition: width 0.12s; }
  .wide em { min-width: 2.6rem; text-align: right; font-style: normal; font-size: 1.4rem; }
  .st { width: 2.6rem; height: 2.6rem; border-radius: 50%; background: rgba(255, 255, 255, 0.16); font-size: 1.4rem; }
</style>
