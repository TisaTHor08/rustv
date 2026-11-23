<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import type { QuickScreenProps } from '../../screens';

  // Réglages rapides style Tizen : carte arrondie bordée de clair en bas à gauche, boutons ronds avec leur libellé dessous
  // sur deux colonnes (le bouton ciblé devient blanc), pastilles bleues d'état au-dessus.
  let { items, index, time, phonesLabel, hover, activate, adjust, close }: QuickScreenProps = $props();
  const GLYPH: Record<string, string> = {
    volume: 'M3 9v6h4l5 5V4L7 9H3Zm13.5 3a4.5 4.5 0 0 0-2.5-4v8a4.5 4.5 0 0 0 2.5-4Z',
    output: 'M20 3H4a2 2 0 0 0-2 2v11a2 2 0 0 0 2 2h6v2H8v2h8v-2h-2v-2h6a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2Zm0 13H4V5h16Z',
    display: 'M12 7a5 5 0 1 0 0 10 5 5 0 0 0 0-10Zm0-5h-1v3h2V2Zm0 17h-1v3h2v-3ZM2 11v2h3v-2Zm17 0v2h3v-2ZM5.6 4.2 4.2 5.6l2.1 2.1 1.4-1.4Zm12.1 12.1-1.4 1.4 2.1 2.1 1.4-1.4ZM18.4 4.2l-2.1 2.1 1.4 1.4 2.1-2.1ZM7.7 16.3l-2.1 2.1 1.4 1.4 2.1-2.1Z',
    phones: 'M17 1H7a2 2 0 0 0-2 2v18a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V3a2 2 0 0 0-2-2Zm0 18H7V5h10Z',
    settings: 'M19.4 13a7.7 7.7 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3.3h-4l-.4 2.6a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.7 7.7 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1a7.4 7.4 0 0 0 1.7 1l.4 2.6h4l.4-2.6a7.4 7.4 0 0 0 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z',
    exit: 'M13 3h-2v10h2Zm4.8 2.2-1.4 1.4A6 6 0 1 1 7.6 6.6L6.2 5.2a8 8 0 1 0 11.6 0Z',
  };
  const glyph = (id: string) => GLYPH[id] ?? GLYPH.settings;
</script>

<div class="scrim" role="presentation" onclick={close} transition:fade={{ duration: 160 }}>
  <div class="wrap">
  <div class="pills">
    <span>{time}</span>
    {#if phonesLabel}<span>{phonesLabel}</span>{/if}
  </div>
  <aside role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} in:fly={{ y: 30, duration: 260, easing: cubicOut }}>
    <div class="grid">
      {#each items as it, i}
        {#if it.kind === 'slider'}
          <div class="sl" class:on={i === index} onmouseenter={() => hover(i)} role="presentation">
            <button class="rb" onclick={() => adjust(i, -1)} aria-label="-"><svg viewBox="0 0 24 24"><path d={glyph(it.id)} /></svg></button>
            <span class="bar"><i style:width="{it.muted ? 0 : (it.level ?? 0)}%"></i></span>
            <em>{it.muted ? '0' : (it.level ?? 0)}</em>
            <button class="st" onclick={() => adjust(i, 1)} aria-label="+">+</button>
          </div>
        {:else}
          <button class="cell" class:on={i === index} onmouseenter={() => hover(i)} onclick={() => activate(i)}>
            <span class="rb"><svg viewBox="0 0 24 24"><path d={glyph(it.id)} /></svg></span>
            <b>{it.label}</b>
            {#if it.value}<small>{it.value}</small>{/if}
          </button>
        {/if}
      {/each}
    </div>
  </aside>
  </div>
</div>

<style>
  .scrim { position: absolute; inset: 0; z-index: 12; background: linear-gradient(0deg, rgba(0, 0, 0, 0.7), rgba(0, 0, 0, 0.2) 60%, transparent); font-family: var(--font); }
  .wrap { position: absolute; left: 9rem; bottom: 2.6rem; width: 36rem; display: flex; flex-direction: column; gap: 1rem; }
  .pills { display: flex; gap: 0.8rem; }
  .pills span { padding: 0.6rem 1.8rem; border-radius: 99rem; background: #1630c8; color: #fff; font-size: 1.4rem; font-weight: 600; box-shadow: 0 0.4rem 1.2rem rgba(0, 0, 0, 0.4); }
  aside { width: 100%; padding: 1.6rem 1.4rem 1.2rem; box-sizing: border-box; border-radius: 2.4rem; background: linear-gradient(160deg, rgba(48, 50, 90, 0.9), rgba(20, 21, 40, 0.96)); color: #f4f5fa; box-shadow: inset 0 0 0 0.1rem rgba(255, 255, 255, 0.35), 0 2rem 5rem rgba(0, 0, 0, 0.55); outline: 0; backdrop-filter: blur(1.6rem); }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.8rem 0.4rem; }
  button { font-family: inherit; border: 0; background: transparent; color: inherit; cursor: pointer; }
  .cell { display: grid; justify-items: center; gap: 0.4rem; padding: 0.4rem 0.2rem; text-align: center; }
  .rb { width: 5.2rem; height: 5.2rem; border-radius: 50%; display: grid; place-items: center; background: rgba(255, 255, 255, 0.2); transition: background 0.15s, color 0.15s, transform 0.25s cubic-bezier(0.3, 1.3, 0.5, 1); }
  .rb svg { width: 2.4rem; height: 2.4rem; fill: currentColor; }
  .cell b { font-size: 1.25rem; font-weight: 500; }
  .cell small { font-size: 1rem; color: rgba(255, 255, 255, 0.62); max-width: 15rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .cell.on .rb { background: #fff; color: #10131c; transform: scale(1.08); box-shadow: 0 0 0 0.2rem rgba(255, 255, 255, 0.35); }
  .cell.on b { font-weight: 700; }
  .sl { grid-column: 1 / -1; display: flex; align-items: center; gap: 1rem; padding: 0.4rem 0.8rem; border-radius: 99rem; }
  .sl .rb { width: 4.4rem; height: 4.4rem; flex: none; }
  .sl.on .rb { background: #fff; color: #10131c; }
  .bar { flex: 1; height: 0.6rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.25); overflow: hidden; }
  .bar i { display: block; height: 100%; background: #3d9bff; transition: width 0.12s; }
  .sl em { min-width: 2.6rem; text-align: right; font-style: normal; font-size: 1.3rem; }
  .st { width: 2.8rem; height: 2.8rem; border-radius: 50%; background: rgba(255, 255, 255, 0.2); font-size: 1.5rem; }
</style>
