<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../i18n';
  import ModWidgets from '../../ModWidgets.svelte';
  import { iconSrc, type DTile } from '../../tiles.svelte';
  import Weather from '../../Weather.svelte';
  import type { HomeModel } from '../model.svelte';
  import { tileFg } from '../util';
  import { NAV } from './fire.meta';

  // Style Fire TV (inspiré, non officiel) : barre de navigation, grand héros, rangées de cartes 16:9. Voir docs/OS-STYLES.md.
  let { model }: { model: HomeModel } = $props();

  const tab = $derived<string>(model.ui.tab ?? 'home');
  const navFocus = $derived<number>(model.ui.navFocus ?? Math.max(0, NAV.findIndex((e) => e.id === tab)));
  const inNav = $derived(model.zone === 'nav');
  const hasHero = $derived(tab === 'home');
  const grid = (key: string) => /^g\d+$/.test(key);
  const ROW_H = (key: string) => (key === 'continue' ? 18.4 : 15.4); // rem : titre + cartes + libellé
  const STEP = 15.6; // rem : carte 14 + espace 1.6
  const WIDE = 18.6;

  // Largeur utile en rem : sert à décider quand la piste défile (écrans larges, portrait...).
  let W = $state(0);
  let rem = $state(16);
  $effect(() => {
    const f = () => (rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16);
    f();
    addEventListener('resize', f);
    return () => removeEventListener('resize', f);
  });
  const usable = $derived(Math.min(W / rem, 150));

  const shift = $derived.by(() => {
    const f = Math.max(0, model.focus.row);
    const skip = Math.max(0, f - 1);
    let y = 0;
    for (let i = 0; i < skip && i < model.rows.length; i++) y += ROW_H(model.rows[i].key);
    return -y;
  });
  const txOf = (key: string, act: boolean) => {
    if (grid(key)) return 0;
    const step = key === 'continue' ? WIDE : STEP;
    const vis = Math.max(2, Math.floor((usable - 8) / step));
    return -Math.max(0, (act ? model.focus.col : 0) - (vis - 1)) * step;
  };

  // Le héros suit la tuile ciblée avec un léger retard (fondu enchaîné).
  let heroTile = $state<DTile | undefined>(undefined);
  $effect(() => {
    const f = model.focused;
    const id = setTimeout(() => (heroTile = f), heroTile ? 150 : 0);
    return () => clearTimeout(id);
  });
  const entryOf = (i: number) => model.continueEntries[i];
  const pct = (i: number) => {
    const e = entryOf(i);
    return e && e.duration > 0 ? Math.min(100, Math.round((e.position / e.duration) * 100)) : 0;
  };
  const press = (i: number) => {
    const e = NAV[i];
    if (e.id === 'search') model.openSearch();
    else if (e.id === 'live') model.openLive();
    else if (e.id === 'settings') model.openSettings();
    else ((model.ui.tab = e.id), model.focusAt(0, 0, true), model.setZone('content'));
  };
  const hover = (i: number) => ((model.ui.navFocus = i), model.setZone('nav'));
  const rowTop = $derived(hasHero && model.focus.row <= 0 ? 'var(--hh)' : '6.6rem');
</script>

<div class="fire" bind:clientWidth={W} style:--hh="max(27rem, 36%)">
  <!-- Héros -->
  {#if hasHero}
    <section class="hero" style:transform="translateY({model.focus.row > 0 ? -5 : 0}rem)" style:opacity={model.focus.row > 0 ? 0 : 1}>
      {#key heroTile?.id}
        <div class="art" transition:fade={{ duration: 300 }} style:--a={heroTile?.color ?? '#233044'} style:--b={heroTile?.color2 ?? '#0f171e'}>
          <div class="stage">
            {#if heroTile?.icon}<img src={iconSrc(heroTile.icon)} alt="" />{:else if heroTile}<span class="big">{heroTile.name[0]}</span>{/if}
          </div>
        </div>
        {#if heroTile}
          <div class="info" transition:fade={{ duration: 300 }}>
            <span class="kicker">{t(`cat.${heroTile.category}`)}</span>
            <h1>{heroTile.name}</h1>
            <p>{heroTile.desc}</p>
            <div class="btns">
              <span class="open"><svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg>{t('os.fire.open')}</span>
              <span class="more">{t('os.fire.more')}</span>
            </div>
            <div class="dots">{#each [0, 1, 2, 3, 4] as d}<i class:on={d === Math.min(4, model.focus.col % 5)}></i>{/each}</div>
          </div>
        {/if}
      {/key}
    </section>
  {/if}

  <!-- Barre de navigation -->
  <header>
    <nav>
      {#each NAV as e, i}
        {#if e.id !== 'settings'}
          <button class="item" class:sq={e.icon} class:sel={tab === e.id && e.kind === 'content'} class:focus={inNav && navFocus === i} onclick={() => press(i)} onmouseenter={() => hover(i)} aria-label={t(`os.fire.${e.id}`)}>
            {#if e.id === 'search'}<svg viewBox="0 0 24 24"><path d="M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z" /></svg>{:else}{t(`os.fire.${e.id}`)}{/if}
          </button>
        {/if}
      {/each}
    </nav>
    <div class="right">
      {#if model.widgetsNative}
        <ModWidgets toast={model.note} at="status" provided={['status']} />
        <Weather />
        {#if model.phones > 0}<span class="phones" title={model.phonesLabel}><svg viewBox="0 0 24 24"><path d="M17 1H7a2 2 0 0 0-2 2v18a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V3a2 2 0 0 0-2-2Zm0 18H7V5h10Z" /></svg>{model.phones}</span>{/if}
        <span class="clock">{model.time}</span>
      {/if}
      <button class="item sq gear" class:focus={inNav && navFocus === NAV.length - 1} onclick={() => press(NAV.length - 1)} onmouseenter={() => hover(NAV.length - 1)} aria-label={t('os.fire.settings')}>
        <svg viewBox="0 0 24 24"><path d="M19.4 13a7.5 7.5 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3.2h-4L10.7 5.9a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.5 7.5 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1a7.4 7.4 0 0 0 1.7 1l.3 2.7h4l.3-2.7a7.4 7.4 0 0 0 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z" transform="translate(-1 0)" /></svg>
      </button>
    </div>
  </header>

  <!-- Rangées -->
  <main style:top={rowTop}>
    <div class="rows" style:--shift="{shift}rem">
      {#each model.rows as r, ri (r.key)}
        {@const act = ri === model.focus.row}
        <div class="row" class:act style:height="{ROW_H(r.key)}rem">
          {#if r.title}<h2>{r.title}</h2>{/if}
          <div class="track" style:--tx="{txOf(r.key, act)}rem">
            {#each r.tiles as tile, ci (tile.id)}
              {@const on = act && ci === model.focus.col && !inNav}
              {#if r.key === 'continue'}
                <button class="card wide" class:on onmouseenter={() => model.focusAt(ri, ci, true)} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
                  <span class="face" style:--a="#2d3b4a" style:--b="#131c25">
                    {#if entryOf(ci)?.poster}<img class="cover" src={iconSrc(entryOf(ci).poster ?? '')} alt="" />{:else}<span class="mark">{tile.name[0]}</span>{/if}
                    <span class="bar"><i style:width="{pct(ci)}%"></i></span>
                  </span>
                  <span class="name"><b>{tile.name}</b><small>{tile.desc.split('·')[0]}</small></span>
                </button>
              {:else}
                <button class="card" class:on onmouseenter={() => model.focusAt(ri, ci, true)} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
                  <span class="face" style:--a={tile.color} style:--b={tile.color2} style:--fg={tileFg(tile.dark_text)}>
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
  .fire { --pad: max(4rem, calc((100% - 140rem) / 2)); position: absolute; inset: 0; background: var(--bg); overflow: hidden; font-family: var(--font); color: var(--text); }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; padding: 0; }

  header { position: absolute; z-index: 5; top: 0; left: 0; right: 0; height: 6.2rem; padding: 0 var(--pad); display: flex; align-items: center; justify-content: space-between; background: linear-gradient(180deg, rgba(var(--bg-rgb), 0.92), transparent); }
  nav { display: flex; align-items: center; gap: 0.4rem; }
  .item { position: relative; height: 3.4rem; padding: 0 1.4rem; border-radius: 0.6rem; font-size: 1.35rem; font-weight: 500; color: var(--muted); display: inline-flex; align-items: center; transition: background 0.15s, color 0.15s; }
  .item.sq { width: 3.4rem; padding: 0; justify-content: center; }
  .item svg { width: 1.8rem; height: 1.8rem; fill: currentColor; }
  .item.sel { color: var(--text); font-weight: 700; }
  .item.sel:not(.focus)::after { content: ''; position: absolute; left: 1.2rem; right: 1.2rem; bottom: 0.1rem; height: 0.22rem; border-radius: 1rem; background: var(--text); }
  .item.focus { color: var(--text); background: var(--glass-strong); box-shadow: inset 0 0 0 0.18rem #fff; font-weight: 700; }
  .right { display: flex; align-items: center; gap: 1.2rem; }
  .clock { font-size: 1.35rem; font-weight: 500; color: var(--muted); }
  .phones { display: inline-flex; align-items: center; gap: 0.3rem; font-size: 1.1rem; color: #7ed39a; }
  .phones svg { width: 1.3rem; height: 1.3rem; fill: currentColor; }

  /* héros */
  .hero { position: absolute; z-index: 1; left: 0; right: 0; top: 0; height: var(--hh); transition: transform 0.45s var(--ease), opacity 0.35s; }
  .art { position: absolute; inset: 0; background: radial-gradient(60rem 34rem at 76% 32%, var(--a), transparent 72%), linear-gradient(120deg, var(--b), transparent 65%); }
  .art::after { content: ''; position: absolute; inset: 0; background: linear-gradient(90deg, rgba(var(--bg-rgb), 0.96) 8%, rgba(var(--bg-rgb), 0.5) 46%, transparent 80%), linear-gradient(0deg, var(--bg) 0%, transparent 42%); }
  .stage { position: absolute; inset: 0 var(--pad); }
  .big { position: absolute; right: 4rem; top: 50%; transform: translateY(-50%); font-size: 26rem; font-weight: 700; line-height: 1; opacity: 0.16; }
  .stage img { position: absolute; right: 10rem; top: 50%; transform: translateY(-50%); width: 15rem; height: 15rem; object-fit: contain; opacity: 0.92; }
  .info { position: absolute; z-index: 2; left: var(--pad); bottom: 3.6rem; width: min(56rem, 70%); }
  .kicker { font-size: 1.15rem; font-weight: 500; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); }
  h1 { margin: 0.4rem 0 0.5rem; font-size: 4.4rem; font-weight: 700; line-height: 1.05; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .info p { margin: 0 0 1.5rem; font-size: 1.45rem; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .btns { display: flex; align-items: center; gap: 1.2rem; }
  .open { display: inline-flex; align-items: center; gap: 0.6rem; height: 3.6rem; padding: 0 2rem 0 1.5rem; border-radius: 0.6rem; background: #fff; color: #0f171e; font-size: 1.35rem; font-weight: 700; box-shadow: 0 0 0 0.25rem rgba(255, 255, 255, 0.35); }
  .open svg { width: 1.7rem; height: 1.7rem; fill: currentColor; }
  .more { font-size: 1.25rem; font-weight: 500; color: var(--muted); }
  .dots { display: flex; gap: 0.5rem; margin-top: 1.4rem; }
  .dots i { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: var(--glass-strong); }
  .dots i.on { background: #fff; }

  /* rangées */
  main { position: absolute; z-index: 2; left: 0; right: 0; bottom: 0; overflow: hidden; transition: top 0.4s var(--ease); }
  .rows { transform: translateY(var(--shift)); transition: transform 0.45s var(--ease); padding-top: 0.6rem; }
  .row { padding-left: var(--pad); }
  .row h2 { margin: 0 0 0.9rem; font-size: 1.55rem; font-weight: 500; }
  .track { display: flex; gap: 1.6rem; width: max-content; transform: translateX(var(--tx)); transition: transform 0.45s var(--ease); padding: 0.6rem 0 0.4rem 0.6rem; margin-left: -0.6rem; }
  .card { flex: none; width: 14rem; text-align: left; transition: transform 0.22s var(--ease); }
  .card.wide { width: 17rem; }
  .face { position: relative; display: block; width: 100%; aspect-ratio: 16 / 9; border-radius: var(--r); overflow: hidden; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg, #fff); transition: box-shadow 0.2s; }
  .letter { position: absolute; inset: 0; display: grid; place-items: center; font-size: 3.4rem; font-weight: 700; }
  .ico { position: absolute; left: 50%; top: 50%; width: 4rem; height: 4rem; transform: translate(-50%, -50%); object-fit: contain; }
  .cover { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .mark { position: absolute; inset: 0; display: grid; place-items: center; font-size: 5rem; font-weight: 700; opacity: 0.25; }
  .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 0.3rem; background: rgba(255, 255, 255, 0.28); }
  .bar i { display: block; height: 100%; background: #ff9900; }
  .name { display: block; margin-top: 0.7rem; opacity: 0; transition: opacity 0.18s; }
  .name b { display: block; font-size: 1.2rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .name small { display: block; font-size: 1rem; color: var(--muted); }
  .card.wide .name { opacity: 0.85; }
  .card.on { transform: scale(var(--focus-scale)); position: relative; z-index: 3; }
  .card.on .face { box-shadow: 0 0 0 var(--focus-ring) #fff, 0 0 2rem 0.2rem rgba(255, 190, 90, 0.28), 0 1rem 2.2rem rgba(0, 0, 0, 0.55); }
  .card.on .name { opacity: 1; }
  @media (prefers-reduced-motion: reduce) { .card, .track, .rows, .hero, main { transition: none; } }
</style>
