<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { SettingsScreenProps } from '../../screens';

  // Réglages style webOS : plein écran quasi noir, liste de catégories à gauche (ligne ciblée en blanc), lignes à droite en
  // rectangles gris arrondis avec valeur à droite, contour blanc épais sur la ligne ciblée (comme « Tous les réglages » de LG).
  let { title, page, categories, rows, index, inCategories, hint, hoverCategory, openCategory, hoverRow, activateRow }: SettingsScreenProps = $props();
  const root = $derived(page === 'root');
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    void index;
    list?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="ws" transition:fade={{ duration: 180 }}>
  <header>
    <svg viewBox="0 0 24 24"><path d="M19.4 13a7.7 7.7 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3.3h-4l-.4 2.6a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.7 7.7 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1a7.4 7.4 0 0 0 1.7 1l.4 2.6h4l.4-2.6a7.4 7.4 0 0 0 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z" /></svg>
    <h1>{t('settings.title')}</h1>
  </header>
  <nav>
    {#each categories as c, i}
      <button class="cat" class:on={inCategories && i === index} class:cur={c.active} onmouseenter={() => hoverCategory(i)} onclick={() => openCategory(i)}>{c.label}</button>
    {/each}
  </nav>
  <section>
    <h2>{root ? (categories[index]?.label ?? '') : title}</h2>
    <div class="rows" bind:this={list}>
      {#each rows as r, i}
        <button class="row" class:on={!inCategories && i === index} class:static={!r.actionable} onmouseenter={() => !root && hoverRow(i)} onclick={() => !root && activateRow(i)}>
          <span class="lb">{r.label}</span>
          <span class="val">{#if r.swatch}<i style:background={r.swatch}></i>{/if}{#if r.arrows && !root}<b>&lt;</b>{/if}{r.value ?? ''}{#if r.arrows && !root}<b>&gt;</b>{/if}</span>
        </button>
      {:else}
        <p class="empty">{t('webos.settings.pick')}</p>
      {/each}
    </div>
  </section>
  <footer>{hint}</footer>
</div>

<style>
  .ws { position: absolute; inset: 0; z-index: 12; color: #f4f7fb; font-family: var(--font); background: radial-gradient(60rem 40rem at 90% 100%, #1b2347 0%, transparent 70%), #0a0b10; }
  header { position: absolute; left: 6rem; top: 3rem; display: flex; align-items: center; gap: 1.2rem; }
  header svg { width: 3rem; height: 3rem; fill: currentColor; }
  header h1 { margin: 0; font-size: 2.6rem; font-weight: 500; }
  nav { position: absolute; left: 5rem; top: 9rem; bottom: 4.6rem; width: 33rem; display: flex; flex-direction: column; gap: 0.3rem; overflow: hidden auto; scrollbar-width: none; }
  button { font-family: inherit; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  .cat { padding: 0.95rem 1.6rem; border-radius: 0.7rem; font-size: 1.6rem; color: rgba(255, 255, 255, 0.8); transition: background 0.15s; }
  .cat.cur { color: #fff; background: rgba(255, 255, 255, 0.1); }
  .cat.on { background: #fff; color: #11172b; font-weight: 600; }
  section { position: absolute; left: 41rem; right: 5rem; top: 9rem; bottom: 4.6rem; }
  h2 { margin: 0 0 1.2rem 0.4rem; font-size: 2rem; font-weight: 500; }
  .rows { height: calc(100% - 4rem); overflow: hidden auto; scrollbar-width: none; display: flex; flex-direction: column; gap: 0.6rem; padding: 0.3rem; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 2rem; padding: 1.05rem 1.8rem; border-radius: 0.8rem; font-size: 1.5rem; background: #1d2027; transition: background 0.15s, box-shadow 0.15s; }
  .row.on { background: #2a2e38; box-shadow: 0 0 0 0.28rem #fff; }
  .row.static { color: rgba(255, 255, 255, 0.75); background: #15171c; }
  .val { display: flex; align-items: center; gap: 0.7rem; font-size: 1.3rem; color: rgba(255, 255, 255, 0.75); white-space: nowrap; max-width: 26rem; overflow: hidden; text-overflow: ellipsis; }
  .val b { font-size: 1.1rem; opacity: 0.7; }
  .val i { width: 1.3rem; height: 1.3rem; border-radius: 50%; box-shadow: 0 0 0 0.12rem rgba(255, 255, 255, 0.7); }
  .empty { margin: 1rem 0.4rem; font-size: 1.4rem; color: rgba(255, 255, 255, 0.65); }
  footer { position: absolute; left: 6rem; bottom: 1.6rem; font-size: 1rem; color: rgba(255, 255, 255, 0.55); }
</style>
