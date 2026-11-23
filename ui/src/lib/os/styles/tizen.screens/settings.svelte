<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { SettingsScreenProps } from '../../screens';

  // Réglages style Tizen : deux cartes sombres bordées de clair (catégories à gauche, lignes à droite), ligne ciblée en blanc,
  // valeur en second texte atténué, flèche de liste déroulante pour les choix (comme les panneaux de réglages One UI).
  let { title, page, categories, rows, index, inCategories, hint, hoverCategory, openCategory, hoverRow, activateRow, back }: SettingsScreenProps = $props();
  const root = $derived(page === 'root');
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    void index;
    list?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="ts" transition:fade={{ duration: 180 }}>
  <aside class="card cats">
    <h1>{t('settings.title')}</h1>
    <div class="scroll">
      {#each categories as c, i}
        <button class="cat" class:on={inCategories && i === index} class:cur={c.active} onmouseenter={() => hoverCategory(i)} onclick={() => openCategory(i)}>{c.label}</button>
      {/each}
    </div>
  </aside>
  <section class="card detail">
    <header>
      <button class="bk" aria-label={t('common.back')} onclick={back}><svg viewBox="0 0 24 24"><path d="M15.4 5.4 14 4l-8 8 8 8 1.4-1.4L8.8 12Z" /></svg></button>
      <h2>{root ? (categories[index]?.label ?? '') : title}</h2>
    </header>
    <div class="rows" bind:this={list}>
      {#each rows as r, i}
        <button class="row" class:on={!inCategories && i === index} class:static={!r.actionable} onmouseenter={() => !root && hoverRow(i)} onclick={() => !root && activateRow(i)}>
          <span class="tx"><b>{r.label}</b>{#if r.value || r.swatch}<small>{#if r.swatch}<i style:background={r.swatch}></i>{/if}{r.value ?? ''}</small>{/if}</span>
          {#if r.arrows && !root}<svg class="dd" viewBox="0 0 24 24"><path d="M7 10l5 5 5-5Z" /></svg>{/if}
        </button>
      {:else}
        <p class="empty">{t('tizen.settings.pick')}</p>
      {/each}
    </div>
    <footer>{hint}</footer>
  </section>
</div>

<style>
  .ts { position: absolute; inset: 0; z-index: 12; display: flex; gap: 2rem; padding: 4rem 6rem; box-sizing: border-box; color: #f4f5fa; font-family: var(--font); background: radial-gradient(80rem 50rem at 20% 0%, #26285a 0%, transparent 70%), linear-gradient(160deg, #14162f 0%, #0a0b17 70%); }
  .card { box-sizing: border-box; border-radius: 2.4rem; background: linear-gradient(160deg, rgba(60, 62, 110, 0.55), rgba(24, 25, 46, 0.85)); box-shadow: inset 0 0 0 0.1rem rgba(255, 255, 255, 0.28), 0 2rem 5rem rgba(0, 0, 0, 0.5); backdrop-filter: blur(1.6rem); }
  .cats { flex: none; width: 34rem; padding: 2.2rem 1.4rem 1.4rem; display: flex; flex-direction: column; }
  h1 { margin: 0 1rem 1.2rem; font-size: 2.4rem; font-weight: 600; }
  .scroll { flex: 1; overflow: hidden auto; scrollbar-width: none; display: flex; flex-direction: column; gap: 0.3rem; }
  button { font-family: inherit; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  .cat { padding: 1rem 1.4rem; border-radius: 1.4rem; font-size: 1.55rem; font-weight: 500; transition: background 0.15s; }
  .cat.cur { background: rgba(255, 255, 255, 0.12); }
  .cat.on { background: #fff; color: #10131c; }
  .detail { flex: 1; min-width: 0; padding: 1.6rem 1.8rem 1.2rem; display: flex; flex-direction: column; }
  header { display: flex; align-items: center; gap: 1rem; margin-bottom: 1rem; }
  header h2 { margin: 0; font-size: 2rem; font-weight: 600; }
  .bk { width: 3.4rem; height: 3.4rem; border-radius: 50%; display: grid; place-items: center; }
  .bk svg { width: 2.2rem; height: 2.2rem; fill: currentColor; }
  .rows { flex: 1; overflow: hidden auto; scrollbar-width: none; display: flex; flex-direction: column; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 2rem; padding: 1rem 1.6rem; border-radius: 1.4rem; border-bottom: 0.08rem solid rgba(255, 255, 255, 0.1); transition: background 0.15s; }
  .row.on { background: #fff; color: #10131c; border-bottom-color: transparent; }
  .row.static { opacity: 0.85; }
  .tx { display: flex; flex-direction: column; gap: 0.2rem; min-width: 0; }
  .tx b { font-size: 1.5rem; font-weight: 500; }
  .tx small { display: flex; align-items: center; gap: 0.6rem; font-size: 1.2rem; color: rgba(255, 255, 255, 0.62); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .row.on small { color: #5b6070; }
  .tx i { width: 1.2rem; height: 1.2rem; border-radius: 50%; box-shadow: 0 0 0 0.1rem rgba(255, 255, 255, 0.7); }
  .dd { flex: none; width: 2.2rem; height: 2.2rem; fill: currentColor; opacity: 0.7; }
  .empty { margin: 1rem; font-size: 1.4rem; color: rgba(255, 255, 255, 0.65); }
  footer { margin-top: 0.8rem; font-size: 1rem; color: rgba(255, 255, 255, 0.55); }
</style>
