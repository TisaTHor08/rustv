<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import type { QuickScreenProps } from '../screens';

  // Panneau de réglages rapides par défaut (RusTV) : feuille à droite avec la liste des réglages.
  let { items, index, title, time, date, hint, hover, activate, adjust, close }: QuickScreenProps = $props();
</script>

<div class="scrim" role="presentation" onclick={close}>
  <aside class="sheet" role="dialog" aria-label={title} tabindex="-1" onclick={(e) => e.stopPropagation()} in:fly={{ x: 60, duration: 320, easing: cubicOut }}>
    <header><h3>{title}</h3><span>{time}<small>{date}</small></span></header>
    {#each items as it, i}
      <div class="item" class:sel={i === index} class:info={it.kind === 'info'} onmouseenter={() => hover(i)} onclick={() => activate(i)} role="presentation">
        <span class="lb">{it.label}{#if it.sub}<small>{it.sub}</small>{/if}</span>
        {#if it.kind === 'slider'}
          <span class="sl"><button class="step" onclick={(e) => (e.stopPropagation(), adjust(i, -1))}>-</button><span class="bar"><i style:width="{it.muted ? 0 : (it.level ?? 0)}%"></i></span><button class="step" onclick={(e) => (e.stopPropagation(), adjust(i, 1))}>+</button><b>{it.value}</b></span>
        {:else if it.kind === 'choice'}
          <span class="val"><b>◀</b>{it.value}<b>▶</b></span>
        {:else if it.value}<span class="val">{it.value}</span>{/if}
      </div>
    {/each}
    <p class="hint">{hint}</p>
  </aside>
</div>

<style>
  .scrim { position: absolute; inset: 0; z-index: 12; display: flex; justify-content: flex-end; background: rgba(4, 6, 10, 0.55); backdrop-filter: blur(0.8rem); }
  .sheet { width: 40rem; height: 100%; box-sizing: border-box; padding: 2.4rem 2rem; overflow: hidden auto; scrollbar-width: none; background: var(--sheet); box-shadow: inset 0.08rem 0 0 var(--glass-strong); color: var(--text); outline: none; }
  header { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 1.2rem; }
  h3 { margin: 0; font-size: 1.9rem; }
  header span { font-size: 1.5rem; font-weight: 600; text-align: right; }
  header small { display: block; font-size: 0.9rem; font-weight: 400; color: var(--muted); }
  .item { display: flex; justify-content: space-between; align-items: center; gap: 1.2rem; margin: 0.25rem 0; padding: 0.95rem 1.1rem; border-radius: 0.9rem; font-size: 1.3rem; transition: background 0.2s; cursor: pointer; }
  .item.sel { background: var(--accent); color: #fff; font-weight: 600; }
  .item.info { cursor: default; }
  .lb small { display: block; font-size: 0.85rem; font-weight: 400; opacity: 0.7; }
  .val { display: flex; align-items: center; gap: 0.6rem; font-size: 1.15rem; max-width: 18rem; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .val b { font-size: 0.8rem; opacity: 0.6; }
  .sl { display: flex; align-items: center; gap: 0.6rem; }
  .bar { width: 11rem; height: 0.5rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.25); overflow: hidden; }
  .bar i { display: block; height: 100%; background: currentColor; transition: width 0.12s; }
  .sl b { min-width: 3rem; text-align: right; font-size: 1.1rem; }
  .step { width: 1.9rem; height: 1.9rem; border: 0; border-radius: 50%; background: rgba(255, 255, 255, 0.2); color: inherit; font: inherit; line-height: 1; cursor: pointer; }
  .hint { margin: 1.4rem 0 0; font-size: 0.9rem; color: var(--muted); }
</style>
