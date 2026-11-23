<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { SettingsScreenProps } from '../../screens';

  // Réglages style Titan (d'après une photo des réglages Philips) : colonne noire à gauche, barre bleue sur la catégorie ciblée,
  // volet central bleu nuit translucide (titre, sous-titre, lignes avec chevron ou valeur), zone libre à droite (l'image du téléviseur
  // transparaît chez Philips ; ici un halo).
  let { title, page, categories, rows, index, inCategories, hint, hoverCategory, openCategory, hoverRow, activateRow }: SettingsScreenProps = $props();
  const root = $derived(page === 'root');
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    void index;
    list?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="ts" transition:fade={{ duration: 160 }}>
  <div class="halo"></div>
  <nav>
    <header><svg viewBox="0 0 24 24"><path d="M19.4 13a7.6 7.6 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3h-4l-.4 2.9a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.6 7.6 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1c.5.4 1.1.7 1.7 1L11 21h4l.4-2.9c.6-.3 1.2-.6 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z" /></svg><span>{t('settings.title')}</span></header>
    <div class="cats">
      {#each categories as c, i}
        <button class="cat" class:on={inCategories && i === index} class:cur={c.active} onmouseenter={() => hoverCategory(i)} onclick={() => openCategory(i)}><i></i>{c.label}</button>
      {/each}
    </div>
  </nav>
  <section>
    <h2>{root ? (categories[index]?.label ?? '') : title}</h2>
    <div class="rows" bind:this={list}>
      {#each rows as r, i}
        <button class="row" class:on={!inCategories && i === index} class:static={!r.actionable} onmouseenter={() => !root && hoverRow(i)} onclick={() => !root && activateRow(i)}>
          <span class="lb">{r.label}</span>
          <span class="val">{#if r.swatch}<i style:background={r.swatch}></i>{/if}{#if r.value}{r.value}{:else if r.actionable}&rsaquo;{/if}</span>
        </button>
      {:else}
        <p class="empty">{t('titan.settings.open')}</p>
      {/each}
    </div>
  </section>
  <footer>{hint}</footer>
</div>

<style>
  .ts { position: absolute; inset: 0; z-index: 12; color: #fff; font-family: var(--font); background: #04040a; }
  .halo { position: absolute; inset: 0; background: radial-gradient(60rem 40rem at 90% 40%, rgba(60, 80, 160, 0.35), transparent 70%), radial-gradient(40rem 30rem at 60% 100%, rgba(120, 40, 120, 0.25), transparent 70%); }
  button { font-family: inherit; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  nav { position: absolute; left: 0; top: 0; bottom: 0; width: 36rem; background: #000; display: flex; flex-direction: column; }
  header { display: flex; align-items: center; gap: 1.4rem; padding: 2.6rem 2.6rem 2.2rem; font-size: 2.2rem; font-weight: 500; }
  header svg { width: 2.6rem; height: 2.6rem; fill: currentColor; }
  .cats { flex: 1; overflow: hidden auto; scrollbar-width: none; padding: 2.8rem 0 4rem; }
  .cat { display: flex; align-items: center; gap: 1.6rem; width: 100%; padding: 0.95rem 2.6rem; font-size: 1.55rem; color: rgba(255, 255, 255, 0.85); }
  .cat i { flex: none; width: 1.8rem; height: 1.4rem; border-radius: 0.3rem; background: rgba(255, 255, 255, 0.55); }
  .cat.cur { color: #fff; font-weight: 600; }
  .cat.on { background: #1d7be5; color: #fff; font-weight: 600; }
  .cat.on i { background: #fff; }
  section { position: absolute; left: 36rem; top: 0; bottom: 0; width: 46rem; box-sizing: border-box; padding: 2.4rem 2.4rem 0; background: linear-gradient(180deg, rgba(22, 36, 66, 0.96), rgba(16, 24, 46, 0.96)); overflow: hidden; }
  h2 { margin: 0 0 4.4rem; font-size: 2.2rem; font-weight: 500; }
  .rows { height: calc(100% - 8rem); overflow: hidden auto; scrollbar-width: none; }
  .row { display: flex; align-items: center; justify-content: space-between; width: 100%; gap: 1.4rem; padding: 0.9rem 0.6rem; font-size: 1.5rem; font-weight: 500; }
  .row.on { background: rgba(29, 123, 229, 0.9); border-radius: 0.3rem; padding-left: 1.2rem; padding-right: 1.2rem; }
  .row.static { color: rgba(255, 255, 255, 0.7); }
  .val { display: flex; align-items: center; gap: 0.6rem; font-size: 1.3rem; max-width: 20rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .val i { width: 1.3rem; height: 1.3rem; border-radius: 50%; box-shadow: 0 0 0 0.12rem rgba(255, 255, 255, 0.7); }
  .empty { margin: 1rem 0 0; font-size: 1.3rem; color: rgba(255, 255, 255, 0.65); }
  footer { position: absolute; left: 2.6rem; bottom: 1.4rem; font-size: 1rem; color: rgba(255, 255, 255, 0.6); }
</style>
