<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../i18n';
  import ModWidgets from '../../ModWidgets.svelte';
  import { iconSrc, type DTile } from '../../tiles.svelte';
  import HeroShot from '../HeroShot.svelte';
  import Weather from '../../Weather.svelte';
  import type { HomeModel } from '../model.svelte';
  import { tileFg } from '../util';
  import { fitTrack } from './trackfit';
  import { NAV } from './titan.meta';

  // Style Titan (inspiré, non officiel) : icônes en haut, héros carrousel, rangée d'applis carrées, rails. Voir docs/OS-STYLES.md.
  let { model }: { model: HomeModel } = $props();

  const tab = $derived<string>(model.ui.tab ?? 'home');
  const navFocus = $derived<number>(model.ui.navFocus ?? Math.max(0, NAV.findIndex((e) => e.id === tab)));
  const inNav = $derived(model.zone === 'nav');
  const isGrid = (k: string) => /^g\d+$/.test(k);
  const ROW_H = (k: string) => (k === 'apps' ? 13 : isGrid(k) ? 11 : 12.2);
  const hasHero = $derived(tab === 'home');
  const hero = $derived(hasHero && model.focus.row <= 0);
  // Ancrage vertical : rangée du haut sous le héros, puis rangée ciblée sous la barre d'icônes.
  const anchor = $derived(hero ? 27 : hasHero ? 9.4 : 8);
  const shift = $derived.by(() => {
    const f = Math.max(0, model.focus.row);
    const skip = hasHero ? f : Math.max(0, f - 2);
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
  const entryOf = (i: number) => model.continueEntries[i];
  const pct = (i: number) => {
    const e = entryOf(i);
    return e && e.duration > 0 ? Math.min(100, Math.round((e.position / e.duration) * 100)) : 0;
  };

  // Défilement horizontal : la carte ciblée reste visible dans la rangée active.
  let main: HTMLElement | undefined = $state();
  let txs = $state<Record<string, number>>({});
  $effect(() => {
    const ri = model.focus.row;
    void model.focus.col;
    void model.rows;
    const key = model.rows[ri]?.key;
    if (!main || !key || inNav) return;
    const card = main.querySelector<HTMLElement>(`.row[data-k="${key}"] .card.on`);
    const track = card?.parentElement as HTMLElement | null | undefined;
    if (!card || !track) return;
    const cs = getComputedStyle(track.parentElement!);
    const padL = parseFloat(cs.paddingLeft);
    const edge = parseFloat(getComputedStyle(main).fontSize) * 1.2;
    const next = fitTrack(track, card, txs[key] ?? 0, main.clientWidth - padL - parseFloat(cs.paddingRight), edge);
    if (next !== (txs[key] ?? 0)) txs[key] = next;
  });
</script>

<div class="titan">
  <div class="ambient"></div>

  {#if hasHero}
    <section class="hero" class:away={!hero}>
      {#key heroTile?.id}
        <div class="art" transition:fade={{ duration: 360 }} style:--a={heroTile?.color ?? '#2a2f5d'} style:--b={heroTile?.color2 ?? '#12142a'}>
        {#if heroTile?.banner}<HeroShot src={iconSrc(heroTile.banner)} />{/if}
          {#if heroTile?.banner}{:else if heroTile?.icon}<img src={iconSrc(heroTile.icon)} alt="" />{:else if heroTile}<span class="big">{heroTile.name[0]}</span>{/if}
        </div>
        {#if heroTile}
          <div class="info" in:fade={{ duration: 300, delay: 60 }}>
            <span class="kicker">{t(`cat.${heroTile.category}`)}</span>
            <h1>{heroTile.name}</h1>
            <p>{heroTile.desc}</p>
            <span class="watch"><svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg>{t('titan.open')}</span>
          </div>
          <div class="dots" transition:fade={{ duration: 360 }}>{#each [0, 1, 2, 3, 4, 5] as d}<i class:on={d === model.focus.col % 6}></i>{/each}</div>
        {/if}
      {/key}
    </section>
  {/if}

  <header>
    <nav>
      {#each NAV.slice(0, 5) as e, i}
        <button class="ni" class:sel={e.kind === 'tab' && tab === e.id} class:focus={inNav && navFocus === i} aria-label={t(`titan.nav.${e.id}`)} onclick={() => (e.id === 'search' ? model.openSearch() : e.id === 'live' ? model.openLive() : ((model.ui.tab = e.id), model.focusAt(0, 0, true), model.setZone('content')))} onmouseenter={() => ((model.ui.navFocus = i), model.setZone('nav'))}>
          <svg viewBox="0 0 24 24">
            {#if e.id === 'search'}<path d="M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z" />
            {:else if e.id === 'home'}<path d="M12 3 2 12h3v8h5v-6h4v6h5v-8h3Z" />
            {:else if e.id === 'apps'}<path d="M4 4h7v7H4Zm9 0h7v7h-7ZM4 13h7v7H4Zm9 0h7v7h-7Z" />
            {:else if e.id === 'live'}<path d="M21 3H3a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h5v2h8v-2h5a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2Zm0 14H3V5h18Z" />
            {:else}<path d="M4 6H2v14a2 2 0 0 0 2 2h14v-2H4Zm16-4H8a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2Zm-9 12V6l7 4Z" />{/if}
          </svg>
          {#if inNav && navFocus === i}<span class="lbl">{t(`titan.nav.${e.id}`)}</span>{/if}
        </button>
      {/each}
    </nav>
    <div class="right">
      {#if model.widgetsNative}
        <ModWidgets toast={model.note} at="status" provided={['status']} />
        <Weather />
        <span class="status" class:on={model.phones > 0} title={model.phonesLabel}>
          <svg viewBox="0 0 24 24"><path d="M17 1H7a2 2 0 0 0-2 2v18a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V3a2 2 0 0 0-2-2Zm0 18H7V5h10Z" /></svg>
          {#if model.phones > 0}<b>{model.phones}</b>{/if}
        </span>
      {/if}
      <button class="ni" class:focus={inNav && navFocus === 5} aria-label={t('titan.nav.settings')} onclick={() => model.openSettings()} onmouseenter={() => ((model.ui.navFocus = 5), model.setZone('nav'))}>
        <svg viewBox="0 0 24 24"><path d="M19.4 13a7.6 7.6 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3h-4l-.4 2.9a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.6 7.6 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1c.5.4 1.1.7 1.7 1L11 21h4l.4-2.9c.6-.3 1.2-.6 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z" /></svg>
        {#if inNav && navFocus === 5}<span class="lbl">{t('titan.nav.settings')}</span>{/if}
      </button>
      {#if model.widgetsNative}<span class="clock">{model.time}</span>{/if}
    </div>
  </header>

  <main bind:this={main} style:--anchor="{anchor}rem">
    <div class="rows" style:--shift="{shift}rem">
      {#each model.rows as r, ri (r.key)}
        {@const act = ri === model.focus.row}
        <div class="row" class:act data-k={r.key} style:height="{ROW_H(r.key)}rem">
          {#if r.title}<h2>{r.title}</h2>{/if}
          <div class="track" style:--tx="{txs[r.key] ?? 0}px">
            {#each r.tiles as tile, ci (tile.id)}
              {@const on = act && ci === model.focus.col && !inNav}
              {#if r.key === 'apps' || isGrid(r.key)}
                <button class="card sq" class:big={isGrid(r.key)} class:on onmouseenter={() => model.focusAt(ri, ci, true)} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
                  <span class="face" style:--a={tile.color} style:--b={tile.color2} style:--fg={tileFg(tile.dark_text)}>
                    {#if tile.icon}<img class="ico" src={iconSrc(tile.icon)} alt="" />{:else}<span class="letter">{tile.name[0]}</span>{/if}
                  </span>
                  <span class="cap">{tile.name}</span>
                </button>
              {:else}
                <button class="card rail" class:on onmouseenter={() => model.focusAt(ri, ci, true)} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
                  <span class="face" style:--a={r.key === 'continue' ? '#3a3f66' : tile.color} style:--b={r.key === 'continue' ? '#161a33' : tile.color2} style:--fg={r.key === 'continue' ? '#fff' : tileFg(tile.dark_text)}>
                    {#if r.key === 'continue' && entryOf(ci)?.poster}<img class="poster" src={iconSrc(entryOf(ci).poster ?? '')} alt="" />
                    {:else if r.key !== 'continue' && tile.icon}<img class="ico" src={iconSrc(tile.icon)} alt="" />
                    {:else}<span class="letter">{tile.name[0]}</span>{/if}
                    <span class="shade"></span>
                    <span class="nm">{tile.name}</span>
                    {#if r.key === 'continue'}<span class="bar"><i style:width="{pct(ci)}%"></i></span>{/if}
                  </span>
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
  .titan { --pad: max(5rem, calc((100% - 140rem) / 2)); position: absolute; inset: 0; background: var(--bg); overflow: hidden; font-family: var(--font); color: var(--text); }
  .ambient { position: absolute; inset: 0; background: radial-gradient(70rem 40rem at 85% 0%, rgba(111, 123, 255, 0.22), transparent 70%), radial-gradient(50rem 36rem at 0% 100%, rgba(150, 70, 200, 0.18), transparent 70%); }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; padding: 0; }

  /* barre d'icônes */
  header { position: absolute; z-index: 6; top: 1.6rem; left: var(--pad); right: var(--pad); display: flex; align-items: flex-start; justify-content: space-between; }
  nav, .right { display: flex; align-items: flex-start; gap: 0.6rem; }
  .right { align-items: center; }
  .ni { position: relative; width: 4.4rem; height: 3.4rem; display: grid; place-items: center; color: color-mix(in srgb, var(--text) 82%, transparent); border-bottom: 0.18rem solid transparent; border-radius: 0.2rem; transition: color 0.2s, border-color 0.2s; }
  .ni svg { width: 2rem; height: 2rem; fill: currentColor; }
  .ni.sel { color: var(--text); border-bottom-color: var(--text); }
  .ni.focus { color: var(--text); border-bottom-color: #4d8dff; background: linear-gradient(0deg, rgba(77, 141, 255, 0.28), transparent 80%); }
  .lbl { position: absolute; top: 3.9rem; left: 50%; transform: translateX(-50%); white-space: nowrap; font-size: 1.05rem; font-weight: 500; }
  .status { display: flex; align-items: center; gap: 0.3rem; color: var(--muted); }
  .status svg { width: 1.5rem; height: 1.5rem; fill: currentColor; }
  .status.on { color: #7be0a8; }
  .status b { font-size: 1.05rem; font-weight: 500; }
  .clock { font-size: 1.5rem; font-weight: 400; margin-left: 0.6rem; line-height: 3.4rem; opacity: 0.9; }

  /* héros */
  .hero { position: absolute; left: 0; right: 0; top: 0; height: 40rem; z-index: 1; transition: opacity 0.4s; }
  .hero.away { opacity: 0; }
  .art { position: absolute; inset: 0; -webkit-mask-image: linear-gradient(180deg, #000 55%, transparent 98%); mask-image: linear-gradient(180deg, #000 55%, transparent 98%); background: radial-gradient(52rem 32rem at 74% 36%, var(--a), transparent 74%), linear-gradient(120deg, var(--b), transparent 70%); }
  .art::after { content: ''; position: absolute; inset: 0; background: linear-gradient(90deg, rgba(var(--bg-rgb), 0.92) 8%, rgba(var(--bg-rgb), 0.5) 44%, transparent 80%), linear-gradient(180deg, rgba(var(--bg-rgb), 0.7), transparent 18%); }
  .art .big { position: absolute; right: max(10rem, calc((100% - 140rem) / 2 + 10rem)); top: 7rem; font-size: 24rem; font-weight: 700; line-height: 1; opacity: 0.14; }
  .art img { position: absolute; right: max(14rem, calc((100% - 140rem) / 2 + 14rem)); top: 9rem; width: 15rem; height: 15rem; object-fit: contain; opacity: 0.9; }
  .info { position: absolute; z-index: 2; left: var(--pad); top: 10.4rem; width: min(50rem, 60%); }
  .kicker { display: inline-block; padding: 0.2rem 0.8rem; border-radius: 99rem; font-size: 1rem; font-weight: 500; letter-spacing: 0.06em; background: var(--glass); color: var(--muted); }
  h1 { margin: 0.8rem 0 0.4rem; font-size: 3.4rem; font-weight: 500; line-height: 1.12; letter-spacing: -0.01em; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .info p { margin: 0 0 1.6rem; font-size: 1.3rem; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .watch { display: inline-flex; align-items: center; gap: 0.6rem; height: 3.6rem; padding: 0 2.2rem 0 1.6rem; border-radius: 99rem; background: rgba(150, 150, 170, 0.42); backdrop-filter: blur(0.6rem); font-size: 1.35rem; font-weight: 500; }
  .watch svg { width: 1.6rem; height: 1.6rem; fill: currentColor; }
  .dots { position: absolute; z-index: 2; right: var(--pad); top: 27.4rem; display: flex; gap: 0.5rem; align-items: center; }
  .dots i { width: 0.5rem; height: 0.5rem; border-radius: 99rem; background: var(--glass-strong); transition: width 0.3s; }
  .dots i.on { width: 1.8rem; background: var(--text); }

  /* rangées */
  main { position: absolute; z-index: 2; left: 0; right: 0; top: var(--anchor); bottom: 0; overflow: hidden; transition: top 0.45s var(--ease); }
  .rows { transform: translateY(var(--shift)); transition: transform 0.45s var(--ease); padding-top: 0.6rem; }
  .row { padding-left: var(--pad); padding-right: var(--pad); box-sizing: border-box; }
  .row h2 { margin: 0 0 0.9rem; font-size: 1.3rem; font-weight: 500; color: var(--text); opacity: 0.92; }
  .track { position: relative; display: flex; gap: 1.1rem; width: max-content; transform: translateX(var(--tx)); transition: transform 0.4s var(--ease); padding: 0.6rem 0.6rem 1.6rem; margin-left: -0.6rem; }
  .card { flex: none; padding: 0; text-align: left; transition: transform 0.22s var(--ease); position: relative; }
  .sq { width: 6.2rem; }
  .sq.big { width: 7rem; }
  .rail { width: 12.8rem; }
  .face { position: relative; display: block; width: 100%; border-radius: var(--r); overflow: hidden; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg, #fff); transition: box-shadow 0.22s; }
  .sq .face { aspect-ratio: 1; border-radius: 1rem; }
  .rail .face { aspect-ratio: 16 / 9; }
  .letter { position: absolute; inset: 0; display: grid; place-items: center; font-size: 2.6rem; font-weight: 600; }
  .rail .letter { font-size: 3.4rem; opacity: 0.85; }
  .ico { position: absolute; left: 50%; top: 50%; width: 55%; height: 55%; transform: translate(-50%, -50%); object-fit: contain; }
  .rail .ico { width: 3.4rem; height: 3.4rem; top: 42%; }
  .poster { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .shade { position: absolute; left: 0; right: 0; bottom: 0; height: 55%; background: linear-gradient(0deg, rgba(0, 0, 0, 0.68), transparent); }
  .nm { position: absolute; left: 0.7rem; right: 0.7rem; bottom: 0.5rem; font-size: 1rem; font-weight: 500; color: #fff; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 0.25rem; background: rgba(255, 255, 255, 0.3); }
  .bar i { display: block; height: 100%; background: #fff; }
  .cap { position: absolute; left: -1rem; right: -1rem; top: calc(100% + 0.5rem); text-align: center; font-size: 0.95rem; font-weight: 500; opacity: 0; transition: opacity 0.2s; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card.on { transform: scale(var(--focus-scale)); z-index: 3; }
  .card.on .face { box-shadow: 0 0 0 var(--focus-ring) var(--ring), 0 0 2rem rgba(120, 140, 255, 0.55), 0 1rem 2rem rgba(0, 0, 0, 0.5); }
  .card.on .cap { opacity: 1; }
</style>
