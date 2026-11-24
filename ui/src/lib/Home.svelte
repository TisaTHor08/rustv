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
  import QuickHost from './os/QuickHost.svelte';
  import ScreenSlot from './os/ScreenSlot.svelte';
  import SystemLayer from './os/SystemLayer.svelte';
  import LivePage from './LivePage.svelte';
  import MediaPage from './MediaPage.svelte';
  import LaunchOverlay from './LaunchOverlay.svelte';
  import { continueList, continueRemove, playMedia, type ContinueEntry } from './player.svelte';
  import './continue.strings';
  import Search from './Search.svelte';
  import ModManager from './ModManager.svelte';
  import WidgetDock from './os/WidgetDock.svelte';
  import { osState } from './os/apply.svelte';
  import { clampFocus, minRow, validZone } from './os/navguard';
  import type { HomeModel, HomeRow } from './os/model.svelte';
  import { mods, pageSrc, type ModTile } from './mods.svelte';
  import PageHost from './PageHost.svelte';
  import Pairing from './Pairing.svelte';
  import { prefs, savePrefs } from './prefs.svelte';
  import { play } from './sounds';
  import Settings from './Settings.svelte';
  import TileMenu from './TileMenu.svelte';
  import { CATEGORIES, findTile, iconSrc, moveTile, tilesOf, type BuiltinTile, type DTile } from './tiles.svelte';

  type Modal = null | { kind: 'settings'; page?: string } | { kind: 'quick' } | { kind: 'pairing' } | { kind: 'mods' } | { kind: 'live' } | { kind: 'search' } | { kind: 'media'; tab: 'movies' | 'series' | 'photos' | 'music' | 'files' } | { kind: 'page'; modId: string; entry: string; title: string } | { kind: 'tile'; id: string };

  let st = $state<ShellState | null>(null);
  let sys = $state<{ startAmbient(): void; showVolume(l: number, m: boolean): void } | undefined>();
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
    openSettings: (page) => (modal = { kind: 'settings', page }),
    openQuick: () => (modal = { kind: 'quick' }),
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
    const f = clampFocus(osState.style.id, rows.map((r) => r.tiles.length), focus);
    if (f.row !== focus.row || f.col !== focus.col) focus = f;
  });

  // Garde-fou : ramène zone et focus à des valeurs valides pour le style actif (zone inconnue du style, rangée -1 hors RusTV...).
  function sanitizeNav() {
    const z = validZone(zone, osState.style.zones);
    if (z !== zone) zone = z;
    const f = clampFocus(osState.style.id, rows.map((r) => r.tiles.length), focus);
    if (f.row !== focus.row || f.col !== focus.col) focus = f;
  }

  // Changement de style : l'état de navigation de l'ancien style (zone, onglet, index de barre) n'a plus de sens pour le nouveau.
  let navStyle = osState.style.id;
  $effect(() => {
    const id = osState.style.id;
    if (id === navStyle) return;
    navStyle = id;
    zone = 'content';
    tilt = '';
    for (const k of Object.keys(ui)) delete ui[k];
    focus = { row: 0, col: 0 };
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
      const nr = Math.max(minRow(osState.style.id), Math.min(rows.length - 1, row + dr));
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
  let launchMax: number | undefined;
  function endLaunch() {
    clearTimeout(launchTimer);
    clearTimeout(launchMax);
    launching = null;
  }
  function beginLaunch(tile: DTile) {
    launching = { id: tile.id, name: tile.name, color: tile.color, color2: tile.color2, slow: false };
    clearTimeout(launchTimer);
    clearTimeout(launchMax);
    launchTimer = window.setTimeout(() => launching && (launching.slow = true), 8000);
    // Sécurité : l'animation ne reste jamais plus de 40 s.
    launchMax = window.setTimeout(endLaunch, 40000);
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
        r = await api.launchCustom({ id: raw.id, name: tile.name, kind: raw.kind, url: raw.url, target: raw.target, spatnav: raw.spatnav, googleSignIn: raw.googleSignIn });
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
        runningApp = web ? { id: tile.id, at: Date.now() } : null;
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
        // Hors d'une tuile (icône Réglages, barre d'onglets, menu latéral) : panneau de réglages rapides.
        if (zone !== 'content' || focus.row < 0 || !focused) modal = { kind: 'quick' };
        else if (focused?.id.startsWith('continue:')) continueRemove(focused.id.slice(9)).then(loadCont);
        else if (focused) modal = { kind: 'tile', id: focused.id };
        break;
      default: return;
    }
    e.preventDefault();
  }

  // ---------- reprise (sauvegarde d'état, 3 h) ----------
  // On mémorise la page ouverte, la tuile sélectionnée et l'app web en cours. Rouverte dans les 3 h, la TV y retombe (réglable).
  let runningApp = $state<{ id: string; at: number } | null>(null);
  let sessionReady = false;
  const sessionSnap = $derived.by(() => {
    const m = modal;
    let page: Record<string, string> | null = null;
    if (m?.kind === 'settings') page = { kind: 'settings', page: m.page ?? '' };
    else if (m?.kind === 'live' || m?.kind === 'search' || m?.kind === 'mods') page = { kind: m.kind };
    else if (m?.kind === 'media') page = { kind: 'media', tab: m.tab };
    else if (m?.kind === 'page') page = { kind: 'page', modId: m.modId, entry: m.entry, title: m.title };
    return JSON.stringify({ style: osState.style.id, tile: focused?.id ?? null, page, app: runningApp?.id ?? null });
  });
  let saveTimer: number | undefined;
  $effect(() => {
    const snap = sessionSnap;
    const on = prefs.resume;
    if (!sessionReady) return;
    clearTimeout(saveTimer);
    saveTimer = window.setTimeout(() => void (on ? api.sessionSave(JSON.parse(snap)) : api.sessionClear()).catch(() => {}), 400);
  });

  async function restoreSession() {
    try {
      if (!prefs.resume) return;
      const s = await api.sessionLoad();
      if (!s) return;
      await tick();
      // même style d'interface : on retrouve la tuile ; sinon l'accueil reste au début
      if (s.tile && s.style === osState.style.id) {
        for (let r = 0; r < rows.length; r++) {
          const c = rows[r].tiles.findIndex((x) => x.id === s.tile);
          if (c >= 0) {
            focus = { row: r, col: c };
            break;
          }
        }
      }
      const p = s.page;
      if (p?.kind === 'settings') modal = { kind: 'settings', page: p.page || undefined };
      else if (p?.kind === 'live' || p?.kind === 'search' || p?.kind === 'mods') modal = { kind: p.kind };
      else if (p?.kind === 'media' && ['movies', 'series', 'photos', 'music', 'files'].includes(p.tab)) modal = { kind: 'media', tab: p.tab };
      else if (p?.kind === 'page' && typeof p.modId === 'string' && mods.list.some((x) => x.id === p.modId)) modal = { kind: 'page', modId: p.modId, entry: p.entry || 'index.html', title: String(p.title ?? '') };
      const tile = s.app ? tileFor(String(s.app)) : undefined;
      if (tile && isWebTile(tile)) {
        note(t('os.resume.app', { name: tile.name }));
        await openTile(tile);
      }
    } catch {
    } finally {
      sessionReady = true;
    }
  }

  // ---------- cycle de vie ----------

  let idleTimer: number | undefined;
  function wake() {
    document.documentElement.classList.remove('idle');
    clearTimeout(idleTimer);
    idleTimer = window.setTimeout(() => document.documentElement.classList.add('idle'), 3000);
  }

  // Retour de la fenêtre au premier plan (fin d'une app web, du lecteur, de la télécommande...) : on ne fait pas confiance à l'état
  // d'avant. Animation de lancement terminée, zone et focus revalidés, élément du DOM resté actif relâché (sinon Entrée le « clique »).
  function resync() {
    if (document.visibilityState === 'hidden') return;
    endLaunch();
    // Retour à l'accueil (hors du lancement lui-même, qui peut aussi donner un focus) : plus d'app en cours à reprendre.
    if (runningApp && Date.now() - runningApp.at > 8000) runningApp = null;
    sanitizeNav();
    if (zone !== 'content' && !modal) zone = 'content';
    const a = document.activeElement as HTMLElement | null;
    if (a && a !== document.body && !/^(INPUT|TEXTAREA|SELECT)$/.test(a.tagName)) a.blur();
    wake();
  }
  
  // Une fenêtre modale qui se ferme ne doit pas laisser un élément actif (bouton cliqué à la souris) capter Entrée.
  $effect(() => {
    if (modal) return;
    const a = document.activeElement as HTMLElement | null;
    if (a && a !== document.body && !/^(INPUT|TEXTAREA|SELECT)$/.test(a.tagName)) a.blur();
  });
  // Un menu de tuile dont la tuile a disparu (masquée, supprimée) ne doit pas garder la capture des touches.
  $effect(() => {
    if (modal?.kind === 'tile' && !menuTile) modal = null;
    else if (modal?.kind === 'search' && !st) modal = null;
  });

  onMount(() => {
    const onWinFocus = () => resync();
    const onVis = () => resync();
    window.addEventListener('focus', onWinFocus);
    window.addEventListener('pageshow', onWinFocus);
    document.addEventListener('visibilitychange', onVis);
    refresh().then(restoreSession, () => (sessionReady = true));
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
    // Harnais de maquette (ui/mock.html) : permet d'ouvrir chaque écran sans touches. Absent en production.
    if ((window as any).__RUSTV_MOCK__) {
      (window as any).__rustvHome = {
        model,
        setStyle: (id: string) => (prefs.osStyle = id as any),
        show: (k: string) => {
          if (k === 'settings') modal = { kind: 'settings' };
          else if (k === 'display') modal = { kind: 'settings', page: 'display' };
          else if (k === 'search') modal = { kind: 'search' };
          else if (k === 'pairing') modal = { kind: 'pairing' };
          else if (k === 'quick') modal = { kind: 'quick' };
          else if (k === 'tilemenu' && focused) modal = { kind: 'tile', id: focused.id };
          else if (k === 'ambient') sys?.startAmbient();
          else if (k === 'osd') sys?.showVolume(42, false);
          else if (k === 'mute') sys?.showVolume(42, true);
          else if (k === 'toast') note('Exemple de message');
          else if (k === 'launch') beginLaunch(rows.flatMap((r) => r.tiles)[0]);
          else if (k === 'resync') resync();
        },
      };
    }
    return () => {
      window.removeEventListener('focus', onWinFocus);
      window.removeEventListener('pageshow', onWinFocus);
      document.removeEventListener('visibilitychange', onVis);
      clearTimeout(idleTimer);
      clearTimeout(launchTimer);
      clearTimeout(launchMax);
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

{#if launching}
  <ScreenSlot name="launch" props={{ name: launching.name, color: launching.color, color2: launching.color2, slow: launching.slow, opening: t('launch.opening', { name: launching.name }), cancelHint: launching.slow ? t('launch.slow') : t('launch.cancel') }}>
    {#snippet fallback()}<LaunchOverlay name={launching!.name} color={launching!.color} color2={launching!.color2} slow={launching!.slow} />{/snippet}
  </ScreenSlot>
{/if}

<div class="root">
  {#key osState.style.id}
    {#if osState.Layout}
      {@const Layout = osState.Layout}
      <div class="layout" transition:fade={{ duration: 260 }}><Layout {model} /></div>
    {/if}
  {/key}
  <WidgetDock {model} />

  <SystemLayer bind:this={sys} {model} {toast} blocked={!!modal || anyOverlay() || !!launching} />

  <!-- Fenêtres modales -->
  {#if modal && ['settings', 'pairing', 'mods', 'tile'].includes(modal.kind)}
    <div class="scrim" transition:fade={{ duration: 180 }}>
      {#if modal.kind === 'settings'}
        <Settings {st} {refresh} initial={modal.kind === 'settings' ? modal.page : undefined} {note} onclose={() => (modal = null)} onpairing={() => (modal = { kind: 'pairing' })} onmods={() => (modal = { kind: 'mods' })} />
      {:else if modal.kind === 'pairing'}
        {#if osState.style.screens?.pairing}
          <Pairing embedded onclose={() => (modal = { kind: 'settings' })} />
        {:else}
          <div class="sheet wide" in:fly={{ y: 30, duration: 360, easing: cubicOut }}>
            <Pairing embedded onclose={() => (modal = { kind: 'settings' })} />
          </div>
        {/if}
      {:else if modal.kind === 'mods'}
        <ModManager {note} onclose={() => (modal = { kind: 'settings' })} />
      {:else if modal.kind === 'tile' && menuTile && st}
        <TileMenu tile={menuTile} onclose={() => (modal = null)} onopen={() => openTile(menuTile)} onmove={(d) => onMove(menuTile.id, d)} />
      {/if}
    </div>
  {/if}

  {#if modal?.kind === 'quick'}
    <QuickHost time={time} {date} {phonesLabel} onclose={() => (modal = null)} onsettings={(p) => (modal = { kind: 'settings', page: p })} onpairing={() => (modal = { kind: 'pairing' })} />
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

  .scrim { position: absolute; inset: 0; background: rgba(4, 6, 10, 0.72); backdrop-filter: blur(1.6rem); display: grid; place-items: center; z-index: 10; }
  .sheet.wide { width: 56rem; padding: 2.2rem 2.4rem; border-radius: 1.8rem; background: var(--sheet); box-shadow: 0 3rem 6rem rgba(0, 0, 0, 0.6), inset 0 0 0 0.08rem var(--glass-strong); }
</style>
