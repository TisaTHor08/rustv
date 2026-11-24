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
  import { RAIL } from './vidaa.meta';

  // Style VIDAA (inspiré, non officiel) : rail d'icônes à gauche, bannière encadrée, applis carrées, rangées d'affiches.
  // Voir docs/OS-STYLES.md.
  let { model }: { model: HomeModel } = $props();

  const inRail = $derived(model.zone === 'rail');
  const inBanner = $derived(model.zone === 'banner');
  const railFocus = $derived<number>(model.ui.railFocus ?? 1);
  const ROW_H = (k: string) => (k === 'apps' ? 11.2 : 16.4);
  const hero = $derived(model.focus.row <= 0);
  const anchor = $derived(hero ? 29.2 : 3.4);
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
  // Icônes en contour (trait), comme le rail de VIDAA.
  const ICON: Record<string, string> = {
    profile: 'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm-8 8c0-4 3.6-6 8-6s8 2 8 6',
    search: 'M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13Zm5-1.5L21 21',
    notif: 'M6 17V11a6 6 0 0 1 12 0v6l1.5 2h-15ZM10 21h4',
    input: 'M3 12h11m-4-4 4 4-4 4M14 4h5a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-5',
    settings: 'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm7.4-2a7.6 7.6 0 0 0 0-2l2-1.5-2-3.4-2.4 1a7.4 7.4 0 0 0-1.7-1L15 3h-4l-.4 2.7a7.4 7.4 0 0 0-1.7 1l-2.4-1-2 3.4 2 1.5a7.6 7.6 0 0 0 0 2l-2 1.5 2 3.4 2.4-1c.5.4 1.1.7 1.7 1L11 21h4l.4-2.7c.6-.3 1.2-.6 1.7-1l2.4 1 2-3.4Z',
  };
  const railAct = (s: string) => (s === 'search' ? model.openSearch() : s === 'notif' ? model.note(t('vidaa.nonotif')) : s === 'input' ? model.openMedia('movies') : model.openSettings());

  // Défilement horizontal : la tuile ciblée reste visible dans la rangée active.
  let main: HTMLElement | undefined = $state();
  let txs = $state<Record<string, number>>({});
  $effect(() => {
    const ri = model.focus.row;
    void model.focus.col;
    void model.rows;
    const key = model.rows[ri]?.key;
    if (!main || !key || inRail || inBanner) return;
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
  <nav class="rail">
    {#each RAIL as s, i}
      <button class="ri" class:focus={inRail && railFocus === i} aria-label={t(`vidaa.${s}`)} onclick={() => railAct(s)} onmouseenter={() => ((model.ui.railFocus = i), model.setZone('rail'))}>
        <svg viewBox="0 0 24 24"><path d={ICON[s]} /></svg>
      </button>
    {/each}
  </nav>

  {#if model.widgetsNative}
    <div class="status">
      <ModWidgets toast={model.note} at="status" provided={['status']} />
      <Weather />
      {#if model.phones > 0}<span class="ph" title={model.phonesLabel}>{model.phones}</span>{/if}
      <span class="clock">{model.time}</span>
    </div>
  {/if}

  <!-- Bannière : suit la tuile ciblée, cadre blanc quand elle a le focus -->
  <section class="banner" class:away={!hero} class:focus={inBanner}>
    {#key heroTile?.id}
      <div class="card-b" in:fade={{ duration: 240 }} style:--a={heroTile?.color ?? '#1d6fb8'} style:--b={heroTile?.color2 ?? '#0b3a66'}>
        {#if heroTile?.banner}<HeroShot src={iconSrc(heroTile.banner)} />{/if}
        {#if heroTile?.banner}{:else if heroTile?.icon}<img src={iconSrc(heroTile.icon)} alt="" />{:else if heroTile}<span class="big">{heroTile.name[0]}</span>{/if}
        {#if heroTile}
          <div class="info" style:color={tileFg(heroTile.dark_text)}>
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
          {#if r.title && r.key !== 'apps'}<h2>{r.title}</h2>{/if}
          <div class="track" style:--tx="{txs[r.key] ?? 0}px">
            {#each r.tiles as tile, ci (tile.id)}
              {@const on = a && ci === model.focus.col && !inRail && !inBanner}
              {@const sq = r.key === 'apps'}
              <button class="card" class:sq class:on onmouseenter={() => model.focusAt(ri, ci, true)} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
                <span class="face" style:--a={r.key === 'continue' ? '#2c6aa8' : tile.color} style:--b={r.key === 'continue' ? '#0f3a66' : tile.color2} style:--fg={r.key === 'continue' ? '#fff' : tileFg(tile.dark_text)}>
                  {#if r.key === 'continue' && entryOf(ci)?.poster}<img class="poster" src={iconSrc(entryOf(ci).poster ?? '')} alt="" />
                  {:else if sq && tile.icon}<img class="ico" src={iconSrc(tile.icon)} alt="" />
                  {:else}<span class="letter">{tile.name[0]}</span>{/if}
                  {#if !sq}<span class="cap">{tile.name}</span>{/if}
                  {#if r.key === 'continue'}<span class="bar"><i style:width="{pct(ci)}%"></i></span>{/if}
                </span>
                {#if sq}<span class="name">{tile.name}</span>{/if}
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </main>
</div>

<style>
  .vidaa { --pad: 5.6rem; --padr: 4.5rem; position: absolute; inset: 0; background: var(--bg); overflow: hidden; font-family: var(--font); color: #fff; }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; padding: 0; }

  /* rail d'icônes (profil, recherche, notifications, entrées, réglages) */
  .rail { position: absolute; z-index: 6; left: 0; top: 0; bottom: 0; width: 4rem; display: flex; flex-direction: column; justify-content: center; gap: 3.4rem; align-items: center; background: rgba(0, 0, 0, 0.18); }
  .ri { display: grid; place-items: center; width: 3rem; height: 3rem; border-radius: 50%; color: rgba(255, 255, 255, 0.85); transition: background 0.15s, color 0.15s, transform 0.15s; }
  .ri svg { width: 1.7rem; height: 1.7rem; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
  .ri.focus { background: #fff; color: #10121a; transform: scale(1.15); }

  .status { position: absolute; z-index: 6; top: 0.8rem; right: var(--padr); display: flex; align-items: center; gap: 1.4rem; font-size: 1.25rem; color: rgba(255, 255, 255, 0.8); }
  .ph { padding: 0.1rem 0.7rem; border-radius: 99rem; background: rgba(47, 181, 200, 0.25); color: #7fe3f0; font-weight: 500; }
  .clock { font-size: 1.4rem; font-weight: 500; color: #fff; }

  /* bannière : forme oblique sombre à gauche (titre), couleur de la tuile à droite */
  .banner { position: absolute; z-index: 1; left: var(--pad); right: var(--padr); top: 3.4rem; height: 22.4rem; box-sizing: border-box; border: 0.22rem solid transparent; transition: opacity 0.35s, transform 0.45s var(--ease), border-color 0.15s; }
  .banner.focus { border-color: #fff; }
  .banner.away { opacity: 0; transform: translateY(-2rem); }
  .card-b { position: absolute; inset: 0; overflow: hidden; background: radial-gradient(50rem 30rem at 80% 30%, var(--a), transparent 80%), linear-gradient(120deg, var(--b), var(--a)); }
  .card-b::before { content: ''; position: absolute; inset: 0; background: linear-gradient(180deg, #2a2c38, #0d0e15); clip-path: polygon(0 0, 38% 0, 49% 100%, 0 100%); }
  .card-b img { position: absolute; right: 14%; top: 50%; width: 10rem; height: 10rem; transform: translateY(-50%); object-fit: contain; }
  .card-b .big { position: absolute; right: 12%; top: -1rem; font-size: 24rem; font-weight: 700; line-height: 1; opacity: 0.2; }
  .info { position: absolute; left: 3.4rem; top: 50%; transform: translateY(-50%); width: 28%; color: #fff !important; }
  h1 { margin: 0 0 0.6rem; font-size: 3.2rem; font-weight: 300; letter-spacing: 0.06em; line-height: 1.1; text-transform: uppercase; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .info p { margin: 0 0 1.3rem; font-size: 1.15rem; opacity: 0.8; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .go { display: inline-flex; align-items: center; height: 2.8rem; padding: 0 1.6rem; border: 0.12rem solid rgba(255, 255, 255, 0.8); border-radius: 0.3rem; font-size: 1.1rem; font-weight: 500; }
  .dots { position: absolute; bottom: 1.2rem; left: 50%; transform: translateX(-50%); display: flex; gap: 0.9rem; z-index: 2; }
  .dots i { width: 0.7rem; height: 0.7rem; border-radius: 50%; background: rgba(255, 255, 255, 0.35); }
  .dots i.on { background: #fff; }

  /* rangées */
  main { position: absolute; z-index: 2; left: 0; right: 0; top: var(--anchor); bottom: 0; overflow: hidden; transition: top 0.45s var(--ease); }
  .rows { transform: translateY(var(--shift)); transition: transform 0.45s var(--ease); }
  .row { padding-left: var(--pad); padding-right: var(--padr); box-sizing: border-box; }
  .row h2 { margin: 0 0 0.6rem; font-size: 1.7rem; font-weight: 400; color: rgba(255, 255, 255, 0.92); }
  .track { position: relative; display: flex; gap: 1.3rem; width: max-content; transform: translateX(var(--tx)); transition: transform 0.4s var(--ease); padding: 0.8rem 1rem; margin-left: -1rem; }
  .card { flex: none; width: 20.4rem; text-align: left; transition: transform 0.2s var(--ease); position: relative; }
  .card.sq { width: 6.7rem; }
  .row:has(.sq) .track { gap: 0.8rem; }
  .face { position: relative; display: block; width: 100%; aspect-ratio: 16 / 9; border-radius: var(--r); overflow: hidden; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg, #fff); }
  .sq .face { aspect-ratio: 1; }
  .letter { position: absolute; inset: 0; display: grid; place-items: center; font-size: 4rem; font-weight: 700; opacity: 0.9; }
  .sq .letter { font-size: 3rem; }
  .ico { position: absolute; left: 50%; top: 50%; width: 4rem; height: 4rem; transform: translate(-50%, -50%); object-fit: contain; }
  .poster { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .cap { position: absolute; left: 0; right: 0; bottom: 0; padding: 1.6rem 0.9rem 0.6rem; background: linear-gradient(transparent, rgba(0, 0, 0, 0.7)); font-size: 1.1rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 0.3rem; background: rgba(255, 255, 255, 0.3); }
  .bar i { display: block; height: 100%; background: #2fb5c8; }
  .name { position: absolute; left: -1rem; right: -1rem; top: calc(100% + 0.5rem); text-align: center; font-size: 1.05rem; opacity: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card.on { transform: scale(var(--focus-scale)); z-index: 3; }
  .card.on .face { box-shadow: 0 0 0 var(--focus-ring) #fff, 0 0.8rem 1.8rem rgba(0, 0, 0, 0.5); }
  .card.on .name { opacity: 1; }
</style>
