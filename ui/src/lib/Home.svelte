<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { cubicOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import { api, type ShellState } from './api';
  import { locale, t } from './i18n';
  import { askText } from './overlays.svelte';
  import { anyOverlay } from './overlays.svelte';
  import OverlayHost from './OverlayHost.svelte';
  import LivePage from './LivePage.svelte';
  import MediaPage from './MediaPage.svelte';
  import LaunchOverlay from './LaunchOverlay.svelte';
  import { continueList, continueRemove, playMedia, type ContinueEntry } from './player.svelte';
  import './continue.strings';
  import Search from './Search.svelte';
  import ModManager from './ModManager.svelte';
  import WidgetDock from './os/WidgetDock.svelte';
  import { osState } from './os/apply.svelte';
  import type { HomeModel, HomeRow } from './os/model.svelte';
  import { mods, pageSrc, type ModTile } from './mods.svelte';
  import PageHost from './PageHost.svelte';
  import Pairing from './Pairing.svelte';
  import { prefs, savePrefs } from './prefs.svelte';
  import { play } from './sounds';
  import Settings from './Settings.svelte';
  import TileMenu from './TileMenu.svelte';
  import { CATEGORIES, findTile, iconSrc, moveTile, tilesOf, type BuiltinTile, type DTile } from './tiles.svelte';

  type Modal = null | { kind: 'settings' } | { kind: 'pairing' } | { kind: 'mods' } | { kind: 'live' } | { kind: 'search' } | { kind: 'media'; tab: 'movies' | 'series' | 'photos' | 'music' | 'files' } | { kind: 'page'; modId: string; entry: string; title: string } | { kind: 'tile'; id: string };

  let st = $state<ShellState | null>(null);
  let now = $state(new Date());
  let focus = $state({ row: 0, col: 0 });
  let tilt = $state('');
  let zone = $state('content');
  const ui = $state<Record<string, any>>({});
  let modal = $state<Modal>(null);
  let toast = $state('');
  let recent = $state<string[]>(loadRecent());
  let cont = $state<ContinueEntry[]>([]);
  const loadCont = () => continueList().then((l) => (cont = l)).catch(() => {});
  const contTile = (e: ContinueEntry): DTile => ({ id: `continue:${e.url}`, name: e.title, desc: t('continue.pct', { n: e.duration > 0 ? Math.round((e.position / e.duration) * 100) : 0 }), category: 'video', color: '#2a2f3d', color2: '#12151c', custom: false, source: 'builtin', customized: false, raw: e as any });

  function loadRecent(): string[] {
    try {
      return JSON.parse(localStorage.getItem('rustv.recent') ?? '[]');
    } catch {
      return [];
    }
  }

  const baseRows = $derived.by(() => {
    if (!st) return [];
    // dépendances explicites : la liste se recalcule quand les préférences de tuiles changent
    const catalog = st.catalog;
    void prefs.tiles.order, prefs.tiles.hidden, prefs.tiles.names, prefs.tiles.colors, prefs.tiles.icons, prefs.tiles.custom, prefs.lang, mods.list;
    const out: HomeRow[] = [];
    if (cont.length) out.push({ key: 'continue', title: t('continue.title'), tiles: cont.map(contTile) });
    const rec = recent.map((id) => findTile(catalog, id)).filter((x): x is DTile => !!x && !prefs.tiles.hidden.includes(x.id));
    if (rec.length) out.push({ key: 'recent', title: t('row.recent'), tiles: rec });
    for (const c of CATEGORIES) {
      const tiles = tilesOf(catalog, c);
      if (tiles.length) out.push({ key: c, title: t(`cat.${c}`), tiles });
    }
    return out;
  });

  // Ce que voient les mises en page de style (voir src/lib/os/model.svelte.ts).
  const model: HomeModel = {
    get rows() { return rows; },
    get focus() { return focus; },
    get focused() { return focused; },
    get zone() { return zone; },
    get tilt() { return tilt; },
    get time() { return time; },
    get date() { return date; },
    get phones() { return phones; },
    get phonesLabel() { return phonesLabel; },
    get weatherEnabled() { return prefs.weather.enabled; },
    get widgetPos() { return prefs.widgetPos; },
    get widgetsNative() { return prefs.widgetPos === 'auto'; },
    get recent() { return recentTiles; },
    get continueEntries() { return cont; },
    get allTiles() { return allTiles; },
    get tinted() { return tinted; },
    get hero() { return hero; },
    ui,
    move,
    focusAt,
    setZone,
    activate: () => void activate(),
    open: (tile) => void openTile(tile),
    resume: (url) => void resume(url),
    openSearch: () => (modal = { kind: 'search' }),
    openSettings: () => (modal = { kind: 'settings' }),
    openLive: () => (modal = { kind: 'live' }),
    openMedia: (tab) => (modal = { kind: 'media', tab }),
    contextMenu: (tile) => {
      if (tile.id.startsWith('continue:')) continueRemove(tile.id.slice(9)).then(loadCont);
      else modal = { kind: 'tile', id: tile.id };
    },
    tileById: (id) => tileFor(id),
    note,
  };

  const rows = $derived(osState.style.rows ? osState.style.rows(baseRows, model) : baseRows);

  // garde le focus dans les limites quand les rangées changent (masquage, suppression…)
  $effect(() => {
    if (!rows.length) return;
    const row = Math.min(focus.row, rows.length - 1);
    const col = row >= 0 ? Math.min(focus.col, rows[row].tiles.length - 1) : 0;
    if (row !== focus.row || col !== focus.col) focus = { row, col };
  });

  const focused = $derived(focus.row >= 0 ? rows[focus.row]?.tiles[focus.col] : undefined);
  const phones = $derived(st ? st.devices.filter((d) => d.connected).length : 0);
  // Ambilight (Hyperion) : couleur d'ambiance de la tuile ciblée (anti-rebond et filtrage côté Rust).
  $effect(() => {
    if (focused) invoke('hyperion_ambient', { color: focused.color, color2: focused.color2 }).catch(() => {});
  });

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
    if (row === focus.row && col === focus.col) return;
    focus = { row, col };
    play('move');
  }

  async function activate() {
    play('select');
    if (focus.row === -1) {
      modal = { kind: 'settings' };
      return;
    }
    if (focused?.id.startsWith('continue:')) return void (await resume(focused.id.slice(9)));
    if (focused) await openTile(focused);
  }

  async function resume(url: string) {
    const e = cont.find((x) => x.url === url);
    if (e) await playMedia(e.kind === 'audio' ? 'audio' : 'video', [{ name: e.title, url: e.url, poster: e.poster ?? undefined }], 0, note, e.position);
  }

  // Animation de lancement d'une app web : bloque les entrées jusqu'à ce que la fenêtre de l'app soit affichée.
  let launching = $state<{ id: string; name: string; color: string; color2: string; slow: boolean } | null>(null);
  let launchTimer: number | undefined;
  function endLaunch() {
    clearTimeout(launchTimer);
    launching = null;
  }
  function beginLaunch(tile: DTile) {
    launching = { id: tile.id, name: tile.name, color: tile.color, color2: tile.color2, slow: false };
    clearTimeout(launchTimer);
    launchTimer = window.setTimeout(() => launching && (launching.slow = true), 8000);
    // Sécurité : l'animation ne reste jamais plus de 40 s.
    window.setTimeout(() => launching?.id === tile.id && endLaunch(), 40000);
  }
  const isWebTile = (tile: DTile) => tile.source !== 'builtin' && (tile.raw as any).kind !== 'native' && (tile.raw as any).kind !== 'page';

  async function openTile(tile: DTile) {
    if (launching) return;
    const web = isWebTile(tile) && !((tile.raw as any).needs_url && !st?.settings.urls[tile.id]);
    if (web) beginLaunch(tile);
    try {
      let r: string;
      if (tile.source === 'builtin') {
        const raw = tile.raw as BuiltinTile;
        if (raw.action === 'live') modal = { kind: 'live' };
        else if (raw.action?.startsWith('media:')) modal = { kind: 'media', tab: raw.action.slice(6) as any };
        else r = await api.launchCustom({ id: raw.id, name: tile.name, kind: 'native', target: raw.target });
        return;
      }
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
        r = needs ? 'needs_url' : await api.launch(tile.id, undefined, prefs.youtubeBrowser);
      }
      if (r === 'needs_url') {
        const v = await askText(t('url.title', { name: tile.name }), st?.settings.urls[tile.id] ?? '', (tile.raw as any).url_hint ?? '');
        if (!v) return;
        r = await api.launch(tile.id, v);
        await refresh();
      }
      if (r !== 'ok') endLaunch();
      if (r === 'ok') {
        if (!web) endLaunch();
        recent = [tile.id, ...recent.filter((x) => x !== tile.id)].slice(0, 8);
        try {
          localStorage.setItem('rustv.recent', JSON.stringify(recent));
        } catch {}
      }
    } catch (e) {
      endLaunch();
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
    if (launching) {
      // Pendant l'animation de lancement : rien ne passe, sauf Retour qui annule.
      if (e.key === 'Escape' || e.key === 'Backspace') {
        void api.launchCancel();
        endLaunch();
      }
      e.preventDefault();
      return;
    }
    if (modal || anyOverlay()) return; // chaque fenêtre gère ses propres touches
    const sty = osState.style;
    const dirs: Record<string, [number, number, 'up' | 'down' | 'left' | 'right']> = { ArrowDown: [1, 0, 'down'], ArrowUp: [-1, 0, 'up'], ArrowLeft: [0, -1, 'left'], ArrowRight: [0, 1, 'right'] };
    const d = dirs[e.key];
    if (d) {
      if (!sty.nav?.(d[2], model)) move(d[0], d[1], d[2]);
      e.preventDefault();
      return;
    }
    switch (e.key) {
      case 'Enter':
        if (!sty.activate?.(model)) activate();
        break;
      case 'Escape':
      case 'Backspace':
        // Retour depuis une zone annexe (barre d'onglets, menu...) : revient d'abord au contenu ; sinon rien (comme avant).
        if (zone === 'content') return;
        setZone('content');
        play('back');
        break;
      case 'F3':
        modal = { kind: 'search' };
        break;
      case 'ContextMenu':
        if (focused?.id.startsWith('continue:')) continueRemove(focused.id.slice(9)).then(loadCont);
        else if (focused) modal = { kind: 'tile', id: focused.id };
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
    loadCont();
    const unClosed = api.onPlayerClosed(() => loadCont());
    // Vérification quotidienne des mises à jour (désactivable) : un simple GET vers l'API GitHub.
    if (prefs.autoUpdate && Date.now() - prefs.lastUpdateCheck > 24 * 3600 * 1000) {
      api.updateCheck().then((info) => {
        prefs.lastUpdateCheck = Date.now();
        savePrefs();
        if (info.available) note(t('upd.available', { v: info.latest }));
      }).catch(() => {});
    }
    const clock = setInterval(() => (now = new Date()), 1000 * 15);
    const un = api.onDevicesChanged(() => refresh());
    // Le téléphone (mod de télécommande) demande d'ouvrir une tuile.
    const unTile = api.onOpenTile((id) => {
      const tile = tileFor(id);
      if (tile) void openTile(tile);
    });
    // Progression du chargement d'une app : l'animation s'arrête quand sa fenêtre est affichée. Un journal est gardé
    // (window.__launchLog) pour mesurer les temps de chargement.
    const unLaunch = api.onAppLaunch((p) => {
      ((window as any).__launchLog ??= []).push({ ...p, at: Date.now() });
      // « shown » : la fenêtre vient d'être découverte ; on garde l'animation le temps de son premier rendu pour ne pas laisser
      // réapparaître l'accueil une fraction de seconde.
      if (p.phase === 'shown') window.setTimeout(endLaunch, 900);
      else if (p.phase === 'timeout' || p.phase === 'cancelled') endLaunch();
    });
    wake();
    return () => {
      clearInterval(clock);
      unLaunch.then((f) => f());
      un.then((f) => f());
      unTile.then((f) => f());
      unClosed.then((f) => f());
    };
  });

  function focusAt(row: number, col: number, silent = false) {
    if (row === focus.row && col === focus.col) return;
    focus = { row, col };
    if (!silent) play('move');
  }
  function setZone(z: string) {
    if (z === zone) return;
    zone = z;
    play('move');
  }
  const allTiles = $derived(st ? CATEGORIES.flatMap((c) => tilesOf(st!.catalog, c)) : []);
  const recentTiles = $derived(st ? recent.map((id) => findTile(st!.catalog, id)).filter((x): x is DTile => !!x && !prefs.tiles.hidden.includes(x.id)) : []);
  const phonesLabel = $derived(phones === 0 ? t('phones.none') : phones === 1 ? t('phones.one') : t('phones.many', { n: phones }));
  const hero = $derived({
    title: focused ? focused.name : t('settings.title'),
    desc: focused ? focused.desc : `${t('settings.appearance')} · ${t('settings.wallpaper')} · ${t('settings.tiles')}`,
    category: focused ? t(`cat.${focused.category}`) : '',
    color: focused?.color ?? '#2a2f3d',
    color2: focused?.color2 ?? '#12151c',
    tile: focused,
  });

  const time = $derived(now.toLocaleTimeString(locale(), { hour: '2-digit', minute: '2-digit', hour12: !prefs.clock24 }));
  const date = $derived(now.toLocaleDateString(locale(), { weekday: 'long', day: 'numeric', month: 'long' }));
  const fg = (x?: DTile) => (x?.dark_text ? '#16130a' : '#ffffff');
  const tileFor = (id: string) => (st ? findTile(st.catalog, id) : undefined);
  const menuTile = $derived(modal?.kind === 'tile' ? tileFor(modal.id) : undefined);
</script>

<svelte:window onkeydown={onKey} onmousemove={wake} />

{#if launching}<LaunchOverlay name={launching.name} color={launching.color} color2={launching.color2} slow={launching.slow} />{/if}

<div class="root">
  {#key osState.style.id}
    {#if osState.Layout}
      {@const Layout = osState.Layout}
      <div class="layout" transition:fade={{ duration: 260 }}><Layout {model} /></div>
    {/if}
  {/key}
  <WidgetDock {model} />

  {#if toast}<div class="toast" transition:fade={{ duration: 200 }}>{toast}</div>{/if}

  <!-- Fenêtres modales -->
  {#if modal && ['settings', 'pairing', 'mods', 'tile'].includes(modal.kind)}
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

  {#if modal?.kind === 'live'}<LivePage {note} onclose={() => (modal = null)} />{/if}
  {#if modal?.kind === 'media'}<MediaPage tab={modal.tab} {note} onclose={() => (modal = null)} />{/if}
  {#if modal?.kind === 'search' && st}
    <Search tiles={CATEGORIES.flatMap((c) => tilesOf(st!.catalog, c))} {note} onclose={() => (modal = null)} onopen={(x) => openTile(x)} onmedia={(tab) => (modal = { kind: 'media', tab })} />
  {/if}

  <OverlayHost />
</div>

<style>
  .root { position: fixed; inset: 0; background: var(--bg); }
  .layout { position: absolute; inset: 0; }

  .toast { position: absolute; left: 50%; bottom: 3rem; transform: translateX(-50%); padding: 0.9rem 1.8rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.95); color: #111; font-size: 1.2rem; font-weight: 600; z-index: 40; }

  .scrim { position: absolute; inset: 0; background: rgba(4, 6, 10, 0.72); backdrop-filter: blur(1.6rem); display: grid; place-items: center; z-index: 10; }
  .sheet.wide { width: 56rem; padding: 2.2rem 2.4rem; border-radius: 1.8rem; background: var(--sheet); box-shadow: 0 3rem 6rem rgba(0, 0, 0, 0.6), inset 0 0 0 0.08rem var(--glass-strong); }
</style>
