<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { SettingsScreenProps } from '../../screens';

  // Réglages style tvOS : à la racine, grande pastille « Réglages » à gauche et liste de catégories à droite (ligne ciblée = pastille blanche,
  // texte sombre, léger zoom) ; dans une page, titre centré et liste centrée de lignes translucides.
  let { title, page, categories, rows, index, inCategories, hint, hoverCategory, openCategory, hoverRow, activateRow }: SettingsScreenProps = $props();
  const root = $derived(page === 'root');
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    void index;
    list?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="ts" transition:fade={{ duration: 220 }}>
  {#if root}
    <div class="left">
      <span class="gear"><svg viewBox="0 0 24 24"><path d="M19.4 13a7.5 7.5 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3.2h-4L10.7 5.9a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.5 7.5 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1a7.4 7.4 0 0 0 1.7 1l.3 2.7h4l.3-2.7a7.4 7.4 0 0 0 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z" transform="translate(-1 0)" /></svg></span>
      <h1>{t('settings.title')}</h1>
    </div>
    <div class="list">
      {#each categories as c, i}
        <button class="row" class:on={inCategories && i === index} onmouseenter={() => hoverCategory(i)} onclick={() => openCategory(i)}><span>{c.label}</span><s>›</s></button>
      {/each}
    </div>
  {:else}
    <h1 class="mid">{title}</h1>
    <div class="list wide" bind:this={list}>
      {#each rows as r, i}
        <button class="row" class:on={i === index} class:static={!r.actionable} onmouseenter={() => hoverRow(i)} onclick={() => activateRow(i)}>
          <span>{r.label}</span>
          <span class="val">{#if r.swatch}<i style:background={r.swatch}></i>{/if}{#if r.arrows}<b>‹</b>{/if}{r.value ?? ''}{#if r.arrows}<b>›</b>{/if}</span>
        </button>
      {/each}
    </div>
  {/if}
  <footer>{hint}</footer>
</div>

<style>
  .ts { position: absolute; inset: 0; z-index: 12; color: #fff; font-family: var(--font); background: radial-gradient(90rem 50rem at 25% 20%, #2b2d3f, transparent 70%), radial-gradient(70rem 50rem at 90% 100%, #1c2a3a, transparent 70%), #08080a; }
  button { font-family: inherit; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  .left { position: absolute; left: 9rem; top: 0; bottom: 0; width: 34rem; display: grid; place-content: center; justify-items: center; gap: 2.4rem; }
  .gear { display: grid; place-items: center; width: 14rem; height: 14rem; border-radius: 3.2rem; background: linear-gradient(160deg, #8e8e93, #48484a); box-shadow: 0 1.6rem 3.6rem rgba(0, 0, 0, 0.5); }
  .gear svg { width: 8rem; height: 8rem; fill: #fff; }
  h1 { margin: 0; font-size: 3.4rem; font-weight: 600; letter-spacing: -0.01em; }
  h1.mid { position: absolute; left: 0; right: 0; top: 4.6rem; text-align: center; }
  .list { position: absolute; left: 52rem; right: 9rem; top: 5rem; bottom: 4.5rem; display: flex; flex-direction: column; gap: 0.5rem; overflow: hidden auto; scrollbar-width: none; padding: 1rem; margin: -1rem; }
  .list.wide { left: 20rem; right: 20rem; top: 12rem; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 2rem; flex: none; min-height: 5.4rem; padding: 0 2.4rem; border-radius: var(--r); background: rgba(118, 118, 128, 0.24); font-size: 1.7rem; font-weight: 500; transition: transform 0.2s var(--ease), background 0.2s, box-shadow 0.2s; }
  .row s { text-decoration: none; opacity: 0.5; font-size: 2.2rem; }
  .row.static { color: var(--muted); }
  .row.on { background: rgba(255, 255, 255, 0.95); color: #111; transform: scale(1.03); box-shadow: 0 1.2rem 2.6rem rgba(0, 0, 0, 0.45); position: relative; z-index: 2; }
  .val { display: flex; align-items: center; gap: 0.7rem; font-size: 1.5rem; color: var(--muted); max-width: 34rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .row.on .val { color: #555; }
  .val b { font-weight: 400; font-size: 2rem; }
  .val i { width: 1.4rem; height: 1.4rem; border-radius: 50%; box-shadow: 0 0 0 0.12rem rgba(128, 128, 128, 0.7); }
  footer { position: absolute; left: 0; right: 0; bottom: 1.6rem; text-align: center; font-size: 1.1rem; color: var(--muted); }
</style>
