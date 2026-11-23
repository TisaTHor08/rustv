<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../i18n';
  import ModWidgets from '../../ModWidgets.svelte';
  import { iconSrc, type DTile } from '../../tiles.svelte';
  import Weather from '../../Weather.svelte';
  import type { HomeModel } from '../model.svelte';
  import { tileFg } from '../util';
  import { navEntries, type FireEntry } from './fire.meta';

  // Style Fire TV (inspiré, non officiel) : héros plein cadre en haut, barre d'icônes + raccourcis d'applis dessous, rangées de cartes
  // 16:9 serrées, titres de rangée en capitales espacées. Proportions relevées sur des captures de l'accueil (voir docs/OS-STYLES.md).
  let { model }: { model: HomeModel } = $props();

  const tab = $derived<string>(model.ui.tab ?? 'home');
  const list = $derived<FireEntry[]>(navEntries(model));
  const navFocus = $derived<number>(Math.min(list.length - 1, model.ui.navFocus ?? Math.max(0, list.findIndex((e) => e.id === tab))));
  const inNav = $derived(model.zone === 'nav');
  const hasHero = $derived(tab === 'home');

  // Hauteurs en rem (écran de 96 x 54 rem) : héros 48 %, barre 7, rangée = titre 1,6 + carte 9,45 + espace 1,35.
  const HERO_H = 25.8;
  const NAV_H = 8.2;
  const TOP = 2.6; // marge au-dessus de la barre quand il n'y a pas de héros
  const ROW_H = (title: string) => (title ? 13.2 : 11.6);
  const STEP = 17.2; // carte 16,8 + espace 0,4
  const head = $derived((hasHero ? HERO_H : TOP) + NAV_H);
  const shift = $derived.by(() => {
    const r = model.focus.row;
    if (inNav || r <= 0) return 0;
    let y = head;
    for (let i = 0; i < r && i < model.rows.length; i++) y += ROW_H(model.rows[i].title);
    return -Math.max(0, y - 6.5);
  });

  let W = $state(0);
  let rem = $state(16);
  $effect(() => {
    const f = () => (rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16);
    f();
    addEventListener('resize', f);
    return () => removeEventListener('resize', f);
  });
  const usable = $derived(Math.min(W / rem, 150) - 10.4);
  const txOf = (act: boolean) => {
    const vis = Math.max(2, Math.floor((usable + 0.4) / STEP));
    return -Math.max(0, (act ? model.focus.col : 0) - (vis - 2)) * STEP;
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
    const e = list[i];
    if (e.id === 'search') model.openSearch();
    else if (e.id === 'live') model.openLive();
    else if (e.id === 'settings') model.openSettings();
    else if (e.id === 'profile') model.openQuick();
    else if (e.id === 'app' && e.tile) model.open(e.tile);
    else ((model.ui.tab = e.id), model.focusAt(0, 0, true), model.setZone('content'));
  };
  const hover = (i: number) => ((model.ui.navFocus = i), model.setZone('nav'));
  const label = (e: FireEntry) => (e.id === 'app' ? (e.tile?.name ?? '') : t(`os.fire.${e.id}`));
  const icons: Record<string, string> = {
    search: 'M10.5 3.5a7 7 0 1 0 0 14 7 7 0 0 0 0-14Zm5.2 12.2L21 21',
    home: 'M3 11.5 12 3.5l9 8M5.5 10v10.5h5v-6h3v6h5V10',
    live: 'M3.5 8.5h17v11h-17zM8 3.5l4 5 4-5',
    library: 'M6.5 3.5h11v17L12 16.6l-5.5 3.9z',
    apps: 'M3.5 3.5h7v7h-7zM13.5 3.5h7v7h-7zM3.5 13.5h7v7h-7zM17 13.5v7M13.5 17h7',
  };
</script>

<div class="fire" bind:clientWidth={W}>
  <div class="page" style:--shift="{shift}rem">
    <!-- Héros -->
    {#if hasHero}
      <section class="hero" style:height="{HERO_H}rem">
        {#key heroTile?.id}
          <div class="art" transition:fade={{ duration: 300 }} style:--a={heroTile?.color ?? '#233044'} style:--b={heroTile?.color2 ?? '#171717'}>
            <div class="stage">
              {#if heroTile?.icon}<img src={iconSrc(heroTile.icon)} alt="" />{:else if heroTile}<span class="big">{heroTile.name[0]}</span>{/if}
            </div>
          </div>
          {#if heroTile}
            <div class="info" transition:fade={{ duration: 300 }}>
              <span class="kicker">{t(`cat.${heroTile.category}`)}</span>
              <h1>{heroTile.name}</h1>
              <p>{heroTile.desc}</p>
              <span class="learn">{t('os.fire.open')}</span>
            </div>
          {/if}
        {/key}
        <div class="dots">{#each [0, 1, 2, 3, 4] as d}<i class:on={d === Math.min(4, model.focus.col % 5)} class:far={d === 4 && d !== model.focus.col % 5}></i>{/each}</div>
      </section>
    {/if}

    <!-- Barre : profil, icônes, raccourcis d'applis, toutes les applis, réglages -->
    <header style:height="{NAV_H}rem" style:margin-top={hasHero ? '0' : `${TOP}rem`}>
      <nav>
        {#each list as e, i}
          {#if e.id === 'app' && e.tile}
            <button class="shortcut" class:focus={inNav && navFocus === i} style:--a={e.tile.color} style:--b={e.tile.color2} style:--fg={tileFg(e.tile.dark_text)} onclick={() => press(i)} onmouseenter={() => hover(i)} aria-label={label(e)}>
              {#if e.tile.icon}<img src={iconSrc(e.tile.icon)} alt="" />{:else}<span>{e.tile.name}</span>{/if}
            </button>
          {:else if e.id === 'apps' || e.id === 'settings'}
            <button class="sq" class:round={e.id === 'settings'} class:focus={inNav && navFocus === i} onclick={() => press(i)} onmouseenter={() => hover(i)} aria-label={label(e)}>
              {#if e.id === 'apps'}<svg viewBox="0 0 24 24" class="line"><path d={icons.apps} /></svg>{:else}<svg viewBox="0 0 24 24"><path d="M19.4 13a7.5 7.5 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3.2h-4L10.7 5.9a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.5 7.5 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1a7.4 7.4 0 0 0 1.7 1l.3 2.7h4l.3-2.7a7.4 7.4 0 0 0 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z" transform="translate(-1 0)" /></svg>{/if}
            </button>
          {:else}
            <button class="item" class:focus={inNav && navFocus === i} class:sel={tab === e.id} class:av={e.id === 'profile'} onclick={() => press(i)} onmouseenter={() => hover(i)} aria-label={label(e)}>
              {#if e.id === 'profile'}
                <span class="avatar"><svg viewBox="0 0 24 24"><circle cx="12" cy="9" r="4.2" /><path d="M3.5 22c0-4.6 3.8-7.6 8.5-7.6s8.5 3 8.5 7.6z" /></svg></span>
              {:else}
                <svg viewBox="0 0 24 24" class="line"><path d={icons[e.id]} /></svg>
              {/if}
              {#if inNav && navFocus === i}<em>{label(e)}</em>{/if}
            </button>
          {/if}
          {#if e.id === 'library'}<span class="gap"></span>{/if}
        {/each}
      </nav>
    </header>

    <!-- Rangées -->
    <main>
      {#each model.rows as r, ri (r.key)}
        {@const act = ri === model.focus.row}
        <div class="row" class:act style:height="{ROW_H(r.title)}rem">
          {#if r.title}<h2>{r.title}</h2>{/if}
          <div class="track" style:--tx="{txOf(act)}rem">
            {#each r.tiles as tile, ci (tile.id)}
              {@const on = act && ci === model.focus.col && !inNav}
              {#if r.key === 'continue'}
                <button class="card" class:on onmouseenter={() => model.focusAt(ri, ci, true)} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
                  <span class="face" style:--a="#3a3f45" style:--b="#1d2024">
                    {#if entryOf(ci)?.poster}<img class="cover" src={iconSrc(entryOf(ci).poster ?? '')} alt="" />{:else}<span class="mark">{tile.name[0]}</span>{/if}
                    <span class="cap"><b>{tile.name}</b><small>{tile.desc.split('·')[0]}</small></span>
                    <span class="bar"><i style:width="{pct(ci)}%"></i></span>
                  </span>
                </button>
              {:else}
                <button class="card" class:on onmouseenter={() => model.focusAt(ri, ci, true)} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
                  <span class="face" style:--a={tile.color} style:--b={tile.color2} style:--fg={tileFg(tile.dark_text)}>
                    {#if tile.icon}<img class="ico" src={iconSrc(tile.icon)} alt="" />{:else}<span class="letter">{tile.name[0]}</span>{/if}
                    <span class="cap"><b>{tile.name}</b></span>
                  </span>
                </button>
              {/if}
            {/each}
          </div>
        </div>
      {/each}
    </main>
  </div>

  {#if model.widgetsNative && hasHero}
    <div class="status" style:opacity={model.focus.row > 0 ? 0 : 1}>
      <ModWidgets toast={model.note} at="status" provided={['status']} />
      <Weather />
      {#if model.phones > 0}<span class="phones" title={model.phonesLabel}><svg viewBox="0 0 24 24"><path d="M17 1H7a2 2 0 0 0-2 2v18a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V3a2 2 0 0 0-2-2Zm0 18H7V5h10Z" /></svg>{model.phones}</span>{/if}
      <span class="clock">{model.time}</span>
    </div>
  {/if}
</div>

<style>
  .fire { --pad: max(5.2rem, calc((100% - 140rem) / 2)); position: absolute; inset: 0; background: var(--bg); overflow: hidden; font-family: var(--font); color: var(--text); }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; padding: 0; }
  .page { position: absolute; left: 0; right: 0; top: 0; transform: translateY(var(--shift)); transition: transform 0.4s var(--ease); }

  /* héros : coupe nette en bas, comme sur l'appareil */
  .hero { position: relative; overflow: hidden; background: #101010; }
  .art { position: absolute; inset: 0; background: radial-gradient(56rem 30rem at 74% 36%, var(--a), transparent 72%), linear-gradient(120deg, var(--b), transparent 70%), #121212; }
  .art::after { content: ''; position: absolute; inset: 0; background: linear-gradient(90deg, rgba(8, 10, 12, 0.88) 6%, rgba(8, 10, 12, 0.35) 46%, transparent 78%), linear-gradient(0deg, rgba(0, 0, 0, 0.35), transparent 40%); }
  .stage { position: absolute; inset: 0 var(--pad); }
  .big { position: absolute; right: 6rem; top: 50%; transform: translateY(-50%); font-size: 24rem; font-weight: 700; line-height: 1; opacity: 0.2; }
  .stage img { position: absolute; right: 12rem; top: 50%; transform: translateY(-50%); width: 14rem; height: 14rem; object-fit: contain; opacity: 0.92; }
  .info { position: absolute; z-index: 2; left: var(--pad); top: 4.2rem; width: min(50rem, 60%); }
  .kicker { display: block; font-size: 0.95rem; font-weight: 500; letter-spacing: 0.32em; text-transform: uppercase; color: rgba(255, 255, 255, 0.82); }
  h1 { margin: 1rem 0 0.8rem; font-size: 4.2rem; font-weight: 500; letter-spacing: 0.06em; line-height: 1.05; text-transform: uppercase; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .info p { margin: 0 0 1.4rem; font-size: 1.25rem; color: rgba(255, 255, 255, 0.78); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .learn { display: inline-flex; align-items: center; height: 3.2rem; padding: 0 2.2rem; border-radius: 1.6rem; background: rgba(110, 110, 110, 0.72); font-size: 1.3rem; font-weight: 500; color: #e8e8e8; }
  .dots { position: absolute; z-index: 2; left: var(--pad); bottom: 1.6rem; display: flex; align-items: center; gap: 0.45rem; }
  .dots i { width: 0.7rem; height: 0.7rem; border-radius: 50%; background: rgba(255, 255, 255, 0.5); transition: width 0.25s var(--ease), background 0.25s; }
  .dots i.on { width: 1.6rem; border-radius: 0.4rem; background: #fff; }
  .dots i.far { width: 0.45rem; height: 0.45rem; }

  /* barre */
  header { position: relative; z-index: 3; padding: 0 var(--pad); box-sizing: border-box; }
  nav { display: flex; align-items: flex-start; gap: 0; padding-top: 1.4rem; }
  .gap { flex: 1; }
  .item { position: relative; flex: none; width: 4.8rem; height: 4rem; display: grid; place-items: center; color: rgba(255, 255, 255, 0.78); }
  .item::before { content: ''; position: absolute; left: 0; top: -0.4rem; width: 4.8rem; height: 4.8rem; border-radius: 50%; background: #242424; opacity: 0; transition: opacity 0.15s; }
  .item::after { content: ''; position: absolute; left: -0.9rem; top: -1.3rem; width: 6.6rem; height: 5.4rem; background: radial-gradient(closest-side, rgba(255, 255, 255, 0.2), transparent); opacity: 0; transition: opacity 0.2s; }
  .item.sel::before, .item.focus::before { opacity: 1; }
  .item.focus::after { opacity: 1; }
  .item.sel { color: #fff; }
  .item.focus { color: #fff; }
  .item > * { position: relative; z-index: 1; }
  .item svg.line { width: 2.2rem; height: 2.2rem; fill: none; stroke: currentColor; stroke-width: 1.7; stroke-linecap: round; stroke-linejoin: round; }
  .item em { position: absolute; top: 4.9rem; left: 50%; transform: translateX(-50%); font-style: normal; font-size: 1.35rem; font-weight: 500; color: #fff; white-space: nowrap; }
  .avatar { display: block; width: 4rem; height: 4rem; border-radius: 50%; background: linear-gradient(160deg, #3fb6e8, #1b6f9a); overflow: hidden; }
  .avatar svg { width: 100%; height: 100%; fill: rgba(255, 255, 255, 0.85); }
  .item.av { width: 5.2rem; justify-items: start; }
  .item.av.focus .avatar { box-shadow: 0 0 0 0.2rem #fff; }
  .item.av::before { display: none; }
  .shortcut, .sq { position: relative; flex: none; display: grid; place-items: center; height: 4rem; margin-left: 0.4rem; transition: transform 0.18s var(--ease); }
  .shortcut { width: 7.2rem; border-radius: 0.2rem; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg, #fff); }
  .shortcut img { width: 2.4rem; height: 2.4rem; object-fit: contain; }
  .shortcut span { padding: 0 0.4rem; font-size: 1rem; font-weight: 700; line-height: 1.1; text-align: center; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; }
  .sq { width: 4rem; border-radius: 0.2rem; background: #5a5a5a; color: #f2f2f2; margin-left: 0.8rem; }
  .sq.round { border-radius: 50%; background: #6a6a6a; margin-left: 0.4rem; }
  .sq svg { width: 2.2rem; height: 2.2rem; fill: currentColor; }
  .sq svg.line { fill: none; stroke: currentColor; stroke-width: 1.7; stroke-linecap: round; stroke-linejoin: round; }
  .shortcut.focus, .sq.focus { transform: scale(1.1); z-index: 2; box-shadow: 0 0 0 0.2rem #fff, 0 0.6rem 1.4rem rgba(0, 0, 0, 0.5); }

  /* rangées : titres en capitales espacées, cartes serrées */
  main { position: relative; z-index: 2; }
  .row { padding-left: var(--pad); box-sizing: border-box; }
  .row h2 { margin: 0; height: 1.6rem; font-size: 1.3rem; font-weight: 700; letter-spacing: 0.22em; text-transform: uppercase; color: rgba(255, 255, 255, 0.9); line-height: 1.2; }
  .track { display: flex; gap: 0.4rem; width: max-content; transform: translateX(var(--tx)); transition: transform 0.4s var(--ease); padding: 0.5rem 0.6rem 0.6rem 0.6rem; margin: 0.3rem 0 0 -0.6rem; }
  .card { flex: none; width: 16.8rem; text-align: left; transition: transform 0.2s var(--ease); }
  .face { position: relative; display: block; width: 100%; aspect-ratio: 16 / 9; border-radius: var(--r); overflow: hidden; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg, #fff); transition: box-shadow 0.15s; }
  .letter { position: absolute; inset: 0; display: grid; place-items: center; font-size: 3.6rem; font-weight: 700; padding-bottom: 1rem; }
  .ico { position: absolute; left: 50%; top: 44%; width: 4.4rem; height: 4.4rem; transform: translate(-50%, -50%); object-fit: contain; }
  .cover { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .mark { position: absolute; inset: 0; display: grid; place-items: center; font-size: 5rem; font-weight: 700; opacity: 0.22; }
  .cap { position: absolute; left: 0; right: 0; bottom: 0; padding: 1.4rem 0.8rem 0.5rem; background: linear-gradient(0deg, rgba(0, 0, 0, 0.7), transparent); }
  .cap b { display: block; font-size: 1.05rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .cap small { display: block; font-size: 0.85rem; color: rgba(255, 255, 255, 0.75); }
  .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 0.3rem; background: rgba(255, 255, 255, 0.28); }
  .bar i { display: block; height: 100%; background: var(--accent); }
  .card.on { transform: scale(var(--focus-scale)); position: relative; z-index: 3; }
  .card.on .face { box-shadow: 0 0 0 var(--focus-ring) #fff, 0 0.8rem 2rem rgba(0, 0, 0, 0.6); }

  .status { position: absolute; z-index: 6; top: 1.6rem; right: var(--pad); display: flex; align-items: center; gap: 1.2rem; transition: opacity 0.3s; }
  .clock { font-size: 1.3rem; font-weight: 500; color: rgba(255, 255, 255, 0.85); }
  .phones { display: inline-flex; align-items: center; gap: 0.3rem; font-size: 1.1rem; color: #7ed39a; }
  .phones svg { width: 1.3rem; height: 1.3rem; fill: currentColor; }
  @media (prefers-reduced-motion: reduce) { .card, .track, .page, .shortcut, .sq, .dots i { transition: none; } }
</style>
