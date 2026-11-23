<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { SettingsScreenProps } from '../../screens';

  // Réglages style Google TV : page plein écran à deux colonnes (catégories à gauche, options à droite). Sans icônes.
  let { title, page, categories, rows, index, inCategories, hint, hoverCategory, openCategory, hoverRow, activateRow, back }: SettingsScreenProps = $props();
  const root = $derived(page === 'root');
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    void index;
    list?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="gs" transition:fade={{ duration: 180 }}>
  <aside>
    <h1>{t('settings.title')}</h1>
    <nav>
      {#each categories as c, i}
        <button class="cat" class:on={inCategories && i === index} class:cur={c.active} onmouseenter={() => hoverCategory(i)} onclick={() => openCategory(i)}>{c.label}</button>
      {/each}
    </nav>
  </aside>
  <section>
    <button class="back" onclick={back} aria-label={t('common.back')}><svg viewBox="0 0 24 24"><path d="M20 11H7.8l5.6-5.6L12 4l-8 8 8 8 1.4-1.4L7.8 13H20z" /></svg></button>
    <h2>{root ? (categories[index]?.label ?? '') : title}</h2>
    <div class="rows" bind:this={list}>
      {#each rows as r, i}
        <button class="row" class:on={!inCategories && i === index} class:static={!r.actionable} onmouseenter={() => !root && hoverRow(i)} onclick={() => !root && activateRow(i)}>
          <span class="lb">{r.label}</span>
          <span class="val">{#if r.swatch}<i style:background={r.swatch}></i>{/if}{#if r.arrows && !root}<b>‹</b>{/if}{r.value ?? ''}{#if r.arrows && !root}<b>›</b>{/if}</span>
        </button>
      {:else}
        <p class="empty">{root ? t('gtv.settings.open') : ''}</p>
      {/each}
    </div>
    <p class="hint">{hint}</p>
  </section>
</div>

<style>
  .gs { position: absolute; inset: 0; z-index: 12; display: grid; grid-template-columns: 38% 1fr; background: radial-gradient(60rem 40rem at 100% 0%, rgba(80, 90, 170, 0.22), transparent 70%), var(--bg); color: var(--text); font-family: var(--font); }
  aside { padding: 4.2rem 2rem 2rem 6rem; overflow: hidden; }
  h1 { margin: 0 0 2.2rem; font-size: 3.4rem; font-weight: 400; letter-spacing: -0.01em; }
  nav { display: flex; flex-direction: column; gap: 0.3rem; max-height: calc(100% - 7rem); overflow: hidden auto; scrollbar-width: none; }
  button { font-family: inherit; border: 0; cursor: pointer; background: transparent; color: inherit; text-align: left; }
  .cat { padding: 0.95rem 1.6rem; border-radius: 99rem; font-size: 1.6rem; color: var(--muted); transition: background 0.18s, color 0.18s; }
  .cat.cur { color: var(--text); background: var(--glass); }
  .cat.on { background: #e8eaed; color: #1f1f1f; font-weight: 500; }
  section { position: relative; padding: 4.2rem 6rem 2rem 2rem; overflow: hidden; }
  .back { position: absolute; left: -1rem; top: 4.8rem; width: 3rem; height: 3rem; border-radius: 50%; display: none; }
  .back svg { width: 1.8rem; height: 1.8rem; fill: currentColor; }
  h2 { margin: 0 0 1.6rem; padding: 0 1.6rem; font-size: 2rem; font-weight: 500; }
  .rows { max-height: calc(100% - 9rem); overflow: hidden auto; scrollbar-width: none; display: flex; flex-direction: column; gap: 0.45rem; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 2rem; padding: 1.05rem 1.6rem; border-radius: 1.2rem; background: var(--glass); font-size: 1.45rem; transition: background 0.18s, color 0.18s; }
  .row.on { background: #e8eaed; color: #1f1f1f; }
  .row.static { background: transparent; color: var(--muted); }
  .val { display: flex; align-items: center; gap: 0.7rem; font-size: 1.25rem; color: var(--muted); white-space: nowrap; max-width: 26rem; overflow: hidden; text-overflow: ellipsis; }
  .row.on .val { color: #444; }
  .val b { font-size: 1.6rem; font-weight: 400; opacity: 0.7; }
  .val i { width: 1.4rem; height: 1.4rem; border-radius: 50%; box-shadow: 0 0 0 0.12rem rgba(255, 255, 255, 0.7); }
  .empty { padding: 0 1.6rem; font-size: 1.3rem; color: var(--muted); }
  .hint { position: absolute; left: 2rem; bottom: 1.6rem; margin: 0; padding: 0 1.6rem; font-size: 1rem; color: var(--muted); }
</style>
