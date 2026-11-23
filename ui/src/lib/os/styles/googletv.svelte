<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../i18n';
  import ModWidgets from '../../ModWidgets.svelte';
  import { iconSrc, type DTile } from '../../tiles.svelte';
  import Weather from '../../Weather.svelte';
  import type { HomeModel } from '../model.svelte';
  import { TABS, TOP_COUNT } from './googletv.meta';

  // Style Google TV (inspiré, non officiel) : barre d'onglets, héros, rangées de cartes. Voir docs/OS-STYLES.md.
  let { model }: { model: HomeModel } = $props();

  const tab = $derived(model.ui.tab ?? 'foryou');
  const tabFocus = $derived<number>(model.ui.tabFocus ?? 2);
  const inTabs = $derived(model.zone === 'tabs');
  const hasHero = $derived(tab === 'foryou' || tab === 'movies' || tab === 'shows');
  const ANCHOR = $derived(hasHero ? 28.6 : 7.6); // rem : position écran de la rangée ciblée
  const ROW_H = (key: string) => (key === 'continue' ? 17.8 : /^g\d+$/.test(key) ? 11.2 : key === 'apps' ? 12.4 : 14.2);
  const STEP = 15.7; // rem : largeur d'une carte + espace

  // Décalage vertical : la rangée ciblée reste ancrée au même endroit (héros) ; en grille, on ne défile que si besoin.
  const shift = $derived.by(() => {
    const f = Math.max(0, model.focus.row);
    const skip = hasHero ? f : Math.max(0, f - 1);
    let y = 0;
    for (let i = 0; i < skip && i < model.rows.length; i++) y += ROW_H(model.rows[i].key);
    return -y;
  });

  // Le héros suit la tuile ciblée avec 150 ms de retard (fondu enchaîné).
  let heroTile = $state<DTile | undefined>(undefined);
  $effect(() => {
    const f = model.focused;
    const id = setTimeout(() => (heroTile = f), heroTile ? 150 : 0);
    return () => clearTimeout(id);
  });
  const bgTint = $derived(model.tinted && heroTile ? heroTile.color : '#4a55a8');
  const fg = (x?: DTile) => (x?.dark_text ? '#16130a' : '#ffffff');
  const entryOf = (i: number) => model.continueEntries[i];
  const pct = (i: number) => {
    const e = entryOf(i);
    return e && e.duration > 0 ? Math.min(100, Math.round((e.position / e.duration) * 100)) : 0;
  };
</script>

<div class="gtv" style:--hue={bgTint}>
  <div class="ambient"></div>

  <!-- Héros -->
  {#if hasHero}
    <section class="hero">
      {#key heroTile?.id}
        <div class="art" transition:fade={{ duration: 320 }} style:--a={heroTile?.color ?? '#2a2f3d'} style:--b={heroTile?.color2 ?? '#12151c'}>
          {#if heroTile?.icon}<img src={iconSrc(heroTile.icon)} alt="" />{:else if heroTile}<span class="big">{heroTile.name[0]}</span>{/if}
        </div>
        {#if heroTile}
          <div class="info" transition:fade={{ duration: 320 }}>
            <span class="kicker">{t(`cat.${heroTile.category}`)}</span>
            <h1>{heroTile.name}</h1>
            <p>{heroTile.desc}</p>
            <div class="btns">
              <span class="open"><svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg>{t('gtv.open')}</span>
              <span class="round" aria-label={t('gtv.add')}><svg viewBox="0 0 24 24"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6z" /></svg></span>
            </div>
          </div>
          <div class="dots" transition:fade={{ duration: 320 }}>{#each [0, 1, 2, 3, 4] as d}<i class:on={d === Math.min(4, model.focus.col % 5)}></i>{/each}</div>
        {/if}
      {/key}
    </section>
  {/if}

  <!-- Barre du haut -->
  <header>
    <div class="left">
      <button class="avatar" class:focus={inTabs && tabFocus === 0} onclick={() => model.openQuick()} onmouseenter={() => ((model.ui.tabFocus = 0), model.setZone('tabs'))} aria-label={t('gtv.profile')}>R</button>
      <div class="pillbar">
        <button class="sbtn" class:focus={inTabs && tabFocus === 1} onclick={() => model.openSearch()} onmouseenter={() => ((model.ui.tabFocus = 1), model.setZone('tabs'))} aria-label={t('gtv.search')}>
          <svg viewBox="0 0 24 24"><path d="M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z" /></svg>
        </button>
        {#each TABS as id, i}
          <button class="tab" class:sel={tab === id} class:focus={inTabs && tabFocus === i + 2} onclick={() => ((model.ui.tab = id), model.focusAt(0, 0, true), model.setZone('content'))} onmouseenter={() => ((model.ui.tabFocus = i + 2), model.setZone('tabs'))}>{t(`gtv.tab.${id}`)}</button>
        {/each}
      </div>
    </div>
    <div class="right">
      {#if model.widgetsNative}
        <ModWidgets toast={model.note} at="status" provided={['status']} />
        <Weather />
        <span class="status" class:on={model.phones > 0} title={model.phonesLabel}>
          <svg viewBox="0 0 24 24"><path d="M17 1H7a2 2 0 0 0-2 2v18a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V3a2 2 0 0 0-2-2Zm0 18H7V5h10Z" /></svg>
          {#if model.phones > 0}<b>{model.phones}</b>{/if}
        </span>
        <span class="clock">{model.time}</span>
      {/if}
    </div>
  </header>

  <!-- Rangées -->
  <main style:--anchor="{ANCHOR}rem">
    <div class="rows" style:--shift="{shift}rem">
      {#each model.rows as r, ri (r.key)}
        {@const act = ri === model.focus.row}
        <div class="row" class:act style:height="{ROW_H(r.key)}rem">
          {#if r.title}<h2 class:small={r.key === 'apps'}>{r.title}</h2>{/if}
          <div class="track" style:--tx="{-Math.max(0, (act ? model.focus.col : 0) - 3) * (r.key === 'continue' ? 18.6 : STEP) * (/^g\d+$/.test(r.key) ? 0 : 1)}rem">
            {#each r.tiles as tile, ci (tile.id)}
              {@const on = act && ci === model.focus.col && !inTabs}
              {#if r.key === 'continue'}
                <button class="card wide" class:on onmouseenter={() => model.focusAt(ri, ci, true)} onclick={() => model.open(tile)}>
                  <span class="face" style:--a="#3b4252" style:--b="#1b1f29">
                    {#if entryOf(ci)?.poster}<img src={iconSrc(entryOf(ci).poster ?? '')} alt="" />{:else}<span class="mark">{tile.name[0]}</span>{/if}
                    <span class="bar"><i style:width="{pct(ci)}%"></i></span>
                  </span>
                  <span class="name"><b>{tile.name}</b><small>{tile.desc.split('·')[0]}</small></span>
                </button>
              {:else}
                <button class="card" class:grid={/^g\d+$/.test(r.key)} class:circ={r.key === 'apps'} class:on onmouseenter={() => model.focusAt(ri, ci, true)} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
                  <span class="face" style:--a={tile.color} style:--b={tile.color2} style:--fg={fg(tile)}>
                    {#if tile.icon}<img class="ico" src={iconSrc(tile.icon)} alt="" />{:else}<span class="letter">{tile.name[0]}</span>{/if}
                  </span>
                  <span class="name"><b>{tile.name}</b></span>
                </button>
              {/if}
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </main>
</div>

<style>
  .gtv { position: absolute; inset: 0; background: var(--bg); overflow: hidden; font-family: var(--font); }
  .ambient { position: absolute; inset: 0; background: radial-gradient(60rem 40rem at 82% -8%, color-mix(in srgb, var(--hue) 34%, transparent), transparent 70%), radial-gradient(52rem 38rem at 6% 8%, rgba(95, 80, 170, 0.28), transparent 70%); transition: background 0.8s; }

  header { position: absolute; z-index: 5; top: 1.8rem; left: 4.2rem; right: 4.2rem; display: flex; align-items: center; justify-content: space-between; }
  .left { display: flex; align-items: center; gap: 1.8rem; }
  nav { display: flex; gap: 0.4rem; }
  button { font-family: inherit; border: 0; cursor: pointer; color: var(--text); background: transparent; }
  .pillbar { display: flex; align-items: center; gap: 0.2rem; padding: 0.4rem; border-radius: 99rem; background: var(--glass); }
  .sbtn { width: 3.2rem; height: 3.2rem; border-radius: 50%; display: grid; place-items: center; }
  .sbtn svg { width: 1.6rem; height: 1.6rem; fill: currentColor; }
  .search { display: flex; align-items: center; gap: 0.7rem; height: 3.2rem; padding: 0 1.4rem 0 1.1rem; border-radius: 99rem; background: var(--glass); font-size: 1.25rem; }
  .search svg { width: 1.6rem; height: 1.6rem; fill: currentColor; }
  .tab { height: 3.2rem; padding: 0 1.4rem; border-radius: 99rem; font-size: 1.3rem; font-weight: 500; color: var(--muted); transition: background 0.2s, color 0.2s; }
  .tab.sel { color: var(--text); font-weight: 700; }
  .tab.sel:not(.focus)::after { content: ''; display: block; height: 0.18rem; margin: 0.1rem auto -0.3rem; width: 1.6rem; border-radius: 1rem; background: var(--text); }
  .focus.tab, .focus.sbtn, .focus.avatar { background: #e8eaed; color: #1f1f1f; }
  .right { display: flex; align-items: center; gap: 1.3rem; }
  .status { display: flex; align-items: center; gap: 0.3rem; color: var(--muted); }
  .status svg { width: 1.5rem; height: 1.5rem; fill: currentColor; }
  .status.on { color: #81c995; }
  .status b { font-size: 1.05rem; font-weight: 500; }
  .clock { font-size: 1.35rem; font-weight: 500; }
  .avatar { width: 3.2rem; height: 3.2rem; border-radius: 50%; font-size: 1.4rem; font-weight: 700; background: linear-gradient(135deg, #7c4dff, #3f7cff); color: #fff; }
  .avatar.focus { box-shadow: 0 0 0 0.2rem #fff; background: linear-gradient(135deg, #7c4dff, #3f7cff); color: #fff; }

  /* héros */
  .hero { position: absolute; left: 0; right: 0; top: 0; height: 33rem; z-index: 1; }
  .art { position: absolute; inset: 0; background: radial-gradient(48rem 30rem at 78% 30%, var(--a), transparent 72%), linear-gradient(135deg, var(--b), transparent 70%); }
  .art::after { content: ''; position: absolute; inset: 0; background: linear-gradient(90deg, rgba(var(--bg-rgb), 0.95) 6%, rgba(var(--bg-rgb), 0.55) 42%, transparent 78%), linear-gradient(0deg, var(--bg) 2%, transparent 48%); }
  .art .big { position: absolute; right: 8rem; top: 5rem; font-size: 24rem; font-weight: 700; line-height: 1; opacity: 0.16; }
  .art img { position: absolute; right: 14rem; top: 8rem; width: 14rem; height: 14rem; object-fit: contain; opacity: 0.9; }
  .info { position: absolute; z-index: 2; left: 4.2rem; top: 9rem; width: 52rem; }
  .kicker { font-size: 1.1rem; font-weight: 500; letter-spacing: 0.1em; text-transform: uppercase; color: var(--muted); }
  h1 { margin: 0.5rem 0 0.5rem; font-size: 4.2rem; font-weight: 400; line-height: 1.05; letter-spacing: -0.01em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .info p { margin: 0 0 1.6rem; font-size: 1.4rem; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .btns { display: flex; align-items: center; gap: 1rem; }
  .open { display: inline-flex; align-items: center; gap: 0.6rem; height: 3.6rem; padding: 0 1.8rem 0 1.4rem; border-radius: 99rem; background: #e8eaed; color: #1f1f1f; font-size: 1.3rem; font-weight: 500; }
  .open svg { width: 1.7rem; height: 1.7rem; fill: currentColor; }
  .round { width: 3.6rem; height: 3.6rem; border-radius: 50%; display: grid; place-items: center; background: var(--glass-strong); }
  .round svg { width: 1.8rem; height: 1.8rem; fill: var(--text); }
  .dots { position: absolute; z-index: 2; right: 4.2rem; top: 22.4rem; display: flex; gap: 0.5rem; }
  .dots i { width: 0.55rem; height: 0.55rem; border-radius: 50%; background: var(--glass-strong); }
  .dots i.on { background: var(--text); }

  /* rangées */
  main { position: absolute; z-index: 2; left: 0; right: 0; top: var(--anchor); bottom: 0; overflow: hidden; transition: top 0.4s var(--ease); }
  .rows { transform: translateY(var(--shift)); transition: transform 0.45s var(--ease); padding-top: 1rem; }
  .row { padding-left: 4.2rem; }
  .row h2 { margin: 0 0 1rem; font-size: 1.45rem; font-weight: 500; color: var(--text); }
  .track { display: flex; gap: 1.5rem; width: max-content; transform: translateX(var(--tx)); transition: transform 0.45s var(--ease); padding-top: 0.5rem; }
  .card { flex: none; width: 14.2rem; padding: 0; text-align: left; transition: transform 0.25s var(--ease); }
  .card.wide { width: 17rem; }
  .card.grid { width: 13rem; }
  .face { position: relative; display: block; width: 100%; aspect-ratio: 16 / 9; border-radius: var(--r); overflow: hidden; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg, #fff); transition: box-shadow 0.25s; }
  .letter { position: absolute; inset: 0; display: grid; place-items: center; font-size: 3.4rem; font-weight: 700; }
  .ico { position: absolute; left: 50%; top: 50%; width: 3.8rem; height: 3.8rem; transform: translate(-50%, -50%); object-fit: contain; }
  .face img:not(.ico) { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .mark { position: absolute; inset: 0; display: grid; place-items: center; font-size: 5rem; font-weight: 700; opacity: 0.25; }
  .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 0.3rem; background: rgba(255, 255, 255, 0.3); }
  .bar i { display: block; height: 100%; background: #8ab4f8; }
  .name { display: block; margin-top: 0.6rem; opacity: 0; transition: opacity 0.2s; }
  .name b { display: block; font-size: 1.15rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .name small { display: block; font-size: 0.95rem; color: var(--muted); }
  .card.wide .name { opacity: 1; }
  .card.circ { width: 7.4rem; }
  .card.circ .face { aspect-ratio: 1; border-radius: 50%; }
  .card.circ .name { text-align: center; }
  .row h2.small { font-size: 1.1rem; color: var(--muted); }
  .card.on { transform: scale(var(--focus-scale)); z-index: 3; }
  .card.on .face { box-shadow: 0 0 0 var(--focus-ring) #fff, 0 1.2rem 2.4rem rgba(0, 0, 0, 0.5); }
  .card.on .name { opacity: 1; }
  .row:not(.act) .card.wide .name { opacity: 0.8; }
</style>
