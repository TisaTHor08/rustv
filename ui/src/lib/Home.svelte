<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import { api, type ShellState } from './api';
  import { locale, t } from './i18n';
  import { askText } from './overlays.svelte';
  import { anyOverlay } from './overlays.svelte';
  import OverlayHost from './OverlayHost.svelte';
  import ModManager from './ModManager.svelte';
  import ModWidgets from './ModWidgets.svelte';
  import { mods, pageSrc, type ModTile } from './mods.svelte';
  import PageHost from './PageHost.svelte';
  import Pairing from './Pairing.svelte';
  import { prefs } from './prefs.svelte';
  import Settings from './Settings.svelte';
  import TileMenu from './TileMenu.svelte';
  import { CATEGORIES, findTile, iconSrc, moveTile, tilesOf, type DTile } from './tiles.svelte';
  import Wallpaper from './Wallpaper.svelte';
  import Weather from './Weather.svelte';

  type Modal = null | { kind: 'settings' } | { kind: 'pairing' } | { kind: 'mods' } | { kind: 'page'; modId: string; entry: string; title: string } | { kind: 'tile'; id: string };

  const TILE_STEP = 18.6; // rem : largeur d'une tuile + espacement
  const ROW_H = 17; // rem

  let st = $state<ShellState | null>(null);
  let now = $state(new Date());
  let focus = $state({ row: 0, col: 0 });
  let tilt = $state('');
  let modal = $state<Modal>(null);
  let toast = $state('');
  let recent = $state<string[]>(loadRecent());

  function loadRecent(): string[] {
    try {
      return JSON.parse(localStorage.getItem('rustv.recent') ?? '[]');
    } catch {
      return [];
    }
  }

  const rows = $derived.by(() => {
    if (!st) return [];
    // dépendances explicites : la liste se recalcule quand les préférences de tuiles changent
    const catalog = st.catalog;
    void prefs.tiles.order, prefs.tiles.hidden, prefs.tiles.names, prefs.tiles.colors, prefs.tiles.icons, prefs.tiles.custom, prefs.lang, mods.list;
    const out: { key: string; title: string; tiles: DTile[] }[] = [];
    const rec = recent.map((id) => findTile(catalog, id)).filter((x): x is DTile => !!x && !prefs.tiles.hidden.includes(x.id));
    if (rec.length) out.push({ key: 'recent', title: t('row.recent'), tiles: rec });
    for (const c of CATEGORIES) {
      const tiles = tilesOf(catalog, c);
      if (tiles.length) out.push({ key: c, title: t(`cat.${c}`), tiles });
    }
    return out;
  });

  // garde le focus dans les limites quand les rangées changent (masquage, suppression…)
  $effect(() => {
    if (!rows.length) return;
    const row = Math.min(focus.row, rows.length - 1);
    const col = row >= 0 ? Math.min(focus.col, rows[row].tiles.length - 1) : 0;
    if (row !== focus.row || col !== focus.col) focus = { row, col };
  });

  const focused = $derived(focus.row >= 0 ? rows[focus.row]?.tiles[focus.col] : undefined);
  const phones = $derived(st ? st.devices.filter((d) => d.connected).length : 0);
  const tinted = $derived(prefs.tint && !!focused);

  async function refresh() {
    st = await api.state();
  }

  function note(msg: string) {
    toast = msg;
    setTimeout(() => (toast = ''), 3500);
  }

  // ---------- navigation à la télécommande ----------

  function move(dr: number, dc: number, dirName: string) {
    tilt = dirName;
    let { row, col } = focus;
    if (dr !== 0) {
      const nr = Math.max(-1, Math.min(rows.length - 1, row + dr));
      if (nr === row) return;
      row = nr;
      col = row >= 0 ? Math.min(col, rows[row].tiles.length - 1) : 0;
    } else if (row >= 0) {
      col = Math.max(0, Math.min(rows[row].tiles.length - 1, col + dc));
    }
    focus = { row, col };
  }

  async function activate() {
    if (focus.row === -1) {
      modal = { kind: 'settings' };
      return;
    }
    if (focused) await openTile(focused);
  }

  async function openTile(tile: DTile) {
    try {
      let r: string;
      if (tile.source === 'mod') {
        const raw = tile.raw as ModTile;
        if (raw.kind === 'page') {
          modal = { kind: 'page', modId: raw.modId, entry: raw.entry ?? 'index.html', title: tile.name };
          return;
        }
        r = await api.launchCustom({ id: raw.id, name: tile.name, kind: raw.kind, url: raw.url, target: raw.target, spatnav: raw.spatnav });
      } else if (tile.custom) r = await api.launchCustom(tile.raw);
      else {
        const needs = (tile.raw as any).needs_url && !st?.settings.urls[tile.id];
        r = needs ? 'needs_url' : await api.launch(tile.id);
      }
      if (r === 'needs_url') {
        const v = await askText(t('url.title', { name: tile.name }), st?.settings.urls[tile.id] ?? '', (tile.raw as any).url_hint ?? '');
        if (!v) return;
        r = await api.launch(tile.id, v);
        await refresh();
      }
      if (r === 'ok') {
        recent = [tile.id, ...recent.filter((x) => x !== tile.id)].slice(0, 8);
        try {
          localStorage.setItem('rustv.recent', JSON.stringify(recent));
        } catch {}
      }
    } catch (e) {
      note(String(e));
    }
  }

  async function onMove(id: string, dir: -1 | 1) {
    if (!st) return;
    const i = moveTile(st.catalog, id, dir);
    await tick();
    if (i >= 0 && focus.row >= 0 && rows[focus.row]?.key !== 'recent') focus = { row: focus.row, col: i };
  }

  function onKey(e: KeyboardEvent) {
    if (modal || anyOverlay()) return; // chaque fenêtre gère ses propres touches
    switch (e.key) {
      case 'ArrowDown': move(1, 0, 'down'); break;
      case 'ArrowUp': move(-1, 0, 'up'); break;
      case 'ArrowLeft': move(0, -1, 'left'); break;
      case 'ArrowRight': move(0, 1, 'right'); break;
      case 'Enter': activate(); break;
      case 'ContextMenu':
        if (focused) modal = { kind: 'tile', id: focused.id };
        break;
      default: return;
    }
    e.preventDefault();
  }

  // ---------- cycle de vie ----------

  let idleTimer: number | undefined;
  function wake() {
    document.documentElement.classList.remove('idle');
    clearTimeout(idleTimer);
    idleTimer = window.setTimeout(() => document.documentElement.classList.add('idle'), 3000);
  }

  onMount(() => {
    refresh();
    const clock = setInterval(() => (now = new Date()), 1000 * 15);
    const un = api.onDevicesChanged(() => refresh());
    wake();
    return () => {
      clearInterval(clock);
      un.then((f) => f());
    };
  });

  const time = $derived(now.toLocaleTimeString(locale(), { hour: '2-digit', minute: '2-digit', hour12: !prefs.clock24 }));
  const date = $derived(now.toLocaleDateString(locale(), { weekday: 'long', day: 'numeric', month: 'long' }));
  const fg = (x?: DTile) => (x?.dark_text ? '#16130a' : '#ffffff');
  const tileFor = (id: string) => (st ? findTile(st.catalog, id) : undefined);
  const menuTile = $derived(modal?.kind === 'tile' ? tileFor(modal.id) : undefined);
</script>

<svelte:window onkeydown={onKey} onmousemove={wake} />

<div class="root" style:--c1={tinted ? focused!.color : 'transparent'} style:--c2={tinted ? focused!.color2 : 'transparent'}>
  <Wallpaper />
  <div class="tint"></div>
  <div class="veil"></div>

  <!-- Barre du haut -->
  <header>
    <div class="brand">
      <svg viewBox="0 0 64 64" aria-hidden="true">
        <defs>
          <linearGradient id="lg" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stop-color="#ffb347" /><stop offset="0.6" stop-color="#f74c00" /><stop offset="1" stop-color="#b83200" />
          </linearGradient>
        </defs>
        <g fill="url(#lg)" transform="translate(32 32)">
          <circle r="20" fill="none" stroke="url(#lg)" stroke-width="4.4" />
          {#each Array(12) as _, i}<rect x="-2.3" y="-29" width="4.6" height="7.4" rx="1" transform="rotate({i * 30})" />{/each}
          <path d="M-5.6 -8 L-5.6 8 Q-5.6 9.4 -4.4 8.6 L9 1.2 Q10 0 9 -1.2 L-4.4 -8.6 Q-5.6 -9.4 -5.6 -8 Z" />
        </g>
      </svg>
      <span>RusTV</span>
    </div>
    <div class="right">
      <ModWidgets toast={note} />
      <Weather />
      <div class="pill" class:on={phones > 0}>
        <span class="dot"></span>{phones === 0 ? t('phones.none') : phones === 1 ? t('phones.one') : t('phones.many', { n: phones })}
      </div>
      <div class="clock"><strong>{time}</strong><small>{date}</small></div>
      <button class="icon" class:focus={focus.row === -1} onclick={() => (modal = { kind: 'settings' })} onmouseenter={() => (focus = { row: -1, col: 0 })} aria-label={t('settings.title')}>
        <svg viewBox="0 0 24 24"><path d="M19.4 13a7.7 7.7 0 0 0 0-2l2-1.6a.5.5 0 0 0 .1-.6l-1.9-3.3a.5.5 0 0 0-.6-.2l-2.4 1a7.2 7.2 0 0 0-1.7-1l-.4-2.5a.5.5 0 0 0-.5-.4h-3.8a.5.5 0 0 0-.5.4L9.3 5.3a7.2 7.2 0 0 0-1.7 1l-2.4-1a.5.5 0 0 0-.6.2L2.7 8.8a.5.5 0 0 0 .1.6l2 1.6a7.7 7.7 0 0 0 0 2l-2 1.6a.5.5 0 0 0-.1.6l1.9 3.3a.5.5 0 0 0 .6.2l2.4-1c.5.4 1.1.7 1.7 1l.4 2.5c0 .2.2.4.5.4h3.8c.3 0 .5-.2.5-.4l.4-2.5c.6-.3 1.2-.6 1.7-1l2.4 1c.2.1.5 0 .6-.2l1.9-3.3a.5.5 0 0 0-.1-.6ZM12 15.5A3.5 3.5 0 1 1 12 8.5a3.5 3.5 0 0 1 0 7Z" /></svg>
      </button>
    </div>
  </header>

  <!-- Héro -->
  <section class="hero">
    {#key focused?.id}
      <div class="hero-in" in:fly={{ y: 24, duration: 520, easing: cubicOut }}>
        {#if focused}
          <span class="eyebrow">{t(`cat.${focused.category}`)}</span>
          <h1>{focused.name}</h1>
          <p>{focused.desc}</p>
          <div class="cta"><span class="play">▶</span> {t('hero.open')} <kbd>OK</kbd></div>
        {:else}
          <span class="eyebrow">RusTV</span>
          <h1>{t('settings.title')}</h1>
          <p>{t('settings.appearance')} · {t('settings.wallpaper')} · {t('settings.tiles')}</p>
        {/if}
      </div>
    {/key}
  </section>

  <!-- Rangées -->
  <main style:--shift="{-focus.row * ROW_H}rem">
    <div class="rows">
      {#each rows as r, ri (r.key)}
        <div class="row" class:active={ri === focus.row}>
          <h2>{r.title}</h2>
          <div class="track" style:--tx="{-Math.max(0, (ri === focus.row ? focus.col : 0) - 2) * TILE_STEP}rem">
            {#each r.tiles as tile, ci (tile.id)}
              <button
                class="tile"
                class:focus={ri === focus.row && ci === focus.col}
                data-tilt={ri === focus.row && ci === focus.col ? tilt : ''}
                style:--a={tile.color}
                style:--b={tile.color2}
                style:--fg={fg(tile)}
                onmouseenter={() => (focus = { row: ri, col: ci })}
                onclick={() => openTile(tile)}
                oncontextmenu={(e) => (e.preventDefault(), (modal = { kind: 'tile', id: tile.id }))}
              >
                {#if tile.icon}
                  <img class="ico" src={iconSrc(tile.icon)} alt="" />
                {:else}
                  <span class="mark">{tile.name[0]}</span>
                {/if}
                <span class="shine"></span>
                <span class="label">{tile.name}</span>
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </main>

  {#if toast}<div class="toast" transition:fade={{ duration: 200 }}>{toast}</div>{/if}

  <!-- Fenêtres modales -->
  {#if modal}
    <div class="scrim" transition:fade={{ duration: 180 }}>
      {#if modal.kind === 'settings'}
        <Settings {st} {refresh} {note} onclose={() => (modal = null)} onpairing={() => (modal = { kind: 'pairing' })} onmods={() => (modal = { kind: 'mods' })} />
      {:else if modal.kind === 'pairing'}
        <div class="sheet wide" in:fly={{ y: 30, duration: 360, easing: cubicOut }}>
          <Pairing embedded onclose={() => (modal = { kind: 'settings' })} />
        </div>
      {:else if modal.kind === 'mods'}
        <ModManager {note} onclose={() => (modal = { kind: 'settings' })} />
      {:else if modal.kind === 'tile' && menuTile && st}
        <TileMenu tile={menuTile} onclose={() => (modal = null)} onopen={() => openTile(menuTile)} onmove={(d) => onMove(menuTile.id, d)} />
      {/if}
    </div>
  {/if}

  {#if modal?.kind === 'page'}
    <PageHost modId={modal.modId} src={pageSrc(modal.modId, modal.entry)} title={modal.title} onclose={() => (modal = null)} toast={note} />
  {/if}

  <OverlayHost />
</div>

<style>
  @property --c1 { syntax: '<color>'; inherits: true; initial-value: transparent; }
  @property --c2 { syntax: '<color>'; inherits: true; initial-value: transparent; }

  .root { position: fixed; inset: 0; background: var(--bg); transition: --c1 0.9s ease, --c2 0.9s ease; }
  .tint {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(70rem 46rem at 14% 4%, color-mix(in srgb, var(--c1) 68%, transparent), transparent 62%),
      radial-gradient(64rem 42rem at 96% -6%, color-mix(in srgb, var(--c2) 85%, transparent), transparent 66%);
  }
  .veil { position: absolute; inset: 0; background: linear-gradient(to bottom, transparent 18%, rgba(var(--bg-rgb), 0.78) 52%, rgba(var(--bg-rgb), 0.94)); }

  /* ---- barre du haut ---- */
  header { position: absolute; inset: 2.4rem 5rem auto 5rem; display: flex; justify-content: space-between; align-items: center; z-index: 5; }
  .brand { display: flex; align-items: center; gap: 0.8rem; font-weight: 700; font-size: 1.5rem; letter-spacing: 0.02em; }
  .brand svg { width: 2.6rem; height: 2.6rem; }
  .right { display: flex; align-items: center; gap: 1.4rem; }
  .pill { display: flex; align-items: center; gap: 0.6rem; padding: 0.5rem 1.1rem; border-radius: 99rem; background: var(--glass); font-size: 1.05rem; color: var(--muted); backdrop-filter: blur(1.2rem); }
  .pill .dot { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: #888; }
  .pill.on { color: var(--text); }
  .pill.on .dot { background: #3ddc84; box-shadow: 0 0 0.8rem #3ddc84; }
  .clock { text-align: right; line-height: 1.15; }
  .clock strong { display: block; font-size: 1.7rem; font-weight: 600; }
  .clock small { display: block; font-size: 0.95rem; color: var(--muted); }
  .clock small::first-letter { text-transform: uppercase; }
  .icon { width: 3.4rem; height: 3.4rem; border-radius: 50%; border: 0; background: var(--glass); display: grid; place-items: center; transition: transform 0.3s var(--ease), background 0.3s; }
  .icon svg { width: 1.7rem; height: 1.7rem; fill: var(--text); }
  .icon.focus { background: var(--ring); transform: scale(1.18); box-shadow: 0 0.6rem 2rem rgba(0, 0, 0, 0.5); }
  .icon.focus svg { fill: var(--bg); }

  /* ---- héro ---- */
  .hero { position: absolute; left: 5rem; top: 10.5rem; width: 56rem; height: 15rem; z-index: 3; }
  .hero-in { position: absolute; inset: 0; }
  .eyebrow { font-size: 1.1rem; letter-spacing: 0.18em; text-transform: uppercase; color: var(--muted); font-weight: 600; }
  h1 { margin: 0.5rem 0 0.4rem; font-size: 4.6rem; line-height: 1; font-weight: 800; letter-spacing: -0.02em; text-shadow: 0 0.3rem 2rem rgba(0, 0, 0, 0.35); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .hero p { margin: 0 0 1.4rem; font-size: 1.5rem; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .cta { display: inline-flex; align-items: center; gap: 0.8rem; padding: 0.7rem 1.6rem 0.7rem 1.3rem; border-radius: 99rem; background: var(--accent); color: #fff; font-weight: 700; font-size: 1.25rem; box-shadow: 0 0.6rem 1.6rem color-mix(in srgb, var(--accent) 45%, transparent); }
  .cta .play { font-size: 0.95rem; }
  kbd { font: inherit; font-size: 0.8rem; padding: 0.15rem 0.5rem; border-radius: 0.4rem; background: rgba(0, 0, 0, 0.2); }

  /* ---- rangées ---- */
  main { position: absolute; left: 0; right: 0; top: 27rem; bottom: 0; overflow: hidden; z-index: 4; }
  .rows { transform: translateY(var(--shift)); transition: transform 0.55s var(--ease); }
  .row { height: 17rem; padding-left: 5rem; opacity: 0.55; transition: opacity 0.4s; }
  .row.active { opacity: 1; }
  .row h2 { margin: 0 0 1.9rem; font-size: 1.35rem; font-weight: 600; color: var(--muted); letter-spacing: 0.02em; }
  .track { display: flex; gap: 1.6rem; transform: translateX(var(--tx)); transition: transform 0.55s var(--ease); width: max-content; }

  .tile {
    position: relative;
    flex: none;
    width: 17rem;
    height: 9.6rem;
    border: 0;
    padding: 0;
    border-radius: 1.2rem;
    overflow: hidden;
    color: var(--fg);
    background: linear-gradient(135deg, var(--a), var(--b));
    box-shadow: 0 0.6rem 1.4rem -0.6rem rgba(0, 0, 0, 0.6);
    transition: transform 0.4s var(--ease), box-shadow 0.4s var(--ease);
    font-family: inherit;
    cursor: pointer;
    transform-style: preserve-3d;
  }
  .tile .mark { position: absolute; right: -0.6rem; bottom: -2.6rem; font-size: 10rem; font-weight: 800; opacity: 0.14; line-height: 1; }
  .tile .ico { position: absolute; left: 1.1rem; top: 1rem; width: 3.6rem; height: 3.6rem; object-fit: contain; filter: drop-shadow(0 0.2rem 0.5rem rgba(0, 0, 0, 0.35)); }
  .tile .label { position: absolute; left: 1.1rem; right: 1rem; bottom: 0.9rem; font-size: 1.55rem; font-weight: 700; letter-spacing: -0.01em; text-shadow: 0 0.1rem 0.6rem rgba(0, 0, 0, 0.35); text-align: left; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tile .shine { position: absolute; inset: 0; background: linear-gradient(115deg, transparent 28%, rgba(255, 255, 255, 0.32) 48%, transparent 66%); transform: translateX(-130%); }
  .tile.focus {
    transform: perspective(60rem) translateY(-0.3rem) scale(1.16);
    box-shadow:
      0 0 0 0.22rem var(--ring),
      0 2.2rem 3.6rem -0.8rem color-mix(in srgb, var(--a) 80%, transparent),
      0 1rem 2rem rgba(0, 0, 0, 0.5);
    z-index: 2;
    animation: settle 0.5s var(--ease);
  }
  .tile.focus .shine { animation: sweep 1.3s 0.1s var(--ease) forwards; }
  .tile.focus[data-tilt='right'] { --ry: 5deg; --rx: 0deg; }
  .tile.focus[data-tilt='left'] { --ry: -5deg; --rx: 0deg; }
  .tile.focus[data-tilt='down'] { --rx: -5deg; --ry: 0deg; }
  .tile.focus[data-tilt='up'] { --rx: 5deg; --ry: 0deg; }
  @keyframes settle {
    from { transform: perspective(60rem) translateY(0) scale(1.04) rotateX(var(--rx, 0deg)) rotateY(var(--ry, 0deg)); }
    to { transform: perspective(60rem) translateY(-0.3rem) scale(1.16) rotateX(0) rotateY(0); }
  }
  @keyframes sweep { to { transform: translateX(130%); } }

  .toast { position: absolute; left: 50%; bottom: 3rem; transform: translateX(-50%); padding: 0.9rem 1.8rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.95); color: #111; font-size: 1.2rem; font-weight: 600; z-index: 40; }

  .scrim { position: absolute; inset: 0; background: rgba(4, 6, 10, 0.72); backdrop-filter: blur(1.6rem); display: grid; place-items: center; z-index: 10; }
  .sheet.wide { width: 56rem; padding: 2.2rem 2.4rem; border-radius: 1.8rem; background: var(--sheet); box-shadow: 0 3rem 6rem rgba(0, 0, 0, 0.6), inset 0 0 0 0.08rem var(--glass-strong); }
</style>
