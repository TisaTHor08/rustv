<script lang="ts">
  import { t } from '../../i18n';
  import ModWidgets from '../../ModWidgets.svelte';
  import { iconSrc, type DTile } from '../../tiles.svelte';
  import Weather from '../../Weather.svelte';
  import type { HomeModel } from '../model.svelte';
  import { scrollFocus, tileFg } from '../util';
  import { MENU } from './roku.meta';

  // Style Roku (inspiré, non officiel) : menu à gauche, grille de tuiles 4 colonnes. Voir docs/OS-STYLES.md.
  let { model }: { model: HomeModel } = $props();

  const inMenu = $derived(model.zone === 'menu');
  const cur = $derived<string>(model.ui.menu ?? 'home');
  const menuFocus = $derived<number>(model.ui.menuFocus ?? Math.max(0, MENU.findIndex((e) => e.id === cur)));
  const tiles = $derived(model.rows[0]?.tiles ?? []);

  // La tuile ciblée reste visible : défilement doux (sans mouvement si l'utilisateur le demande).
  let grid: HTMLElement | undefined = $state();
  $effect(() => {
    void model.focus.col;
    if (!grid || inMenu) return;
    scrollFocus(grid.querySelector('.tile.on'));
  });
</script>

<div class="roku">
  <aside>
    <div class="brand"><span class="logo"><svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg></span><b>RusTV</b></div>
    <nav>
      {#each MENU as e, i}
        <button class="item" class:sel={cur === e.id && e.kind === 'content'} class:focus={inMenu && menuFocus === i} onclick={() => (e.id === 'search' ? model.openSearch() : e.id === 'live' ? model.openLive() : e.id === 'settings' ? model.openSettings() : ((model.ui.menu = e.id), model.focusAt(0, 0, true), model.setZone('content')))} onmouseenter={() => ((model.ui.menuFocus = i), model.setZone('menu'))}>
          <span>{t(`roku.${e.id}`)}</span>
        </button>
      {/each}
    </nav>
  </aside>

  <header>
    {#if model.widgetsNative}
      <ModWidgets toast={model.note} at="status" provided={['status']} />
      <Weather />
      <span class="chip" class:on={model.phones > 0}><i></i>{model.phonesLabel}</span>
      <span class="clock">{model.time}</span>
    {/if}
  </header>

  <main bind:this={grid}>
    {#if tiles.length}
      <div class="count">{tiles.length} {t('roku.items')}</div>
      <div class="grid">
        {#each tiles as tile, i (tile.id)}
          {@const on = !inMenu && i === model.focus.col}
          <button class="tile" class:on onmouseenter={() => model.focusAt(0, i, true)} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
            <span class="art" style:--a={tile.color} style:--b={tile.color2} style:--fg={tileFg(tile.dark_text)}>
              {#if tile.icon}<img src={iconSrc(tile.icon)} alt="" />{:else}<b>{tile.name[0]}</b>{/if}
            </span>
            <span class="name">{tile.name}</span>
          </button>
        {/each}
      </div>
    {:else}
      <p class="empty">{t('roku.empty')}</p>
    {/if}
  </main>

  <footer>
    <span><kbd>OK</kbd> {t('roku.ok')}</span>
    <span><kbd>*</kbd> {t('roku.options')}</span>
  </footer>
</div>

<style>
  .roku { position: absolute; inset: 0; overflow: hidden; font-family: var(--font); color: #fff; background: radial-gradient(70rem 50rem at 10% 0%, #662d91 0%, transparent 70%), linear-gradient(160deg, #3b1259 0%, #1b0a2c 55%, #0b0412 100%); }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; padding: 0; }

  aside { position: absolute; left: 0; top: 0; bottom: 0; width: 22%; min-width: 21rem; padding: 3rem 1.6rem 0 2.6rem; background: linear-gradient(90deg, rgba(10, 3, 18, 0.45), transparent); }
  .brand { display: flex; align-items: center; gap: 0.8rem; margin-bottom: 2.2rem; font-size: 1.5rem; font-weight: 700; letter-spacing: 0.02em; }
  .logo { width: 2.4rem; height: 2.4rem; border-radius: 0.6rem; background: #6f1ab1; display: grid; place-items: center; }
  .logo svg { width: 1.4rem; height: 1.4rem; fill: #fff; }
  nav { display: flex; flex-direction: column; gap: 0.3rem; }
  .item { display: flex; align-items: center; gap: 1.1rem; height: 3.6rem; padding: 0 1.2rem; border-radius: 0.5rem; font-size: 1.4rem; font-weight: 500; color: rgba(255, 255, 255, 0.78); text-align: left; transition: background 0.2s; }
  .item.sel { color: #fff; }
  .item.focus { background: #fff; color: #2b0f45; font-weight: 700; box-shadow: 0 0.4rem 1.2rem rgba(0, 0, 0, 0.35); }

  header { position: absolute; top: 1.8rem; right: 3.6rem; display: flex; align-items: center; gap: 1.4rem; z-index: 4; font-size: 1.2rem; }
  .chip { display: inline-flex; align-items: center; gap: 0.5rem; color: var(--muted); }
  .chip i { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: #9a8bb0; }
  .chip.on { color: #fff; }
  .chip.on i { background: #3ddc84; }
  .clock { font-size: 1.5rem; font-weight: 600; }

  main { scroll-padding: 2rem; position: absolute; left: max(24rem, 24%); right: 3.4rem; top: 6.2rem; bottom: 5.4rem; overflow-y: auto; overflow-x: hidden; padding: 1.4rem 1.4rem 2rem; scrollbar-width: none; }
  main::-webkit-scrollbar { display: none; }
  .grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 1.6rem 1.6rem; max-width: 74rem; }
  .tile { display: block; text-align: left; transition: transform 0.2s var(--ease); }
  .art { position: relative; display: grid; place-items: center; aspect-ratio: 16 / 9; border-radius: var(--r); overflow: hidden; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg); box-shadow: 0 0.4rem 1rem rgba(0, 0, 0, 0.4); }
  .art b { font-size: 4rem; font-weight: 800; }
  .art img { width: 4.6rem; height: 4.6rem; object-fit: contain; }
  .name { display: block; margin-top: 0.55rem; font-size: 1.1rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: rgba(255, 255, 255, 0.92); }
  .tile.on { transform: scale(var(--focus-scale)); position: relative; z-index: 2; }
  .tile.on .art { box-shadow: 0 0 0 var(--focus-ring) #fff, 0 1rem 2rem rgba(0, 0, 0, 0.55); }
  .tile.on .name { font-weight: 700; }
  .count { text-align: right; font-size: 1rem; color: var(--muted); max-width: 74rem; margin: -0.6rem 0 0.6rem; }
  .empty { margin-top: 6rem; font-size: 1.4rem; color: var(--muted); }

  footer { position: absolute; left: max(24rem, 24%); right: 3.4rem; bottom: 1.6rem; display: flex; gap: 2rem; justify-content: flex-end; font-size: 1.05rem; color: var(--muted); }
  kbd { font: inherit; font-weight: 700; padding: 0.1rem 0.6rem; border-radius: 0.4rem; background: rgba(255, 255, 255, 0.16); color: #fff; margin-right: 0.3rem; }
</style>
