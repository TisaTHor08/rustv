<script lang="ts">
  import { t } from '../../i18n';
  import ModWidgets from '../../ModWidgets.svelte';
  import { iconSrc } from '../../tiles.svelte';
  import Weather from '../../Weather.svelte';
  import type { HomeModel } from '../model.svelte';
  import { reducedMotion, tileFg } from '../util';
  import { previewsOf, SYS, TOP } from './tizen.meta';

  // Style Tizen (inspiré, non officiel) : héros ambiant, aperçus, barre de lancement en bas. Voir docs/OS-STYLES.md.
  let { model }: { model: HomeModel } = $props();

  const zone = $derived(model.zone);
  const tiles = $derived(model.rows[0]?.tiles ?? []);
  const f = $derived(model.focused);
  const previews = $derived(previewsOf(model));
  const sysFocus = $derived<number>(model.ui.sysFocus ?? 0);
  const pvFocus = $derived<number>(model.ui.pvFocus ?? 0);
  const topFocus = $derived<number>(model.ui.topFocus ?? 1);

  const ICONS: Record<string, string> = {
    settings: 'M19.4 13a7.7 7.7 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3.3h-4l-.4 2.6a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.7 7.7 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1a7.4 7.4 0 0 0 1.7 1l.4 2.6h4l.4-2.6a7.4 7.4 0 0 0 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z',
    source: 'M21 11H7.8l3.6-3.6L10 6l-6 6 6 6 1.4-1.4L7.8 13H21Z',
    search: 'M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z',
    ambient: 'M12 3a9 9 0 1 0 9 9 7 7 0 0 1-9-9Z',
    menu: 'M4 6h16v2H4Zm0 5h16v2H4Zm0 5h16v2H4Z',
    bell: 'M12 22a2 2 0 0 0 2-2h-4a2 2 0 0 0 2 2Zm6-6v-5c0-3.1-1.6-5.6-4.5-6.3V4a1.5 1.5 0 0 0-3 0v.7C7.6 5.4 6 7.9 6 11v5l-2 2v1h16v-1Z',
    profile: 'M12 12a4 4 0 1 0-4-4 4 4 0 0 0 4 4Zm0 2c-2.7 0-8 1.3-8 4v2h16v-2c0-2.7-5.3-4-8-4Z',
  };

  // La tuile ciblée reste dans la barre : défilement centré (sans mouvement si l'utilisateur le demande).
  let track: HTMLElement | undefined = $state();
  $effect(() => {
    void model.focus.col;
    if (!track || zone === 'sys') return;
    anchorTo(track, track.querySelector('.app.on, .app.sel'), 14);
  });
  let pvTrack: HTMLElement | undefined = $state();
  $effect(() => {
    void pvFocus;
    if (pvTrack && zone === 'preview') pvTrack.querySelector('.pv.on')?.scrollIntoView({ block: 'nearest', inline: 'center', behavior: reducedMotion() ? 'auto' : 'smooth' });
  });


  // Ancrage horizontal : la carte ciblée reste à une position fixe (sauf en début de liste), défilement doux.
  function anchorTo(box: HTMLElement, el: Element | null, rems: number) {
    if (!el) return;
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
    const left = (el as HTMLElement).offsetLeft - rems * rem;
    box.scrollTo({ left: Math.max(0, left), behavior: reducedMotion() ? 'auto' : 'smooth' });
  }

  const hover = (z: string, key: string, i: number) => () => {
    model.ui[key] = i;
    model.setZone(z);
  };
</script>

<div class="tz">
  <!-- Héros : contenu ambiant plein cadre de l'application ciblée -->
  <div class="hero" style:--a={f?.color ?? '#233a6b'} style:--b={f?.color2 ?? '#0d1630'}>
    {#key f?.id}
      <div class="art">
        {#if f?.icon}<img src={iconSrc(f.icon)} alt="" />{:else if f}<span class="big">{f.name[0]}</span>{/if}
      </div>
      {#if f}
        <div class="info">
          <span class="kicker">{t(`cat.${f.category}`)}</span>
          <h1>{f.name}</h1>
          <p>{f.desc}</p>
        </div>
      {/if}
    {/key}
  </div>

  <!-- Haut : cloche, profil, état (très discret) -->
  <header>
    {#if model.widgetsNative}
      <ModWidgets toast={model.note} at="status" provided={['status', 'bar']} />
      <Weather />
      <span class="chip" class:on={model.phones > 0} title={model.phonesLabel}><i></i>{model.phones > 0 ? model.phones : ''}</span>
      <span class="clock">{model.time}</span>
    {/if}
    {#each TOP as id, i}
      <button class="top" class:focus={zone === 'top' && topFocus === i} aria-label={t(`os.tizen.${id}`)} onmouseenter={hover('top', 'topFocus', i)} onclick={() => (i === 0 ? model.note(t('os.tizen.nonotif')) : model.openSettings())}>
        <svg viewBox="0 0 24 24"><path d={ICONS[id]} /></svg>
      </button>
    {/each}
  </header>

  <!-- Aperçus de l'application ciblée -->
  {#if previews.length}
    <section class="previews" class:act={zone === 'preview'}>
      {#key f?.id}
        <div class="pvhead">{previews[0].resume ? t('os.tizen.resume') : t('os.tizen.related')}</div>
        <div class="pvtrack" bind:this={pvTrack}>
          {#each previews as p, i (p.id)}
            <button class="pv" class:on={zone === 'preview' && pvFocus === i} onmouseenter={hover('preview', 'pvFocus', i)} onclick={() => (p.resume ? model.resume(p.resume) : model.open(p.tile))}>
              <span class="pface" style:--a={p.tile.color} style:--b={p.tile.color2} style:--fg={tileFg(p.tile.dark_text)}>
                {#if p.poster}<img class="poster" src={iconSrc(p.poster)} alt="" />{:else if p.tile.icon}<img class="ico" src={iconSrc(p.tile.icon)} alt="" />{:else}<b>{p.title[0]}</b>{/if}
                {#if p.resume}<span class="bar"><i style:width="{p.pct ?? 0}%"></i></span>{/if}
              </span>
              <span class="pname">{p.title}</span>
            </button>
          {/each}
        </div>
      {/key}
    </section>
  {/if}

  <!-- Barre de lancement -->
  <nav class="launcher">
    <div class="sys">
      {#each SYS as id, i}
        <button class="rb" class:on={zone === 'sys' && sysFocus === i} aria-label={t(`os.tizen.${id}`)} onmouseenter={hover('sys', 'sysFocus', i)} onclick={() => ((model.ui.sysFocus = i), model.setZone('sys'), model.activate())}>
          <svg viewBox="0 0 24 24"><path d={ICONS[id]} /></svg>
        </button>
      {/each}
    </div>
    <div class="sep"></div>
    <div class="apps" bind:this={track}>
      {#each tiles as tile, i (tile.id)}
        {@const on = zone === 'content' && i === model.focus.col}
        <button class="app" class:on class:sel={zone !== 'content' && i === model.focus.col} onmouseenter={() => (model.setZone('content'), model.focusAt(0, i, true))} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
          <span class="face" style:--a={tile.color} style:--b={tile.color2} style:--fg={tileFg(tile.dark_text)}>
            {#if tile.icon}<img src={iconSrc(tile.icon)} alt="" />{:else}<b>{tile.name[0]}</b>{/if}
          </span>
          <span class="name">{tile.name}</span>
        </button>
      {/each}
    </div>
  </nav>

  <!-- Mode ambiance : écran sombre avec horloge, n'importe quelle touche le quitte -->
  {#if zone === 'ambient'}
    <div class="ambient" role="presentation" onclick={() => model.setZone('content')}>
      <strong>{model.time}</strong>
      <span>{model.date}</span>
      <small>{t('os.tizen.hint')}</small>
    </div>
  {/if}
</div>

<style>
  .tz { position: absolute; inset: 0; overflow: hidden; font-family: var(--font); color: #f2f4f8; background: #070913; }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; padding: 0; }
  svg { fill: currentColor; }

  /* héros ambiant : couleurs de la tuile ciblée sur fond bleu nuit */
  .hero { position: absolute; inset: 0; background: radial-gradient(90rem 46rem at 72% 8%, rgba(43, 95, 208, 0.38), transparent 70%), radial-gradient(60rem 36rem at 10% 0%, rgba(86, 70, 190, 0.22), transparent 70%), linear-gradient(180deg, #0c1224 0%, #080b16 60%, #05070d 100%); }
  .art { position: absolute; inset: 0; animation: fadein 0.6s var(--ease) both; background: radial-gradient(56rem 30rem at 74% 24%, color-mix(in srgb, var(--a) 62%, transparent), transparent 72%), linear-gradient(135deg, color-mix(in srgb, var(--b) 55%, transparent), transparent 70%); }
  .art::after { content: ''; position: absolute; inset: 0; background: linear-gradient(90deg, rgba(7, 9, 19, 0.7) 6%, transparent 62%), linear-gradient(0deg, #070913 6%, transparent 55%); }
  .art .big { position: absolute; right: 10%; top: 4rem; font-size: 26rem; font-weight: 800; line-height: 1; opacity: 0.14; }
  .art img { position: absolute; right: 16%; top: 7rem; width: 16rem; height: 16rem; object-fit: contain; opacity: 0.92; filter: drop-shadow(0 1rem 2rem rgba(0, 0, 0, 0.5)); }
  .info { position: absolute; z-index: 2; left: 6rem; top: 11rem; width: min(60rem, 56%); animation: rise 0.55s var(--ease) both; }
  .kicker { font-size: 1.15rem; font-weight: 600; letter-spacing: 0.12em; text-transform: uppercase; color: rgba(255, 255, 255, 0.6); }
  h1 { margin: 0.6rem 0 0.6rem; font-size: 4.6rem; font-weight: 700; line-height: 1.04; letter-spacing: -0.02em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .info p { margin: 0; font-size: 1.5rem; font-weight: 400; color: rgba(255, 255, 255, 0.72); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  header { position: absolute; z-index: 5; top: 1.8rem; right: 3rem; display: flex; align-items: center; gap: 1.2rem; font-size: 1.3rem; }
  .clock { font-size: 1.4rem; font-weight: 500; margin-right: 0.4rem; }
  .chip { display: inline-flex; align-items: center; gap: 0.4rem; color: rgba(255, 255, 255, 0.6); font-size: 1.1rem; }
  .chip i { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: #7a8296; }
  .chip.on i { background: #3ddc84; }
  .top { width: 3.4rem; height: 3.4rem; border-radius: 50%; display: grid; place-items: center; color: rgba(255, 255, 255, 0.82); transition: background 0.2s, transform 0.25s var(--ease); }
  .top svg { width: 1.8rem; height: 1.8rem; }
  .top.focus { background: #fff; color: #10131c; transform: scale(1.14); }

  /* aperçus au-dessus de la barre */
  .previews { position: absolute; z-index: 3; left: 0; right: 0; bottom: 13.6rem; opacity: 0.86; transition: opacity 0.3s; }
  .previews.act { opacity: 1; }
  .pvhead { margin: 0 0 0.4rem 6rem; font-size: 1.3rem; font-weight: 600; color: rgba(255, 255, 255, 0.78); animation: rise 0.4s var(--ease) both; }
  .pvtrack { display: flex; gap: 1.3rem; padding: 0.8rem 3rem 1.4rem 6rem; overflow-x: hidden; scrollbar-width: none; animation: rise 0.45s var(--ease) both; }
  .pv { flex: none; width: 13rem; text-align: left; transition: transform 0.3s cubic-bezier(0.3, 1.4, 0.5, 1); }
  .pface { position: relative; display: grid; place-items: center; aspect-ratio: 16 / 9; border-radius: var(--r); overflow: hidden; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg); box-shadow: 0 0.5rem 1.2rem rgba(0, 0, 0, 0.4); }
  .pface b { font-size: 3rem; font-weight: 700; opacity: 0.4; }
  .pface .ico { width: 3.4rem; height: 3.4rem; object-fit: contain; }
  .pface .poster { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .pface .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 0.3rem; background: rgba(255, 255, 255, 0.3); }
  .pface .bar i { display: block; height: 100%; background: #6aa2ff; }
  .pname { display: block; margin-top: 0.5rem; font-size: 1.05rem; color: rgba(255, 255, 255, 0.8); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pv.on { transform: scale(var(--focus-scale)); position: relative; z-index: 2; }
  .pv.on .pface { box-shadow: 0 0 0 var(--focus-ring) #fff, 0 1rem 2rem rgba(0, 0, 0, 0.55); }
  .pv.on .pname { color: #fff; font-weight: 600; }

  /* barre de lancement : bas de l'écran, fond sombre translucide */
  .launcher { position: absolute; z-index: 4; left: 0; right: 0; bottom: 0; height: 13rem; display: flex; align-items: flex-start; padding: 2.2rem 0 0 3rem; background: linear-gradient(0deg, rgba(6, 8, 15, 0.94) 0%, rgba(8, 11, 20, 0.86) 62%, rgba(8, 11, 20, 0.5) 100%); backdrop-filter: blur(1.6rem); border-top: 0.06rem solid rgba(255, 255, 255, 0.07); }
  .sys { flex: none; display: flex; gap: 0.9rem; align-items: center; height: 5.9rem; }
  .rb { width: 4.4rem; height: 4.4rem; border-radius: 50%; display: grid; place-items: center; background: rgba(255, 255, 255, 0.12); color: rgba(255, 255, 255, 0.9); transition: transform 0.3s cubic-bezier(0.3, 1.4, 0.5, 1), background 0.2s; }
  .rb svg { width: 2.2rem; height: 2.2rem; }
  .rb.on { transform: scale(var(--focus-scale)); background: #fff; color: #10131c; box-shadow: 0 0.6rem 1.6rem rgba(0, 0, 0, 0.5); }
  .sep { flex: none; width: 0.1rem; height: 4rem; margin: 0.95rem 1.6rem; background: rgba(255, 255, 255, 0.16); }
  .apps { mask-image: linear-gradient(90deg, transparent 0, #000 1.4rem); position: relative; flex: 1; min-width: 0; display: flex; gap: 1.5rem; padding: 0.6rem 3rem 2rem 1rem; margin-top: -0.6rem; overflow-x: hidden; scrollbar-width: none; }
  .app { flex: none; width: 10.4rem; text-align: center; transition: transform 0.3s cubic-bezier(0.3, 1.4, 0.5, 1); }
  .face { position: relative; display: grid; place-items: center; width: 100%; aspect-ratio: 16 / 9; border-radius: var(--r); overflow: hidden; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg); box-shadow: 0 0.3rem 0.8rem rgba(0, 0, 0, 0.35); }
  .face b { font-size: 2.6rem; font-weight: 700; }
  .face img { width: 2.9rem; height: 2.9rem; object-fit: contain; }
  .name { display: block; margin-top: 0.8rem; font-size: 1.15rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; opacity: 0; transition: opacity 0.2s; }
  .app.on { transform: scale(var(--focus-scale)); position: relative; z-index: 2; }
  .app.on .face { box-shadow: 0 0 0 var(--focus-ring) #fff, 0 1rem 2rem rgba(0, 0, 0, 0.55); }
  .app.on .name { opacity: 1; font-weight: 600; }
  .app.sel .face { box-shadow: 0 0 0 0.12rem rgba(255, 255, 255, 0.45); }

  .ambient { position: absolute; inset: 0; z-index: 20; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 0.6rem; background: radial-gradient(70rem 40rem at 50% 40%, #0f1a38 0%, #04050a 70%); animation: fadein 0.6s both; }
  .ambient strong { font-size: 12rem; font-weight: 200; letter-spacing: -0.03em; }
  .ambient span { font-size: 2.2rem; color: rgba(255, 255, 255, 0.7); text-transform: capitalize; }
  .ambient small { margin-top: 5rem; font-size: 1.1rem; color: rgba(255, 255, 255, 0.35); }

  @keyframes fadein { from { opacity: 0; } to { opacity: 1; } }
  @keyframes rise { from { opacity: 0; transform: translateY(1rem); } to { opacity: 1; transform: none; } }
</style>
