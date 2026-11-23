<script lang="ts">
  import { t } from '../../i18n';
  import ModWidgets from '../../ModWidgets.svelte';
  import { iconSrc } from '../../tiles.svelte';
  import Weather from '../../Weather.svelte';
  import type { HomeModel } from '../model.svelte';
  import { reducedMotion, tileFg } from '../util';
  import Ambient from './tizen.screens/ambient.svelte';
  import { isAppsRow, RAIL, RAIL_BOTTOM, TABS } from './tizen.meta';

  // Style Tizen (inspiré de One UI Tizen 2024-2025, non officiel) : rail à gauche, héros, onglets, cartes 16:9, rangée d'applications.
  let { model }: { model: HomeModel } = $props();

  const zone = $derived(model.zone);
  const rows = $derived(model.rows);
  const f = $derived(model.focused);
  const railFocus = $derived<number>(model.ui.railFocus ?? 5);
  const tabFocus = $derived<number>(model.ui.tabFocus ?? 0);
  const tab = $derived<string>(model.ui.tab === 'apps' ? 'apps' : 'foryou');

  const ICONS: Record<string, string> = {
    profile: 'M12 12a4 4 0 1 0-4-4 4 4 0 0 0 4 4Zm0 2c-2.7 0-8 1.3-8 4v2h16v-2c0-2.7-5.3-4-8-4Z',
    search: 'M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z',
    ambient: 'M12 2C7 2 3 6 3 11c0 4 2.5 7 6 8.3V22h6v-2.7c3.5-1.3 6-4.3 6-8.3 0-5-4-9-9-9Zm0 4a5 5 0 0 1 5 5c0 2.4-1.7 4.4-4 4.9V12h-2v3.9C8.7 15.4 7 13.4 7 11a5 5 0 0 1 5-5Z',
    media: 'M4 8h4V4H4v4Zm6 12h4v-4h-4v4Zm-6 0h4v-4H4v4Zm0-6h4v-4H4v4Zm6 0h4v-4h-4v4Zm6-10v4h4V4h-4Zm-6 4h4V4h-4v4Zm6 6h4v-4h-4v4Zm0 6h4v-4h-4v4Z',
    live: 'M21 6h-7.6l3.1-3.1L15.6 2 12 5.6 8.4 2l-.9.9L10.6 6H3a1 1 0 0 0-1 1v13a1 1 0 0 0 1 1h18a1 1 0 0 0 1-1V7a1 1 0 0 0-1-1Zm-1 13H4V8h16Z',
    home: 'M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8Z',
    settings: 'M19.4 13a7.7 7.7 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3.3h-4l-.4 2.6a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.7 7.7 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1a7.4 7.4 0 0 0 1.7 1l.4 2.6h4l.4-2.6a7.4 7.4 0 0 0 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z',
    alerts: 'M12 1 3 5v6c0 5.6 3.8 10.7 9 12 5.2-1.3 9-6.4 9-12V5Zm0 10.99h7c-.5 4.1-3.2 7.8-7 8.9V12H5V6.3l7-3.1Z',
  };

  // Défilement vertical : la rangée ciblée reste en haut (sauf la première, qui montre le héros en entier).
  let scroller: HTMLElement | undefined = $state();
  $effect(() => {
    const r = model.focus.row;
    void rows.length;
    if (!scroller) return;
    const el = scroller.querySelector<HTMLElement>(`[data-r="${r}"]`);
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
    const top = zone === 'content' && r > 0 && el ? el.offsetTop - 3.2 * rem : 0;
    scroller.scrollTo({ top, behavior: reducedMotion() ? 'auto' : 'smooth' });
  });
  // Défilement horizontal de la rangée ciblée : la carte ciblée reste à une position fixe, sauf en début de rangée.
  $effect(() => {
    const c = model.focus.col;
    const r = model.focus.row;
    void zone;
    const track = scroller?.querySelector<HTMLElement>(`[data-r="${r}"] .track`);
    const el = track?.querySelector<HTMLElement>('.it.on, .it.sel');
    if (!track || !el) return;
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
    void c;
    track.scrollTo({ left: Math.max(0, el.offsetLeft - 4 * rem), behavior: reducedMotion() ? 'auto' : 'smooth' });
  });

  const hover = (z: string, key: string, i: number) => () => {
    model.ui[key] = i;
    model.setZone(z);
  };
</script>

<div class="tz">
  <!-- Rail de gauche -->
  <nav class="rail">
    {#each RAIL as id, i}
      <button class="rb" class:bottom={i === RAIL_BOTTOM} class:cur={id === 'home'} class:on={zone === 'rail' && railFocus === i} aria-label={t(`os.tizen.${id}`)} onmouseenter={hover('rail', 'railFocus', i)} onclick={() => ((model.ui.railFocus = i), model.setZone('rail'), model.activate())}>
        {#if id === 'profile'}<span class="av"></span>{:else}<svg viewBox="0 0 24 24"><path d={ICONS[id]} /></svg>{/if}
      </button>
    {/each}
  </nav>

  <div class="scroller" bind:this={scroller}>
    <!-- Héros : contenu ambiant de la tuile ciblée -->
    <section class="hero" style:--a={f?.color ?? '#233a6b'} style:--b={f?.color2 ?? '#0d1630'}>
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
          <button class="cta" class:on={zone === 'tabs' && tabFocus === 3} onmouseenter={hover('tabs', 'tabFocus', 3)} onclick={() => model.open(f)}>{t('os.tizen.open')}</button>
        {/if}
      {/key}
      <div class="tabs">
        {#each TABS as id, i}
          <button class="tab" class:sel={id === tab || (id === 'foryou' && tab !== 'apps')} class:on={zone === 'tabs' && tabFocus === i} onmouseenter={hover('tabs', 'tabFocus', i)} onclick={() => ((model.ui.tabFocus = i), model.setZone('tabs'), model.activate())}>{t(`os.tizen.tab.${id}`)}</button>
        {/each}
      </div>
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
                {#if tile.id.startsWith('continue:')}<span class="bar"><i style:width="{(tile.raw as any)?.duration > 0 ? Math.min(100, ((tile.raw as any).position / (tile.raw as any).duration) * 100) : 0}%"></i></span>{/if}
              </span>
              <span class="name">{tile.name}</span>
            </button>
          {/each}
        </div>
      </section>
    {/each}
    <div class="pad"></div>
  </div>

  <!-- Haut : horloge et état -->
  {#if model.widgetsNative}
    <header>
      <ModWidgets toast={model.note} at="status" provided={['status']} />
      <Weather />
      <span class="chip" class:on={model.phones > 0} title={model.phonesLabel}><i></i>{model.phones > 0 ? model.phones : ''}</span>
      <span class="clock">{model.time}</span>
    </header>
  {/if}

  <!-- Mode ambiance : n'importe quelle touche le quitte -->
  {#if zone === 'ambient'}
    <div class="amb" role="presentation" onclick={() => model.setZone('content')}>
      <Ambient time={model.time} date={model.date} palette={model.allTiles.slice(0, 6).map((x) => x.color)} weatherEnabled={model.weatherEnabled} phones={model.phones} status={t('os.tizen.hint')} />
    </div>
  {/if}
</div>

<style>
  .tz { position: absolute; inset: 0; overflow: hidden; font-family: var(--font); color: #f4f5fa; background: linear-gradient(180deg, #0d1022 0%, #080915 100%); }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; padding: 0; }
  svg { fill: currentColor; }

  /* rail de gauche : fine ligne bleutée à droite comme sur l'écran réel */
  .rail { position: absolute; z-index: 6; left: 0; top: 0; bottom: 0; width: 7rem; display: flex; flex-direction: column; align-items: center; gap: 0.9rem; padding: 3.2rem 0 2.4rem; box-sizing: border-box; background: linear-gradient(90deg, rgba(8, 10, 22, 0.85), rgba(8, 10, 22, 0.3)); }
  .rail::after { content: ''; position: absolute; right: 0; top: 4rem; bottom: 11rem; width: 0.12rem; background: linear-gradient(180deg, transparent, #2c8cff 25%, #2c8cff 75%, transparent); opacity: 0.55; }
  .rb { position: relative; width: 4rem; height: 4rem; border-radius: 50%; display: grid; place-items: center; color: rgba(255, 255, 255, 0.86); transition: background 0.2s, color 0.2s, transform 0.3s cubic-bezier(0.3, 1.3, 0.5, 1); }
  .rb svg { width: 2.2rem; height: 2.2rem; }
  .rb.bottom { margin-top: auto; }
  .rb.bottom ~ .rb { margin-top: 0; }
  .rb.cur { color: #2c9bff; }
  .rb.on { background: #fff; color: #10131c; transform: scale(1.1); }
  .av { width: 2.8rem; height: 2.8rem; border-radius: 50%; background: radial-gradient(circle at 50% 40%, #e8e6fb, #a9a3e8); }

  .scroller { position: absolute; z-index: 1; left: 7rem; right: 0; top: 0; bottom: 0; overflow: hidden; }

  /* héros */
  .hero { position: relative; height: 21.5rem; margin-bottom: 0.6rem; background: radial-gradient(70rem 36rem at 70% 0%, rgba(54, 96, 200, 0.4), transparent 70%), linear-gradient(180deg, #121634, transparent); }
  .art { position: absolute; inset: 0; animation: fadein 0.6s var(--ease) both; background: radial-gradient(52rem 26rem at 70% 20%, color-mix(in srgb, var(--a) 70%, transparent), transparent 72%), linear-gradient(120deg, color-mix(in srgb, var(--b) 55%, transparent), transparent 70%); }
  .art::after { content: ''; position: absolute; inset: 0; background: linear-gradient(0deg, #090a16 2%, transparent 65%), linear-gradient(90deg, rgba(9, 10, 22, 0.55), transparent 55%); }
  .art .big { position: absolute; right: 22%; top: 1rem; font-size: 17rem; font-weight: 800; line-height: 1; opacity: 0.16; }
  .art img { position: absolute; right: 24%; top: 3rem; width: 11rem; height: 11rem; object-fit: contain; filter: drop-shadow(0 1rem 2rem rgba(0, 0, 0, 0.5)); }
  .info { position: absolute; z-index: 2; left: 2.4rem; top: 4.4rem; width: min(40rem, 44%); animation: rise 0.5s var(--ease) both; }
  .kicker { font-size: 1.1rem; font-weight: 600; letter-spacing: 0.1em; text-transform: uppercase; color: rgba(255, 255, 255, 0.65); }
  h1 { margin: 0.4rem 0; font-size: 3.2rem; font-weight: 600; line-height: 1.1; letter-spacing: -0.01em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .info p { margin: 0; font-size: 1.35rem; color: rgba(255, 255, 255, 0.75); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .cta { position: absolute; z-index: 3; right: 6rem; top: 9.4rem; padding: 0.9rem 3.4rem; border-radius: 99rem; font-size: 1.3rem; font-weight: 500; background: rgba(255, 255, 255, 0.2); box-shadow: inset 0 0 0 0.1rem rgba(255, 255, 255, 0.35); backdrop-filter: blur(1rem); transition: background 0.2s, color 0.2s, transform 0.3s cubic-bezier(0.3, 1.3, 0.5, 1); }
  .cta.on { background: #fff; color: #10131c; transform: scale(1.06); }
  .tabs { position: absolute; z-index: 3; left: 2.4rem; bottom: 0.4rem; display: flex; padding: 0.25rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.12); box-shadow: inset 0 0 0 0.08rem rgba(255, 255, 255, 0.28); backdrop-filter: blur(1rem); }
  .tab { padding: 0.55rem 2.2rem; border-radius: 99rem; font-size: 1.25rem; font-weight: 500; color: rgba(255, 255, 255, 0.88); transition: background 0.2s, color 0.2s; }
  .tab.sel { background: rgba(255, 255, 255, 0.28); }
  .tab.on { background: #fff; color: #10131c; }

  /* rangées */
  .row { margin-bottom: 0.4rem; }
  h2 { margin: 0 0 0.2rem 2.4rem; font-size: 1.35rem; font-weight: 500; color: rgba(255, 255, 255, 0.9); }
  .track { display: flex; gap: 1.4rem; padding: 0.8rem 3rem 1rem 2.4rem; overflow-x: hidden; scrollbar-width: none; }
  .it { flex: none; width: 20rem; text-align: left; transition: transform 0.3s cubic-bezier(0.3, 1.3, 0.5, 1); }
  .face { position: relative; display: grid; place-items: center; aspect-ratio: 16 / 9; border-radius: var(--r); overflow: hidden; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg); box-shadow: 0 0.4rem 1.2rem rgba(0, 0, 0, 0.4); transition: box-shadow 0.2s; }
  .face b { font-size: 3.4rem; font-weight: 700; opacity: 0.45; }
  .face .ico { width: 4.2rem; height: 4.2rem; object-fit: contain; }
  .face .poster { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .face .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 0.3rem; background: rgba(255, 255, 255, 0.3); }
  .face .bar i { display: block; height: 100%; background: #3d9bff; }
  .name { display: block; margin-top: 0.6rem; font-size: 1.1rem; color: rgba(255, 255, 255, 0.78); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .it.on { transform: scale(var(--focus-scale)); position: relative; z-index: 2; }
  .it.on .face { box-shadow: 0 0 0 var(--focus-ring) #fff, 0 1rem 2rem rgba(0, 0, 0, 0.55); }
  .it.on .name { color: #fff; font-weight: 600; }
  .it.sel .face { box-shadow: 0 0 0 0.1rem rgba(255, 255, 255, 0.45); }

  /* rangée d'applications : icônes arrondies, nom visible seulement sous l'icône ciblée */
  .apps .it { width: 7.2rem; text-align: center; }
  .apps .face { aspect-ratio: 1; border-radius: 2.2rem; }
  .apps .face .ico { width: 3.8rem; height: 3.8rem; }
  .apps .face b { opacity: 1; font-size: 3rem; }
  .apps .name { opacity: 0; font-size: 1rem; transition: opacity 0.2s; }
  .apps .it.on .name { opacity: 1; }
  .apps .track { gap: 1.2rem; }
  .pad { height: 40rem; }

  header { position: absolute; z-index: 7; top: 1.6rem; right: 2.6rem; display: flex; align-items: center; gap: 1.1rem; font-size: 1.2rem; text-shadow: 0 0.1rem 0.6rem rgba(0, 0, 0, 0.6); }
  .clock { font-size: 1.4rem; font-weight: 500; }
  .chip { display: inline-flex; align-items: center; gap: 0.4rem; color: rgba(255, 255, 255, 0.7); }
  .chip i { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: #7a8296; }
  .chip.on i { background: #3ddc84; }

  .amb { position: absolute; inset: 0; z-index: 20; }

  @keyframes fadein { from { opacity: 0; } to { opacity: 1; } }
  @keyframes rise { from { opacity: 0; transform: translateY(0.8rem); } to { opacity: 1; transform: none; } }
</style>
