<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { SettingsScreenProps } from '../../screens';

  // Réglages style VIDAA : fond bleu nuit uni, colonne de catégories à gauche (cadre blanc sur la ligne ciblée), liste de lignes
  // séparées par un filet à droite, valeur alignée à droite. Proportions estimées (aucune capture des réglages VIDAA trouvée).
  let { title, page, categories, rows, index, inCategories, hint, hoverCategory, openCategory, hoverRow, activateRow }: SettingsScreenProps = $props();
  const root = $derived(page === 'root');
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    void index;
    list?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="vs" transition:fade={{ duration: 160 }}>
  <h1>{t('settings.title')}</h1>
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
        <p class="empty">{t('vidaa.settings.open')}</p>
      {/each}
    </div>
  </section>
  <footer>{hint}</footer>
</div>

<style>
  .vs { position: absolute; inset: 0; z-index: 12; color: #fff; font-family: var(--font); background: #10121a; }
  h1 { position: absolute; left: 5.6rem; top: 3rem; margin: 0; font-size: 2.6rem; font-weight: 300; letter-spacing: 0.04em; }
  button { font-family: inherit; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  nav { position: absolute; left: 5rem; top: 9rem; bottom: 4.4rem; width: 32rem; display: flex; flex-direction: column; gap: 0.3rem; padding: 0.4rem; overflow: hidden auto; scrollbar-width: none; }
  .cat { padding: 0.95rem 1.4rem; border-radius: 0.3rem; font-size: 1.5rem; color: rgba(255, 255, 255, 0.7); border: 0.2rem solid transparent; }
  .cat.cur { color: #fff; background: rgba(255, 255, 255, 0.08); }
  .cat.on { color: #fff; background: rgba(255, 255, 255, 0.14); border-color: #fff; }
  section { position: absolute; left: 40rem; right: 4.5rem; top: 3rem; bottom: 4.4rem; padding: 0 0 0 3rem; border-left: 0.1rem solid rgba(255, 255, 255, 0.14); box-sizing: border-box; overflow: hidden; }
  h2 { margin: 0 0 1.6rem; padding-top: 0.2rem; font-size: 2.1rem; font-weight: 400; }
  .rows { height: calc(100% - 4.2rem); overflow: hidden auto; scrollbar-width: none; padding: 0.3rem; }
  .row { display: flex; align-items: center; justify-content: space-between; width: 100%; gap: 2rem; padding: 1rem 1.4rem; border: 0.2rem solid transparent; border-bottom-color: rgba(255, 255, 255, 0.1); font-size: 1.45rem; box-sizing: border-box; }
  .row.on { border-color: #fff; border-radius: 0.3rem; background: rgba(255, 255, 255, 0.1); }
  .row.static { color: rgba(255, 255, 255, 0.65); }
  .val { display: flex; align-items: center; gap: 0.7rem; font-size: 1.3rem; color: #7fe3f0; white-space: nowrap; max-width: 26rem; overflow: hidden; text-overflow: ellipsis; }
  .val b { font-weight: 400; opacity: 0.7; }
  .val i { width: 1.3rem; height: 1.3rem; border-radius: 50%; box-shadow: 0 0 0 0.12rem rgba(255, 255, 255, 0.7); }
  .empty { margin: 1rem 0 0; font-size: 1.3rem; color: rgba(255, 255, 255, 0.65); }
  footer { position: absolute; left: 5.6rem; bottom: 1.4rem; font-size: 1rem; color: rgba(255, 255, 255, 0.6); }
</style>
