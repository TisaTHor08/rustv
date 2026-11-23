<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { SettingsScreenProps } from '../../screens';

  // Réglages style Roku : liste à gauche (blanc sur la ligne ciblée), volet de détail à droite, palette violette.
  let { title, page, categories, rows, index, inCategories, hint, hoverCategory, openCategory, hoverRow, activateRow }: SettingsScreenProps = $props();
  const root = $derived(page === 'root');
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    void index;
    list?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="rs" transition:fade={{ duration: 180 }}>
  <header><b>RusTV</b><span>{t('settings.title')}</span></header>
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
        <p class="empty">{t('roku.settings.open')}</p>
      {/each}
    </div>
  </section>
  <footer>{hint}</footer>
</div>

<style>
  .rs { position: absolute; inset: 0; z-index: 12; color: #fff; font-family: var(--font); background: radial-gradient(70rem 50rem at 10% 0%, #662d91 0%, transparent 70%), linear-gradient(160deg, #3b1259 0%, #1b0a2c 55%, #0b0412 100%); }
  header { position: absolute; left: 6rem; top: 3rem; display: flex; align-items: baseline; gap: 1.2rem; }
  header b { font-size: 1.6rem; font-weight: 700; opacity: 0.8; }
  header span { font-size: 3rem; font-weight: 600; }
  nav { position: absolute; left: 4.6rem; top: 9.6rem; bottom: 5rem; width: 34rem; display: flex; flex-direction: column; gap: 0.2rem; overflow: hidden auto; scrollbar-width: none; }
  button { font-family: inherit; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  .cat { padding: 0.8rem 1.4rem; border-radius: 0.5rem; font-size: 1.55rem; font-weight: 500; color: rgba(255, 255, 255, 0.75); transition: background 0.15s; }
  .cat.cur { color: #fff; font-weight: 700; }
  .cat.on { background: #fff; color: #2b0f45; font-weight: 700; box-shadow: 0 0.4rem 1.2rem rgba(0, 0, 0, 0.35); }
  section { position: absolute; left: 41rem; right: 4rem; top: 3rem; bottom: 4rem; padding: 2.4rem 2.8rem; box-sizing: border-box; border-radius: 0.8rem; background: rgba(12, 4, 22, 0.55); box-shadow: inset 0 0 0 0.1rem rgba(255, 255, 255, 0.12); overflow: hidden; }
  h2 { margin: 0 0 1.4rem; font-size: 2.2rem; font-weight: 700; }
  .rows { height: calc(100% - 4.4rem); overflow: hidden auto; scrollbar-width: none; display: flex; flex-direction: column; gap: 0.2rem; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 2rem; padding: 0.95rem 1.4rem; border-radius: 0.5rem; font-size: 1.4rem; font-weight: 500; transition: background 0.15s; }
  .row.on { background: #fff; color: #2b0f45; font-weight: 700; }
  .row.static { color: rgba(255, 255, 255, 0.7); }
  .val { display: flex; align-items: center; gap: 0.6rem; font-size: 1.25rem; opacity: 0.85; white-space: nowrap; max-width: 24rem; overflow: hidden; text-overflow: ellipsis; }
  .val b { font-size: 1rem; opacity: 0.7; }
  .val i { width: 1.3rem; height: 1.3rem; border-radius: 50%; box-shadow: 0 0 0 0.12rem rgba(255, 255, 255, 0.7); }
  .empty { margin: 1rem 0 0; font-size: 1.3rem; color: rgba(255, 255, 255, 0.7); }
  footer { position: absolute; left: 6rem; bottom: 1.6rem; font-size: 1rem; color: rgba(255, 255, 255, 0.7); }
</style>
