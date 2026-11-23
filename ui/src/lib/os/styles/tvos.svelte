<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../i18n';
  import ModWidgets from '../../ModWidgets.svelte';
  import { iconSrc, type DTile } from '../../tiles.svelte';
  import Weather from '../../Weather.svelte';
  import type { HomeModel } from '../model.svelte';
  import { tileFg } from '../util';
  import { NAV } from './tvos.meta';

  // Style Apple TV / tvOS (inspiré, non officiel) : capsule d'onglets, vitrine plein cadre, icônes en relief. Voir docs/OS-STYLES.md.
  let { model }: { model: HomeModel } = $props();

  const tab = $derived<string>(model.ui.tab ?? 'home');
  const navFocus = $derived<number>(model.ui.navFocus ?? Math.max(0, NAV.findIndex((e) => e.id === tab)));
  const inNav = $derived(model.zone === 'nav');
  const hasHero = $derived(tab === 'home');
  const grid = (key: string) => /^g\d+$/.test(key);
  const ROW_H = (key: string) => (key === 'continue' ? 20.4 : grid(key) ? 16.4 : 18);
  const STEP = 17.4; // rem : icône 15 + espace 2.4
  const WIDE = 22.4;

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
    const vis = Math.max(2, Math.floor((usable - 10) / step));
    return -Math.max(0, (act ? model.focus.col : 0) - (vis - 1)) * step;
  };

  // La vitrine suit la tuile ciblée avec un léger retard (fondu enchaîné).
  let heroTile = $state<DTile | undefined>(undefined);
  $effect(() => {
    const f = model.focused;
    const id = setTimeout(() => (heroTile = f), heroTile ? 180 : 0);
    return () => clearTimeout(id);
  });
  const tint = $derived(model.tinted && heroTile ? heroTile.color : '#3a3f55');

  // Inclinaison : la dernière direction décale le reflet et le sens de la bascule (la touche remplace le doigt sur le pavé tactile).
  const tx = $derived(model.tilt === 'left' ? -1 : model.tilt === 'right' ? 1 : 0);
  const ty = $derived(model.tilt === 'up' ? -1 : model.tilt === 'down' ? 1 : 0);

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
  const rowTop = $derived(hasHero && model.focus.row <= 0 ? 'var(--rt)' : '9.4rem');
</script>

<div class="tvos" bind:clientWidth={W} style:--rt="max(33rem, 60%)" style:--tint={tint}>
  <!-- Vitrine (Top Shelf) -->
  {#if hasHero}
    <section class="shelf" style:opacity={model.focus.row > 0 ? 0 : 1}>
      {#key heroTile?.id}
        <div class="art" transition:fade={{ duration: 420 }} style:--a={heroTile?.color ?? '#2a2f45'} style:--b={heroTile?.color2 ?? '#0b0d16'}>
          <div class="stage">
            {#if heroTile?.icon}<img src={iconSrc(heroTile.icon)} alt="" />{:else if heroTile}<span class="big">{heroTile.name[0]}</span>{/if}
          </div>
        </div>
        {#if heroTile}
          <div class="info" transition:fade={{ duration: 420 }}>
            <h1>{heroTile.name}</h1>
            <p>{t(`cat.${heroTile.category}`)} · {heroTile.desc}</p>
            <span class="play"><svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg>{t('os.tvos.open')}</span>
          </div>
        {/if}
      {/key}
    </section>
  {/if}

  <!-- Capsule d'onglets + profil -->
  <header>
    <div class="side"></div>
    <nav class="capsule">
      {#each NAV as e, i}
        {#if e.id !== 'settings'}
          <button class="tabi" class:sq={e.icon} class:sel={tab === e.id && e.kind === 'content'} class:focus={inNav && navFocus === i} onclick={() => press(i)} onmouseenter={() => hover(i)} aria-label={t(`os.tvos.${e.id}`)}>
            {#if e.id === 'search'}<svg viewBox="0 0 24 24"><path d="M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z" /></svg>{:else}{t(`os.tvos.${e.id}`)}{/if}
          </button>
        {/if}
      {/each}
    </nav>
    <div class="side right">
      {#if model.widgetsNative}
        <ModWidgets toast={model.note} at="status" provided={['status']} />
        <Weather />
        {#if model.phones > 0}<span class="phones" title={model.phonesLabel}><i></i>{model.phones}</span>{/if}
        <span class="clock">{model.time}</span>
      {/if}
      <button class="avatar" class:focus={inNav && navFocus === NAV.length - 1} onclick={() => press(NAV.length - 1)} onmouseenter={() => hover(NAV.length - 1)} aria-label={t('os.tvos.settings')}>R</button>
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
              <button class="card" class:wide={r.key === 'continue'} class:on class:dim={act && !on} onmouseenter={() => model.focusAt(ri, ci, true)} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))} style:--rx="{-ty * 5}deg" style:--ry="{tx * 5}deg" style:--sx="{50 + tx * 28}%" style:--sy="{50 + ty * 28}%" style:--gx="{-tx * 0.5}rem" style:--gy="{-ty * 0.4}rem">
                <span class="tilt">
                  {#if r.key === 'continue'}
                    <span class="face" style:--a="#3a4156" style:--b="#171a26">
                      {#if entryOf(ci)?.poster}<img class="cover" src={iconSrc(entryOf(ci).poster ?? '')} alt="" />{:else}<span class="mark">{tile.name[0]}</span>{/if}
                      <span class="meta"><svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg><span class="bar"><i style:width="{pct(ci)}%"></i></span><small>{tile.desc.split('·')[0]}</small></span>
                      <span class="spec"></span>
                    </span>
                  {:else}
                    <span class="face" style:--a={tile.color} style:--b={tile.color2} style:--fg={tileFg(tile.dark_text)}>
                      <span class="glyph">
                        {#if tile.icon}<img class="ico" src={iconSrc(tile.icon)} alt="" />{:else}<span class="letter">{tile.name[0]}</span>{/if}
                      </span>
                      <span class="spec"></span>
                    </span>
                  {/if}
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
  .tvos { --pad: max(5rem, calc((100% - 150rem) / 2)); position: absolute; inset: 0; background: var(--bg); overflow: hidden; font-family: var(--font); color: var(--text); }
  .tvos::before { content: ''; position: absolute; inset: 0; background: radial-gradient(70rem 36rem at 50% 70%, color-mix(in srgb, var(--tint) 26%, transparent), transparent 75%); transition: background 0.9s; }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; padding: 0; }

  header { position: absolute; z-index: 5; top: 2rem; left: 0; right: 0; padding: 0 var(--pad); display: grid; grid-template-columns: 1fr auto 1fr; align-items: center; }
  .side { display: flex; align-items: center; gap: 1.2rem; }
  .side.right { justify-content: flex-end; font-size: 1.25rem; font-weight: 600; text-shadow: 0 0.1rem 0.6rem rgba(0, 0, 0, 0.5); }
  .clock { padding: 0.5rem 1.2rem; border-radius: 99rem; background: var(--glass); backdrop-filter: blur(1.6rem) saturate(1.6); }
  .phones { display: inline-flex; align-items: center; gap: 0.4rem; color: var(--muted); }
  .phones i { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: #30d158; }
  .avatar { width: 3.4rem; height: 3.4rem; border-radius: 50%; font-size: 1.4rem; font-weight: 600; background: linear-gradient(135deg, #ff9f0a, #ff375f); color: #fff; transition: transform 0.2s var(--ease), box-shadow 0.2s; }
  .avatar.focus { transform: scale(1.14); box-shadow: 0 0 0 0.22rem #fff, 0 0.8rem 1.8rem rgba(0, 0, 0, 0.5); }
  .capsule { display: flex; align-items: center; gap: 0.2rem; padding: 0.4rem; border-radius: 99rem; background: var(--glass); backdrop-filter: blur(2.4rem) saturate(1.8); box-shadow: inset 0 0 0 0.06rem rgba(255, 255, 255, 0.1); }
  .tabi { height: 3.6rem; padding: 0 2.2rem; border-radius: 99rem; font-size: 1.35rem; font-weight: 500; color: var(--muted); display: inline-flex; align-items: center; transition: background 0.2s, color 0.2s, transform 0.2s var(--ease), box-shadow 0.2s; }
  .tabi.sq { width: 3.6rem; padding: 0; justify-content: center; }
  .tabi svg { width: 1.7rem; height: 1.7rem; fill: currentColor; }
  .tabi.sel { color: var(--text); font-weight: 600; }
  .tabi.focus { background: rgba(255, 255, 255, 0.92); color: #111; transform: scale(1.08); box-shadow: 0 0.8rem 1.8rem rgba(0, 0, 0, 0.45); }

  /* vitrine */
  .shelf { position: absolute; z-index: 1; left: 0; right: 0; top: 0; bottom: 0; transition: opacity 0.5s; }
  .art { position: absolute; inset: 0; background: radial-gradient(70rem 40rem at 66% 28%, var(--a), transparent 74%), linear-gradient(150deg, var(--b), transparent 72%); }
  .art::after { content: ''; position: absolute; inset: 0; background: linear-gradient(0deg, var(--bg) 0%, rgba(var(--bg-rgb), 0.78) 30%, transparent 62%), linear-gradient(90deg, rgba(var(--bg-rgb), 0.5), transparent 55%); }
  .stage { position: absolute; inset: 0 var(--pad); }
  .big { position: absolute; right: 6rem; top: 24%; font-size: 26rem; font-weight: 700; line-height: 1; opacity: 0.14; }
  .stage img { position: absolute; right: 12rem; top: 20%; width: 15rem; height: 15rem; object-fit: contain; opacity: 0.9; filter: drop-shadow(0 1.6rem 2.4rem rgba(0, 0, 0, 0.5)); }
  .info { position: absolute; z-index: 2; left: var(--pad); bottom: calc(100% - var(--rt) + 3.2rem); width: min(60rem, 70%); }
  h1 { margin: 0 0 0.6rem; font-size: 5.2rem; font-weight: 700; line-height: 1.04; letter-spacing: -0.025em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .info p { margin: 0 0 1.8rem; font-size: 1.5rem; font-weight: 500; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .play { display: inline-flex; align-items: center; gap: 0.6rem; height: 3.8rem; padding: 0 2.2rem 0 1.7rem; border-radius: 99rem; background: rgba(235, 235, 245, 0.26); backdrop-filter: blur(1.6rem) saturate(1.6); font-size: 1.35rem; font-weight: 600; }
  .play svg { width: 1.7rem; height: 1.7rem; fill: currentColor; }

  /* rangées */
  main { position: absolute; z-index: 2; left: 0; right: 0; bottom: 0; overflow: hidden; transition: top 0.4s var(--ease); }
  .rows { transform: translateY(var(--shift)); transition: transform 0.5s var(--ease); }
  .row { padding-left: var(--pad); opacity: 0.62; transition: opacity 0.4s; }
  .row.act { opacity: 1; }
  .row h2 { margin: 0 0 0.5rem; font-size: 1.55rem; font-weight: 600; color: var(--muted); }
  .track { display: flex; gap: 2.4rem; width: max-content; transform: translateX(var(--tx)); transition: transform 0.5s var(--ease); padding: 1.8rem 0 0 2rem; margin-left: -2rem; }
  .card { position: relative; flex: none; width: 15rem; text-align: center; perspective: 70rem; transition: transform 0.3s var(--ease), filter 0.3s; }
  .card.wide { width: 20rem; }
  .tilt { display: block; transform-style: preserve-3d; transition: transform 0.3s var(--ease); }
  .face { position: relative; display: block; width: 100%; aspect-ratio: 16 / 10; border-radius: var(--r); overflow: hidden; background: linear-gradient(145deg, var(--a), var(--b)); color: var(--fg, #fff); box-shadow: 0 0.6rem 1.6rem rgba(0, 0, 0, 0.45); transition: box-shadow 0.3s; }
  .card.wide .face { aspect-ratio: 16 / 9; }
  .face::before { content: ''; position: absolute; inset: 0; background: linear-gradient(180deg, rgba(255, 255, 255, 0.16), transparent 45%); }
  .glyph { position: absolute; inset: 0; display: grid; place-items: center; transition: transform 0.3s var(--ease), filter 0.3s; }
  .letter { font-size: 4rem; font-weight: 700; }
  .ico { width: 5.4rem; height: 5.4rem; object-fit: contain; filter: drop-shadow(0 0.4rem 0.6rem rgba(0, 0, 0, 0.3)); }
  .cover { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .mark { position: absolute; inset: 0; display: grid; place-items: center; font-size: 5rem; font-weight: 700; opacity: 0.22; }
  .meta { position: absolute; left: 1rem; right: 1rem; bottom: 0.8rem; display: flex; align-items: center; gap: 0.6rem; font-size: 1.05rem; font-weight: 500; }
  .meta svg { width: 1.1rem; height: 1.1rem; fill: currentColor; }
  .meta .bar { width: 3rem; height: 0.28rem; border-radius: 1rem; background: rgba(255, 255, 255, 0.3); overflow: hidden; }
  .meta .bar i { display: block; height: 100%; background: #fff; }
  .spec { position: absolute; inset: 0; opacity: 0; background: radial-gradient(circle at var(--sx, 50%) var(--sy, 50%), rgba(255, 255, 255, 0.5), transparent 58%); mix-blend-mode: soft-light; transition: opacity 0.3s; pointer-events: none; }
  .name { position: absolute; left: -1rem; right: -1rem; top: 100%; margin-top: 1.5rem; font-size: 1.3rem; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; opacity: 0; transform: translateY(-0.4rem); transition: opacity 0.25s, transform 0.3s var(--ease); }
  .card.dim { filter: brightness(0.8); }
  .card.on { z-index: 3; transform: translateY(-0.8rem) scale(var(--focus-scale)); }
  .card.on .tilt { animation: settle 0.9s var(--ease) both; }
  .card.on .face { box-shadow: 0 2.6rem 3.6rem -0.6rem rgba(0, 0, 0, 0.7), 0 0.8rem 1.6rem rgba(0, 0, 0, 0.45), 0 0 0 0.1rem rgba(255, 255, 255, 0.35); }
  .card.on .glyph { transform: translate(var(--gx), var(--gy)) scale(1.1); }
  .card.on .spec { opacity: 1; }
  .card.on .name { opacity: 1; transform: none; }
  @keyframes settle { from { transform: rotateX(var(--rx)) rotateY(var(--ry)); } to { transform: rotateX(calc(var(--rx) * 0.3)) rotateY(calc(var(--ry) * 0.3)); } }
  @media (prefers-reduced-motion: reduce) { .card, .tilt, .track, .rows, main, .glyph { transition: none; } .card.on .tilt { animation: none; } }
</style>
