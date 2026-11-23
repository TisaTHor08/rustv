<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../i18n';
  import ModWidgets from '../../ModWidgets.svelte';
  import { iconSrc, type DTile } from '../../tiles.svelte';
  import Weather from '../../Weather.svelte';
  import type { HomeModel } from '../model.svelte';
  import { tileFg } from '../util';
  import { fitTrack } from './trackfit';
  import { SHORTCUTS } from './vidaa.meta';

  // Style VIDAA (inspiré, non officiel) : bande de raccourcis, bannière, grandes tuiles colorées. Voir docs/OS-STYLES.md.
  let { model }: { model: HomeModel } = $props();

  const inTop = $derived(model.zone === 'top');
  const topFocus = $derived<number>(model.ui.topFocus ?? 0);
  const ROW_H = (k: string) => (k === 'continue' ? 15.6 : 15);
  const hero = $derived(model.focus.row <= 0);
  const anchor = $derived(hero ? 30.4 : 9.4);
  const shift = $derived.by(() => {
    const f = Math.max(0, model.focus.row);
    let y = 0;
    for (let i = 0; i < f && i < model.rows.length; i++) y += ROW_H(model.rows[i].key);
    return -y;
  });

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
  const ICON: Record<string, string> = {
    live: 'M21 3H3a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h5v2h8v-2h5a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2Zm0 14H3V5h18Z',
    library: 'M4 6H2v14a2 2 0 0 0 2 2h14v-2H4Zm16-4H8a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2Zm-9 12V6l7 4Z',
    search: 'M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z',
    settings: 'M19.4 13a7.6 7.6 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3h-4l-.4 2.9a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.6 7.6 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1c.5.4 1.1.7 1.7 1L11 21h4l.4-2.9c.6-.3 1.2-.6 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z',
  };
  const act = (s: string) => (s === 'live' ? model.openLive() : s === 'library' ? model.openMedia('movies') : s === 'search' ? model.openSearch() : model.openSettings());

  // Défilement horizontal : la tuile ciblée reste visible dans la rangée active.
  let main: HTMLElement | undefined = $state();
  let txs = $state<Record<string, number>>({});
  $effect(() => {
    const ri = model.focus.row;
    void model.focus.col;
    void model.rows;
    const key = model.rows[ri]?.key;
    if (!main || !key || inTop) return;
    const card = main.querySelector<HTMLElement>(`.row[data-k="${key}"] .card.on`);
    const track = card?.parentElement as HTMLElement | null | undefined;
    if (!card || !track) return;
    const cs = getComputedStyle(track.parentElement!);
    const edge = parseFloat(getComputedStyle(main).fontSize) * 1.4;
    const next = fitTrack(track, card, txs[key] ?? 0, main.clientWidth - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight), edge);
    if (next !== (txs[key] ?? 0)) txs[key] = next;
  });
</script>

<div class="vidaa">
  <div class="bgfx"></div>

  <header>
    <div class="left">
      <span class="src"><svg viewBox="0 0 24 24"><path d="M12 3 2 12h3v8h5v-6h4v6h5v-8h3Z" /></svg>{t('vidaa.home')}</span>
      <div class="chips">
        {#each SHORTCUTS as s, i}
          <button class="chip" class:focus={inTop && topFocus === i} onclick={() => act(s)} onmouseenter={() => ((model.ui.topFocus = i), model.setZone('top'))}>
            <svg viewBox="0 0 24 24"><path d={ICON[s]} /></svg><span>{t(`vidaa.${s}`)}</span>
          </button>
        {/each}
      </div>
    </div>
    <div class="right">
      {#if model.widgetsNative}
        <ModWidgets toast={model.note} at="status" provided={['status']} />
        <Weather />
        <span class="status" class:on={model.phones > 0} title={model.phonesLabel}>
          <svg viewBox="0 0 24 24"><path d="M12 21 .6 8.9a16 16 0 0 1 22.8 0Z" /></svg>
          {#if model.phones > 0}<b>{model.phones}</b>{/if}
        </span>
        <span class="clock">{model.time}</span>
      {/if}
    </div>
  </header>

  <!-- Bannière promotionnelle : suit la tuile ciblée -->
  <section class="banner" class:away={!hero}>
    {#key heroTile?.id}
      <div class="card-b" transition:fade={{ duration: 320 }} style:--a={heroTile?.color ?? '#1d6fb8'} style:--b={heroTile?.color2 ?? '#0b3a66'}>
        {#if heroTile?.icon}<img src={iconSrc(heroTile.icon)} alt="" />{:else if heroTile}<span class="big">{heroTile.name[0]}</span>{/if}
        {#if heroTile}
          <div class="info" style:color={tileFg(heroTile.dark_text)}>
            <span class="kicker">{t(`cat.${heroTile.category}`)}</span>
            <h1>{heroTile.name}</h1>
            <p>{heroTile.desc}</p>
            <span class="go">{t('vidaa.open')}</span>
          </div>
        {/if}
      </div>
    {/key}
    <div class="dots">{#each [0, 1, 2, 3, 4] as d}<i class:on={d === model.focus.col % 5}></i>{/each}</div>
  </section>

  <main bind:this={main} style:--anchor="{anchor}rem">
    <div class="rows" style:--shift="{shift}rem">
      {#each model.rows as r, ri (r.key)}
        {@const a = ri === model.focus.row}
        <div class="row" class:act={a} data-k={r.key} style:height="{ROW_H(r.key)}rem">
          {#if r.title}<h2>{r.title}</h2>{/if}
          <div class="track" style:--tx="{txs[r.key] ?? 0}px">
            {#each r.tiles as tile, ci (tile.id)}
              {@const on = a && ci === model.focus.col && !inTop}
              <button class="card" class:on onmouseenter={() => model.focusAt(ri, ci, true)} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
                <span class="face" style:--a={r.key === 'continue' ? '#2c6aa8' : tile.color} style:--b={r.key === 'continue' ? '#0f3a66' : tile.color2} style:--fg={r.key === 'continue' ? '#fff' : tileFg(tile.dark_text)}>
                  {#if r.key === 'continue' && entryOf(ci)?.poster}<img class="poster" src={iconSrc(entryOf(ci).poster ?? '')} alt="" />
                  {:else if r.key !== 'continue' && tile.icon}<img class="ico" src={iconSrc(tile.icon)} alt="" />
                  {:else}<span class="letter">{tile.name[0]}</span>{/if}
                  {#if r.key === 'continue'}<span class="bar"><i style:width="{pct(ci)}%"></i></span>{/if}
                </span>
                <span class="name">{tile.name}</span>
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </main>
</div>

<style>
  .vidaa { --pad: max(4rem, calc((100% - 150rem) / 2)); position: absolute; inset: 0; background: var(--bg); overflow: hidden; font-family: var(--font); color: #fff; }
  .bgfx { position: absolute; inset: 0; background: radial-gradient(80rem 50rem at 100% 0%, rgba(37, 194, 217, 0.34), transparent 70%), radial-gradient(70rem 50rem at 0% 100%, rgba(40, 90, 200, 0.4), transparent 70%), linear-gradient(160deg, #0a3a63 0%, #06223d 60%, #04152a 100%); }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; padding: 0; }

  header { position: absolute; z-index: 6; top: 1.8rem; left: var(--pad); right: var(--pad); display: flex; align-items: center; justify-content: space-between; gap: 2rem; }
  .left, .right { display: flex; align-items: center; gap: 1.8rem; }
  .src { display: inline-flex; align-items: center; gap: 0.7rem; font-size: 1.6rem; font-weight: 700; }
  .src svg { width: 2rem; height: 2rem; fill: currentColor; }
  .chips { display: flex; gap: 0.8rem; }
  .chip { display: inline-flex; align-items: center; gap: 0.6rem; height: 3.4rem; padding: 0 1.4rem 0 1.1rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.12); font-size: 1.2rem; font-weight: 500; transition: background 0.2s, transform 0.2s; }
  .chip svg { width: 1.6rem; height: 1.6rem; fill: currentColor; }
  .chip.focus { background: #fff; color: #0a3a63; transform: scale(1.08); box-shadow: 0 0 0 0.2rem rgba(255, 255, 255, 0.4), 0 0.6rem 1.6rem rgba(0, 0, 0, 0.4); }
  .status { display: flex; align-items: center; gap: 0.3rem; color: rgba(255, 255, 255, 0.74); }
  .status svg { width: 1.5rem; height: 1.5rem; fill: currentColor; }
  .status.on { color: #7df0b4; }
  .status b { font-size: 1.05rem; font-weight: 500; }
  .clock { font-size: 1.6rem; font-weight: 500; }

  /* bannière */
  .banner { position: absolute; z-index: 1; left: var(--pad); right: var(--pad); top: 6.6rem; height: 21rem; transition: opacity 0.35s, transform 0.45s var(--ease); }
  .banner.away { opacity: 0; transform: translateY(-2rem); }
  .card-b { position: absolute; inset: 0; border-radius: 1.6rem; overflow: hidden; background: radial-gradient(40rem 24rem at 80% 40%, var(--a), transparent 80%), linear-gradient(120deg, var(--b), var(--a)); box-shadow: 0 1rem 2.4rem rgba(0, 0, 0, 0.35); }
  .card-b::before { content: ''; position: absolute; inset: 0; background: linear-gradient(90deg, rgba(0, 0, 0, 0.45), transparent 60%); }
  .card-b img { position: absolute; right: 12%; top: 50%; width: 11rem; height: 11rem; transform: translateY(-50%); object-fit: contain; }
  .card-b .big { position: absolute; right: 9%; top: -1rem; font-size: 24rem; font-weight: 700; line-height: 1; opacity: 0.2; }
  .info { position: absolute; left: 3rem; top: 2.6rem; width: min(46rem, 60%); }
  .kicker { font-size: 1.05rem; font-weight: 500; letter-spacing: 0.12em; text-transform: uppercase; opacity: 0.85; }
  h1 { margin: 0.4rem 0 0.4rem; font-size: 3.4rem; font-weight: 700; line-height: 1.1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .info p { margin: 0 0 1.4rem; font-size: 1.3rem; opacity: 0.88; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .go { display: inline-flex; align-items: center; height: 3.4rem; padding: 0 2rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.92); color: #0a3a63; font-size: 1.25rem; font-weight: 700; }
  .dots { position: absolute; bottom: 1rem; left: 50%; transform: translateX(-50%); display: flex; gap: 0.6rem; z-index: 2; }
  .dots i { width: 0.7rem; height: 0.7rem; border-radius: 50%; background: rgba(255, 255, 255, 0.4); }
  .dots i.on { background: #fff; }

  /* rangées */
  main { position: absolute; z-index: 2; left: 0; right: 0; top: var(--anchor); bottom: 0; overflow: hidden; transition: top 0.45s var(--ease); }
  .rows { transform: translateY(var(--shift)); transition: transform 0.45s var(--ease); }
  .row { padding-left: var(--pad); padding-right: var(--pad); box-sizing: border-box; }
  .row h2 { margin: 0 0 0.4rem; font-size: 1.6rem; font-weight: 700; }
  .track { position: relative; display: flex; gap: 1.6rem; width: max-content; transform: translateX(var(--tx)); transition: transform 0.4s var(--ease); padding: 1rem 1rem 1rem; margin-left: -1rem; }
  .card { flex: none; width: 13.6rem; text-align: left; transition: transform 0.22s var(--ease); position: relative; }
  .face { position: relative; display: block; width: 100%; aspect-ratio: 16 / 9; border-radius: var(--r); overflow: hidden; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg, #fff); box-shadow: 0 0.4rem 1rem rgba(0, 0, 0, 0.35); transition: box-shadow 0.22s; }
  .letter { position: absolute; inset: 0; display: grid; place-items: center; font-size: 3.6rem; font-weight: 700; }
  .ico { position: absolute; left: 50%; top: 50%; width: 4rem; height: 4rem; transform: translate(-50%, -50%); object-fit: contain; }
  .poster { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 0.3rem; background: rgba(255, 255, 255, 0.3); }
  .bar i { display: block; height: 100%; background: #25c2d9; }
  .name { display: block; margin-top: 0.7rem; text-align: center; font-size: 1.15rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: rgba(255, 255, 255, 0.74); }
  .card.on { transform: scale(var(--focus-scale)); z-index: 3; }
  .card.on .face { box-shadow: 0 0 0 var(--focus-ring) #fff, 0 0 2.2rem rgba(37, 194, 217, 0.65), 0 1rem 2rem rgba(0, 0, 0, 0.5); }
  .card.on .name { color: #fff; font-weight: 700; }
</style>
