<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { SettingsScreenProps } from '../../screens';

  // Réglages style Fire TV : à la racine, une rangée de grandes tuiles (catégories) ; dans une page, liste à gauche (ligne ciblée à contour blanc)
  // et volet d'explication à droite. Fond gris très sombre, pas de couleur d'accent.
  let { title, page, categories, rows, index, inCategories, hint, hoverCategory, openCategory, hoverRow, activateRow }: SettingsScreenProps = $props();
  const root = $derived(page === 'root');
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    void index;
    list?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  });
  const sel = $derived(rows[index]);
</script>

<div class="fs" transition:fade={{ duration: 180 }}>
  <h1>{root ? t('settings.title') : title}</h1>
  {#if root}
    <div class="tiles">
      {#each categories as c, i}
        <button class="tile" class:on={inCategories && i === index} onmouseenter={() => hoverCategory(i)} onclick={() => openCategory(i)}><span>{c.label}</span></button>
      {/each}
    </div>
    <p class="sub">{t('fire.settings.pick')}</p>
  {:else}
    <div class="split">
      <div class="rows" bind:this={list}>
        {#each rows as r, i}
          <button class="row" class:on={i === index} class:static={!r.actionable} onmouseenter={() => hoverRow(i)} onclick={() => activateRow(i)}>
            <span class="lb">{r.label}</span>
            <span class="val">{#if r.swatch}<i style:background={r.swatch}></i>{/if}{#if r.arrows}<b>&lsaquo;</b>{/if}{r.value ?? ''}{#if r.arrows}<b>&rsaquo;</b>{/if}</span>
          </button>
        {/each}
      </div>
      <aside>
        <h3>{sel?.label ?? ''}</h3>
        {#if sel?.value}<p class="big">{sel.value}</p>{/if}
      </aside>
    </div>
  {/if}
  <footer>{hint}</footer>
</div>

<style>
  .fs { position: absolute; inset: 0; z-index: 12; box-sizing: border-box; padding: 4.4rem 5.2rem 0; color: #fff; font-family: var(--font); background: #171717; }
  button { font-family: inherit; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  h1 { margin: 0 0 3rem; font-size: 3rem; font-weight: 500; letter-spacing: 0.02em; }
  .tiles { display: grid; grid-template-columns: repeat(5, 1fr); gap: 0.6rem; }
  .tile { display: grid; place-items: center; aspect-ratio: 16 / 10; border-radius: var(--r); background: linear-gradient(145deg, #3a3a3a, #262626); font-size: 1.6rem; font-weight: 500; text-align: center; padding: 0 1rem; transition: transform 0.18s var(--ease); }
  .tile.on { transform: scale(1.05); box-shadow: 0 0 0 0.2rem #fff, 0 0.8rem 2rem rgba(0, 0, 0, 0.55); position: relative; z-index: 2; }
  .sub { margin: 2.4rem 0 0; font-size: 1.3rem; color: rgba(255, 255, 255, 0.6); letter-spacing: 0.04em; }
  .split { display: grid; grid-template-columns: 1fr 30rem; gap: 4rem; height: calc(100% - 12rem); }
  .rows { overflow: hidden auto; scrollbar-width: none; display: flex; flex-direction: column; gap: 0.3rem; padding: 0.4rem; margin: -0.4rem; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 2rem; min-height: 4.6rem; padding: 0 1.8rem; border-radius: var(--r); background: #242424; font-size: 1.5rem; font-weight: 400; }
  .row.on { background: #333; box-shadow: 0 0 0 0.2rem #fff; position: relative; }
  .row.static { color: rgba(255, 255, 255, 0.65); }
  .val { display: flex; align-items: center; gap: 0.7rem; font-size: 1.35rem; color: rgba(255, 255, 255, 0.82); white-space: nowrap; max-width: 28rem; overflow: hidden; text-overflow: ellipsis; }
  .val b { font-size: 1.8rem; font-weight: 400; opacity: 0.75; }
  .val i { width: 1.3rem; height: 1.3rem; border-radius: 50%; box-shadow: 0 0 0 0.12rem rgba(255, 255, 255, 0.7); }
  aside { padding-top: 0.6rem; border-left: 0.1rem solid rgba(255, 255, 255, 0.12); padding-left: 2.4rem; }
  aside h3 { margin: 0 0 1rem; font-size: 1.6rem; font-weight: 500; }
  aside .big { margin: 0; font-size: 2.4rem; font-weight: 300; color: rgba(255, 255, 255, 0.85); word-break: break-word; }
  footer { position: absolute; left: 5.2rem; bottom: 1.8rem; font-size: 1.05rem; color: rgba(255, 255, 255, 0.55); }
</style>
