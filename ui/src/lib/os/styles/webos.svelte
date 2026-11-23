<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../i18n';
  import ModWidgets from '../../ModWidgets.svelte';
  import { iconSrc } from '../../tiles.svelte';
  import Weather from '../../Weather.svelte';
  import type { HomeModel } from '../model.svelte';
  import { reducedMotion, tileFg } from '../util';
  import { isAppsRow, promosOf, SYS, TOP } from './webos.meta';

  // Style webOS (accueil LG 2024-2025, inspiré, non officiel) : colonne d'icônes, bannière, cartes de raccourcis, applications, contenus.
  let { model }: { model: HomeModel } = $props();

  const zone = $derived(model.zone);
  const rows = $derived(model.rows);
  const promos = $derived(promosOf(model));
  const bi = $derived(promos.length ? Math.min(model.ui.bannerIdx ?? 0, promos.length - 1) : 0);
  const promo = $derived(promos[bi]);
  const sysFocus = $derived<number>(model.ui.sysFocus ?? 0);
  const topFocus = $derived<number>(model.ui.topFocus ?? 2);

  const ICONS: Record<string, string> = {
    profile: 'M12 12a4 4 0 1 0-4-4 4 4 0 0 0 4 4Zm0 2c-2.7 0-8 1.3-8 4v2h16v-2c0-2.7-5.3-4-8-4Z',
    alerts: 'M12 22a2 2 0 0 0 2-2h-4a2 2 0 0 0 2 2Zm6-6v-5c0-3.1-1.6-5.6-4.5-6.3V4a1.5 1.5 0 0 0-3 0v.7C7.6 5.4 6 7.9 6 11v5l-2 2v1h16v-1Z',
    settings: 'M19.4 13a7.7 7.7 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3.3h-4l-.4 2.6a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.7 7.7 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1a7.4 7.4 0 0 0 1.7 1l.4 2.6h4l.4-2.6a7.4 7.4 0 0 0 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z',
    search: 'M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z',
  };

  // Rotation automatique de la bannière (8 s), arrêtée quand elle a le focus ou si l'utilisateur limite les mouvements.
  $effect(() => {
    const n = promos.length;
    if (n < 2 || zone === 'banner' || reducedMotion()) return;
    const id = setInterval(() => (model.ui.bannerIdx = ((model.ui.bannerIdx ?? 0) + 1) % n), 8000);
    return () => clearInterval(id);
  });

  // Défilement vertical : à partir de la 2e rangée la bannière et les cartes sortent de l'écran.
  let scroller: HTMLElement | undefined = $state();
  $effect(() => {
    const r = model.focus.row;
    void rows.length;
    if (!scroller) return;
    const el = scroller.querySelector<HTMLElement>(`[data-r="${r}"]`);
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
    const top = zone === 'content' && r > 0 && el ? el.offsetTop - 4 * rem : 0;
    scroller.scrollTo({ top, behavior: reducedMotion() ? 'auto' : 'smooth' });
  });
  // Défilement horizontal de la rangée ciblée (ancrage fixe sauf en début de rangée).
  $effect(() => {
    void model.focus.col;
    void zone;
    const track = scroller?.querySelector<HTMLElement>(`[data-r="${model.focus.row}"] .track`);
    const el = track?.querySelector<HTMLElement>('.it.on, .it.sel');
    if (!track || !el) return;
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
    track.scrollTo({ left: Math.max(0, el.offsetLeft - 14 * rem), behavior: reducedMotion() ? 'auto' : 'smooth' });
  });

  const hover = (z: string, key: string, i: number) => () => {
    model.ui[key] = i;
    model.setZone(z);
  };
  const pct = (tile: any) => (tile.raw?.duration > 0 ? Math.min(100, (tile.raw.position / tile.raw.duration) * 100) : 0);
</script>

<div class="wo">
  <!-- Colonne d'icônes à gauche -->
  <nav class="col">
    {#each TOP as id, i}
      <button class="ic" class:focus={zone === 'top' && topFocus === i} aria-label={t(`os.webos.${id}`)} onmouseenter={hover('top', 'topFocus', i)} onclick={() => ((model.ui.topFocus = i), model.setZone('top'), model.activate())}>
        {#if id === 'profile'}<span class="av">R</span>{:else}<svg viewBox="0 0 24 24"><path d={ICONS[id]} /></svg>{/if}
      </button>
    {/each}
  </nav>

  {#if model.widgetsNative}
    <header>
      <ModWidgets toast={model.note} at="status" provided={['status']} />
      <Weather />
      <span class="chip" class:on={model.phones > 0} title={model.phonesLabel}><i></i>{model.phones > 0 ? model.phones : ''}</span>
      <span class="clock">{model.time}</span>
    </header>
  {/if}

  <div class="scroller" bind:this={scroller}>
    <!-- Bannière -->
    <section class="banner" class:act={zone === 'banner'}>
      {#key promo?.id}
        <div class="art" transition:fade={{ duration: 500 }} style:--a={promo?.color ?? '#2a3f8f'} style:--b={promo?.color2 ?? '#10183a'}>
          {#if promo?.poster}<img class="poster" src={iconSrc(promo.poster)} alt="" />{:else if promo?.tile?.icon}<img class="ico" src={iconSrc(promo.tile.icon)} alt="" />{:else if promo}<span class="big">{promo.title[0]}</span>{/if}
        </div>
        {#if promo}
          <div class="info" transition:fade={{ duration: 500 }}>
            <h1>{promo.title}</h1>
            {#if promo.desc}<p>{promo.desc}</p>{/if}
            {#if promo.pct !== undefined}<span class="prog"><i style:width="{promo.pct}%"></i></span>{/if}
            <button class="cta" class:focus={zone === 'banner'} onmouseenter={() => model.setZone('banner')} onclick={() => (promo.resume ? model.resume(promo.resume) : promo.tile && model.open(promo.tile))}>{promo.resume ? t('os.webos.watch') : t('os.webos.open')}</button>
          </div>
        {/if}
      {/key}
      {#if promos.length > 1}
        <div class="dots">{#each promos as p, i (p.id)}<i class:on={i === bi}></i>{/each}</div>
      {/if}
    </section>

    <!-- Cartes de raccourcis colorées -->
    <section class="qcards">
      {#each SYS as id, i}
        <button class="q" data-k={id} class:on={zone === 'sys' && sysFocus === i} onmouseenter={hover('sys', 'sysFocus', i)} onclick={() => ((model.ui.sysFocus = i), model.setZone('sys'), model.activate())}>{t(`os.webos.${id}`)}</button>
      {/each}
    </section>

    {#each rows as row, r (row.key)}
      <section class="row" class:apps={isAppsRow(row.key)} data-r={r}>
        {#if row.title}<h2>{row.title}</h2>{/if}
        <div class="track">
          {#each row.tiles as tile, i (tile.id)}
            {@const on = zone === 'content' && r === model.focus.row && i === model.focus.col}
            <button class="it" class:on class:sel={zone !== 'content' && r === model.focus.row && i === model.focus.col} onmouseenter={() => (model.setZone('content'), model.focusAt(r, i, true))} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
              <span class="face" style:--a={tile.color} style:--b={tile.color2} style:--fg={tileFg(tile.dark_text)}>
                {#if !isAppsRow(row.key) && tile.id.startsWith('continue:') && (tile.raw as any)?.poster}<img class="poster" src={iconSrc((tile.raw as any).poster)} alt="" />{:else if tile.icon}<img class="ico" src={iconSrc(tile.icon)} alt="" />{:else}<b>{tile.name[0]}</b>{/if}
                {#if tile.id.startsWith('continue:')}<span class="bar"><i style:width="{pct(tile)}%"></i></span>{/if}
              </span>
              <span class="name">{tile.name}</span>
            </button>
          {/each}
        </div>
      </section>
    {/each}
    <div class="pad"></div>
  </div>
</div>

<style>
  .wo { position: absolute; inset: 0; overflow: hidden; font-family: var(--font); color: #f4f7fb; background: #050608; }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; padding: 0; }
  svg { fill: currentColor; }

  /* colonne d'icônes à gauche */
  .col { position: absolute; z-index: 6; left: 1.8rem; top: 2.2rem; display: flex; flex-direction: column; gap: 1.2rem; }
  .ic { width: 3.8rem; height: 3.8rem; border-radius: 50%; display: grid; place-items: center; color: rgba(255, 255, 255, 0.92); transition: background 0.2s, color 0.2s, transform 0.25s var(--ease); }
  .ic svg { width: 2.2rem; height: 2.2rem; }
  .av { width: 3rem; height: 3rem; border-radius: 50%; display: grid; place-items: center; background: #3a4cf0; font-size: 1.4rem; font-weight: 600; text-transform: uppercase; }
  .ic.focus { background: #fff; color: #11172b; transform: scale(1.12); }
  .ic.focus .av { background: #11172b; color: #fff; }

  header { position: absolute; z-index: 5; top: 1.8rem; right: 3.4rem; display: flex; align-items: center; gap: 1.1rem; text-shadow: 0 0.1rem 0.6rem rgba(0, 0, 0, 0.6); }
  .clock { margin-left: 0.6rem; font-size: 1.6rem; font-weight: 500; }
  .chip { display: inline-flex; align-items: center; gap: 0.4rem; font-size: 1.1rem; color: rgba(255, 255, 255, 0.7); }
  .chip i { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: #8590aa; }
  .chip.on i { background: #3ddc84; }

  .scroller { position: absolute; z-index: 1; left: 0; right: 0; top: 0; bottom: 0; overflow: hidden; }

  /* bannière */
  .banner { position: relative; height: 22.5rem; }
  .art { position: absolute; inset: 0; background: radial-gradient(52rem 26rem at 40% 60%, color-mix(in srgb, var(--a) 62%, transparent), transparent 72%), linear-gradient(135deg, color-mix(in srgb, var(--b) 60%, transparent), transparent 72%); }
  .art::after { content: ''; position: absolute; inset: 0; background: linear-gradient(0deg, #050608 2%, transparent 55%); }
  .art .big { position: absolute; left: 30%; top: 2rem; font-size: 20rem; font-weight: 700; line-height: 1; opacity: 0.18; }
  .art .ico { position: absolute; left: 34%; top: 4rem; width: 13rem; height: 13rem; object-fit: contain; opacity: 0.92; filter: drop-shadow(0 1rem 2rem rgba(0, 0, 0, 0.45)); }
  .art .poster { position: absolute; left: 30%; top: 2.6rem; height: 19rem; width: 13.5rem; object-fit: cover; border-radius: var(--r); box-shadow: 0 1.4rem 3rem rgba(0, 0, 0, 0.5); }
  .info { position: absolute; z-index: 2; right: 5rem; top: 6.4rem; width: min(44rem, 46%); text-align: right; display: flex; flex-direction: column; align-items: flex-end; }
  h1 { margin: 0 0 0.6rem; font-size: 2.8rem; font-weight: 600; line-height: 1.15; max-width: 100%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; text-shadow: 0 0.2rem 1rem rgba(0, 0, 0, 0.6); }
  .info p { margin: 0 0 1rem; font-size: 1.3rem; color: rgba(255, 255, 255, 0.78); max-width: 100%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .prog { display: block; width: 20rem; height: 0.3rem; margin: 0 0 1rem; border-radius: 1rem; background: rgba(255, 255, 255, 0.25); overflow: hidden; }
  .prog i { display: block; height: 100%; background: #fff; }
  .cta { height: 3.4rem; padding: 0 3.6rem; border-radius: 0.7rem; font-size: 1.25rem; font-weight: 500; background: rgba(0, 0, 0, 0.25); box-shadow: inset 0 0 0 0.16rem rgba(255, 255, 255, 0.9); transition: background 0.2s, color 0.2s, transform 0.25s var(--ease); }
  .cta.focus { background: #fff; color: #11172b; transform: scale(1.06); }
  .dots { position: absolute; z-index: 2; right: 5rem; bottom: 1.2rem; display: flex; gap: 0.6rem; }
  .dots i { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: rgba(255, 255, 255, 0.35); }
  .dots i.on { width: 2rem; border-radius: 1rem; background: #fff; }

  /* cartes de raccourcis : rectangles de couleur unie, comme les « cartes rapides » */
  .qcards { display: flex; gap: 1.4rem; padding: 0.6rem 3rem 1rem 9rem; }
  .q { flex: none; width: 21rem; height: 4.8rem; border-radius: 0.8rem; font-size: 1.5rem; font-weight: 500; color: #fff; text-shadow: 0 0.1rem 0.4rem rgba(0, 0, 0, 0.25); transition: transform 0.25s var(--ease), box-shadow 0.2s; }
  .q[data-k='live'] { background: #8cc63f; }
  .q[data-k='movies'] { background: #2b8cf0; }
  .q[data-k='music'] { background: #9440e0; }
  .q[data-k='photos'] { background: #e8776e; }
  .q.on { transform: scale(var(--focus-scale)); box-shadow: 0 0 0 var(--focus-ring) #fff, 0 0.8rem 1.8rem rgba(0, 0, 0, 0.55); position: relative; z-index: 2; }

  /* rangées */
  .row { margin-bottom: 0.4rem; }
  h2 { margin: 0.4rem 0 0 9rem; font-size: 1.3rem; font-weight: 500; color: rgba(255, 255, 255, 0.88); }
  .track { display: flex; gap: 1.2rem; padding: 0.8rem 3rem 1.2rem 9rem; overflow-x: hidden; scrollbar-width: none; }
  .it { flex: none; width: 17rem; text-align: left; transition: transform 0.28s var(--ease); }
  .face { position: relative; display: grid; place-items: center; aspect-ratio: 16 / 9; border-radius: 0.8rem; overflow: hidden; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg); box-shadow: 0 0.3rem 0.9rem rgba(0, 0, 0, 0.4); }
  .face b { font-size: 3rem; font-weight: 700; opacity: 0.5; }
  .face .ico { width: 3.6rem; height: 3.6rem; object-fit: contain; }
  .face .poster { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .face .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 0.3rem; background: rgba(255, 255, 255, 0.3); }
  .face .bar i { display: block; height: 100%; background: #fff; }
  .name { display: block; margin-top: 0.5rem; font-size: 1.1rem; color: rgba(255, 255, 255, 0.75); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .it.on { transform: scale(var(--focus-scale)); position: relative; z-index: 2; }
  .it.on .face { box-shadow: 0 0 0 var(--focus-ring) #fff, 0 1rem 2rem rgba(0, 0, 0, 0.55); }
  .it.on .name { color: #fff; font-weight: 600; }
  .it.sel .face { box-shadow: 0 0 0 0.1rem rgba(255, 255, 255, 0.4); }

  /* rangée d'applications : carrés arrondis, nom sous le carré seulement quand il est ciblé */
  .apps .it { width: 7.6rem; text-align: center; }
  .apps .face { aspect-ratio: 1; border-radius: 1rem; }
  .apps .face b { opacity: 1; font-size: 2.8rem; }
  .apps .face .ico { width: 4rem; height: 4rem; }
  .apps .name { opacity: 0; font-size: 1rem; text-align: center; transition: opacity 0.2s; }
  .apps .it.on .name { opacity: 1; }
  .apps .track { gap: 1rem; }
  .pad { height: 40rem; }
</style>
