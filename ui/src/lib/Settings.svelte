<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import { api, type ShellState } from './api';
  import { t } from './i18n';
  import { askConfirm, askText, overlay, pickApp, pickFile } from './overlays.svelte';
  import { ACCENTS, TILE_COLORS, prefs, resetTiles, savePrefs, type Category, type CustomTile } from './prefs.svelte';
  import { geocode } from './weather';
  import { modThemes } from './mods.svelte';
  import { scanLibrary } from './library.svelte';
  import { loadLive } from './live.svelte';

  let {
    st,
    refresh,
    onclose,
    onpairing,
    onmods,
    note,
  }: { st: ShellState | null; refresh: () => Promise<void>; onclose: () => void; onpairing: () => void; onmods: () => void; note: (m: string) => void } = $props();

  type Page = 'root' | 'appearance' | 'wallpaper' | 'tiles' | 'weather' | 'live' | 'library' | 'apps' | 'system';
  interface Row {
    label: string;
    value?: string;
    swatch?: string;
    left?: () => void | Promise<void>;
    right?: () => void | Promise<void>;
    enter?: () => void | Promise<void>;
    arrows?: boolean;
  }

  let page = $state<Page>('root');
  let idx = $state(0);
  let newCat = $state<Category>('video');
  let confirmReset = $state(false);
  let detected = $state('');

  const CATS: Category[] = ['video', 'tv', 'music', 'library', 'games'];

  function cycle<T>(arr: T[], cur: T, d: number): T {
    const i = arr.indexOf(cur);
    return arr[(i + d + arr.length) % arr.length];
  }
  const set = <K extends keyof typeof prefs>(k: K, v: (typeof prefs)[K]) => {
    prefs[k] = v;
    savePrefs();
  };
  const hash = (s: string) => [...s].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);
  const colorsFor = (seed: string) => TILE_COLORS[hash(seed) % TILE_COLORS.length];

  function choice<T extends string>(label: string, opts: T[], get: () => T, put: (v: T) => void, text: (v: T) => string): Row {
    return { label, value: text(get()), arrows: true, left: () => put(cycle(opts, get(), -1)), right: () => put(cycle(opts, get(), 1)), enter: () => put(cycle(opts, get(), 1)) };
  }

  async function addWeb() {
    const name = await askText(t('tiles.name'));
    if (!name) return;
    let url = await askText(t('tiles.url'), 'https://');
    if (!url) return;
    if (!url.includes('://')) url = 'https://' + url;
    try {
      new URL(url);
    } catch {
      return note('URL invalide');
    }
    const [color, color2] = colorsFor(name);
    const tile: CustomTile = { id: `web-${hash(name + url).toString(36)}`, name, kind: 'web', url, category: newCat, color, color2, spatnav: true };
    prefs.tiles.custom = [...prefs.tiles.custom.filter((c) => c.id !== tile.id), tile];
    savePrefs();
    note(t('tiles.added'));
  }

  async function addApp() {
    const app = await pickApp();
    if (!app) return;
    const [color, color2] = colorsFor(app.name);
    const icon = await api.appIcon(app.target);
    const tile: CustomTile = { id: `app-${hash(app.target).toString(36)}`, name: app.name, kind: 'native', target: app.target, category: newCat, color, color2, icon: icon ? `data:image/png;base64,${icon}` : undefined };
    prefs.tiles.custom = [...prefs.tiles.custom.filter((c) => c.id !== tile.id), tile];
    savePrefs();
    note(t('tiles.added'));
  }

  async function setCity() {
    const c = await askText(t('weather.city'), prefs.weather.city, t('weather.cityhint'));
    if (!c) return;
    try {
      const g = await geocode(c, prefs.lang);
      if (!g) return note(t('weather.notfound'));
      prefs.weather = { ...prefs.weather, city: g.name, lat: g.lat, lon: g.lon, enabled: true };
      savePrefs();
    } catch {
      note('Réseau indisponible');
    }
  }

  async function addLive() {
    const name = await askText(t('live.name'), '');
    if (!name) return;
    const m3u = await askText(t('live.m3u'), 'https://');
    if (!m3u) return;
    const xmltv = (await askText(t('live.xmltv'), '', t('live.xmltv'))) ?? '';
    const id = `src${Date.now().toString(36)}`;
    prefs.live.sources = [...prefs.live.sources, { id, name, m3u, xmltv }];
    prefs.live.active = id;
    savePrefs();
    loadLive(true).catch(() => {});
  }

  function removeLive(id: string) {
    prefs.live.sources = prefs.live.sources.filter((s) => s.id !== id);
    if (prefs.live.active === id) prefs.live.active = prefs.live.sources[0]?.id ?? '';
    savePrefs();
    loadLive(true).catch(() => {});
  }

  type LibKind = 'movies' | 'series' | 'photos' | 'music';
  async function addLib(kind: LibKind) {
    const p = await pickFile(t('picker.title.folder'), 'folder');
    if (!p) return;
    if (!prefs.library[kind].includes(p)) prefs.library[kind] = [...prefs.library[kind], p];
    savePrefs();
    scanLibrary(true).catch(() => {});
  }
  function removeLib(kind: LibKind, path: string) {
    prefs.library[kind] = prefs.library[kind].filter((x) => x !== path);
    savePrefs();
    scanLibrary(true).catch(() => {});
  }

  async function updateNow() {
    overlay.busy = { title: t('upd.checking'), text: '', pct: null };
    let info;
    try {
      info = await api.updateCheck();
    } catch (e) {
      overlay.busy = null;
      return note(t('upd.failed', { e: String(e) }));
    }
    overlay.busy = null;
    prefs.lastUpdateCheck = Date.now();
    savePrefs();
    if (!info.available) return note(t('upd.uptodate'));
    const ok = await askConfirm(t('upd.title', { v: info.latest }), info.notes || info.latest, info.asset ? t('upd.install') : t('upd.openpage'), t('common.cancel'));
    if (!ok) return;
    if (!info.asset) {
      await api.launchCustom({ id: 'update-page', name: 'RusTV', kind: 'web', url: info.page, spatnav: true });
      return;
    }
    overlay.busy = { title: t('upd.downloading'), text: info.latest, pct: 0 };
    const off = await api.onUpdateProgress((p) => overlay.busy && (overlay.busy.pct = p.pct));
    try {
      await api.updateInstall(info.asset); // l'installeur se lance et RusTV se ferme
    } catch (e) {
      note(t('upd.failed', { e: String(e) }));
    }
    off();
    overlay.busy = null;
  }

  async function pickWallpaper() {
    const w = prefs.wallpaper;
    const kind = w.type === 'folder' ? 'folder' : w.type === 'video' ? 'video' : 'image';
    const p = await pickFile(t(`picker.title.${kind}`), kind);
    if (p) {
      prefs.wallpaper.path = p;
      savePrefs();
    }
  }

  const rows = $derived.by<Row[]>(() => {
    const w = prefs.wallpaper;
    switch (page) {
      case 'root':
        return [
          { label: t('settings.appearance'), enter: () => go('appearance') },
          { label: t('settings.wallpaper'), enter: () => go('wallpaper') },
          { label: t('settings.tiles'), enter: () => go('tiles') },
          { label: t('settings.weather'), enter: () => go('weather') },
          { label: t('settings.live'), enter: () => go('live') },
          { label: t('settings.library'), enter: () => go('library') },
          { label: t('settings.mods'), enter: onmods },
          { label: t('settings.phones'), enter: onpairing },
          { label: t('settings.apps'), enter: () => go('apps') },
          { label: t('settings.screens'), enter: () => go('system') },
        ];
      case 'appearance':
        return [
          choice(t('settings.language'),['fr', 'en'] as const, () => prefs.lang, (v) => set('lang', v), (v) => (v === 'fr' ? 'Français' : 'English')),
          choice(t('settings.theme'), ['dark', 'light', 'auto'] as const, () => prefs.theme, (v) => set('theme', v), (v) => t(`theme.${v}`)),
          { ...choice(t('settings.accent'), ACCENTS, () => prefs.accent, (v) => set('accent', v), () => ''), swatch: prefs.accent },
          ...(modThemes().length
            ? [choice(t('settings.modtheme'), ['', ...modThemes().map((x) => x.key)], () => prefs.modTheme, (v) => set('modTheme', v), (v) => (v ? (modThemes().find((x) => x.key === v)?.name ?? v) : t('common.none')))]
            : []),
          choice(t('settings.tint'), ['yes', 'no'] as const, () => (prefs.tint ? 'yes' : 'no'), (v) => set('tint', v === 'yes'), (v) => t(`common.${v}`)),
          choice(t('settings.size'), ['80', '90', '100', '110', '125', '140'], () => String(prefs.uiScale), (v) => ((prefs.uiScale = +v), savePrefs()), (v) => ` %`),
          choice(t('settings.clock24'), ['24', '12'] as const, () => (prefs.clock24 ? '24' : '12'), (v) => set('clock24', v === '24'), (v) => t(`clock.${v}`)),
        ];
      case 'wallpaper': {
        const r: Row[] = [
          choice(t('wp.type'), ['aurora', 'color', 'image', 'folder', 'video'] as const, () => w.type, (v) => ((prefs.wallpaper.type = v), savePrefs()), (v) => t(`wp.${v}`)),
        ];
        if (w.type === 'image' || w.type === 'folder' || w.type === 'video') {
          r.push({ label: t(w.type === 'folder' ? 'wp.folderpath' : 'wp.file'), value: w.path ? w.path.split('\\').slice(-2).join('\\') : t('wp.choose'), enter: pickWallpaper });
        }
        if (w.type === 'color') {
          const cols = ['#0b0e14', '#101828', '#1a1033', '#2a0f14', '#0f2a1f', '#2b2b2b', '#e9edf5'];
          r.push({ ...choice(t('wp.color.label'), cols, () => w.color, (v) => ((prefs.wallpaper.color = v), savePrefs()), () => ''), swatch: w.color });
        }
        if (w.type !== 'aurora' && w.type !== 'color') {
          r.push(choice(t('wp.blur'), [0, 4, 8, 12, 20, 30].map(String), () => String(w.blur), (v) => ((prefs.wallpaper.blur = +v), savePrefs()), (v) => `${v}`));
          r.push(choice(t('wp.dim'), [0, 15, 30, 45, 60, 80].map(String), () => String(w.dim), (v) => ((prefs.wallpaper.dim = +v), savePrefs()), (v) => `${v} %`));
        }
        if (w.type === 'folder') r.push(choice(t('wp.interval'), [10, 20, 30, 60, 120].map(String), () => String(w.interval), (v) => ((prefs.wallpaper.interval = +v), savePrefs()), (v) => `${v} s`));
        return r;
      }
      case 'tiles':
        return [
          { label: t('tiles.addweb'), enter: addWeb },
          { label: t('tiles.addapp'), enter: addApp },
          choice(t('tiles.newcat'), CATS, () => newCat, (v) => (newCat = v), (v) => t(`cat.${v}`)),
          {
            label: t('tiles.hidden', { n: prefs.tiles.hidden.length }),
            enter: () => {
              prefs.tiles.hidden = [];
              savePrefs();
            },
          },
          {
            label: confirmReset ? t('tiles.reset.confirm') : t('tiles.reset'),
            enter: () => {
              if (confirmReset) {
                resetTiles();
                confirmReset = false;
              } else confirmReset = true;
            },
          },
        ];
      case 'weather':
        return [
          choice(t('weather.show'), ['yes', 'no'] as const, () => (prefs.weather.enabled ? 'yes' : 'no'), (v) => {
            if (v === 'yes' && prefs.weather.lat == null) setCity();
            else {
              prefs.weather.enabled = v === 'yes';
              savePrefs();
            }
          }, (v) => t(`common.${v}`)),
          { label: t('weather.city'), value: prefs.weather.city || '—', enter: setCity },
          choice(t('weather.unit'), ['c', 'f'] as const, () => prefs.weather.unit, (v) => ((prefs.weather.unit = v), savePrefs()), (v) => (v === 'c' ? '°C' : '°F')),
        ];
      case 'live': {
        const r: Row[] = prefs.live.sources.map((s) => ({
          label: s.name,
          value: prefs.live.active === s.id ? '✓ ' + s.m3u.slice(0, 36) : s.m3u.slice(0, 36),
          enter: () => {
            prefs.live.active = s.id;
            savePrefs();
            loadLive(true).catch(() => {});
          },
        }));
        r.push({ label: t('live.add'), enter: addLive });
        for (const s of prefs.live.sources) r.push({ label: t('live.remove.named', { name: s.name }), enter: () => removeLive(s.id) });
        return r;
      }
      case 'library': {
        const r: Row[] = [];
        for (const k of ['movies', 'series', 'photos', 'music'] as LibKind[]) {
          r.push({ label: t(`lib.${k}`), value: `${prefs.library[k].length}`, enter: () => addLib(k) });
          for (const p of prefs.library[k]) r.push({ label: t('lib.remove', { path: p.split('\\').slice(-2).join('\\') }), enter: () => removeLib(k, p) });
        }
        return r;
      }
      case 'apps':
        return [
          { label: t('apps.detected'), value: detected || t('apps.none') },
          choice(t('apps.browser'), ['auto', 'builtin', 'brave', 'vivaldi'] as const, () => prefs.youtubeBrowser, (v) => set('youtubeBrowser', v), (v) => t(`apps.opt.${v}`)),
          { label: t('apps.note') },
        ];
      case 'system':
        return [
          choice(t('sys.blackout'), ['yes', 'no'] as const, () => (st?.settings.blackout ? 'yes' : 'no'), async (v) => {
            await api.setBlackout(v === 'yes');
            await refresh();
          }, (v) => t(`common.${v}`)),
          choice(t('sys.autostart'), ['off', 'tray', 'tv'] as const, () => st?.autostart ?? 'off', async (v) => {
            await api.setAutostart(v);
            await refresh();
          }, (v) => t(`sys.auto.${v}`)),
          choice(t('sys.autoupdate'), ['yes', 'no'] as const, () => (prefs.autoUpdate ? 'yes' : 'no'), (v) => ((prefs.autoUpdate = v === 'yes'), savePrefs()), (v) => t(`common.${v}`)),
          { label: t('sys.update', { v: st?.version ?? '' }), enter: updateNow },
          { label: t('sys.screen'), enter: async () => (onclose(), await api.tvExit(), await api.tvStart()) },
          { label: t('sys.exit'), enter: () => api.tvExit() },
          { label: t('sys.quit'), enter: () => api.quit() },
        ];
    }
  });

  const titles: Record<Page, string> = { root: 'settings.title', appearance: 'settings.appearance', wallpaper: 'settings.wallpaper', tiles: 'settings.tiles', weather: 'settings.weather', live: 'settings.live', library: 'settings.library', apps: 'settings.apps', system: 'settings.screens' };

  function go(p: Page) {
    page = p;
    idx = 0;
    if (p === 'apps') void api.browsersDetected().then((l) => (detected = l.map((b) => (b.kind === 'brave' ? 'Brave' : 'Vivaldi')).join(', '))).catch(() => {});
    confirmReset = false;
  }

  function onKey(e: KeyboardEvent) {
    const r = rows[idx];
    if (e.key === 'ArrowDown') idx = Math.min(rows.length - 1, idx + 1);
    else if (e.key === 'ArrowUp') idx = Math.max(0, idx - 1);
    else if (e.key === 'ArrowLeft') r?.left?.();
    else if (e.key === 'ArrowRight') r?.right?.();
    else if (e.key === 'Enter') r?.enter?.();
    else if (e.key === 'Escape' || e.key === 'Backspace') {
      if (page === 'root') onclose();
      else go('root');
    } else return;
    if (idx >= rows.length) idx = rows.length - 1;
    e.preventDefault();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="sheet" in:fly={{ y: 30, duration: 360, easing: cubicOut }}>
  <h3>{t(titles[page])}</h3>
  <div class="rows">
    {#each rows as r, i}
      <button class="item" class:sel={i === idx} onmouseenter={() => (idx = i)} onclick={() => r.enter?.()}>
        <span class="lb">{r.label}</span>
        <span class="val">
          {#if r.swatch}<i style:background={r.swatch}></i>{/if}
          {#if r.arrows}<b>◀</b>{/if}{r.value ?? ''}{#if r.arrows}<b>▶</b>{/if}
        </span>
      </button>
    {/each}
  </div>
  <p class="hint">{t('settings.hint')}</p>
</div>

<style>
  .sheet { width: 46rem; padding: 2.2rem 2.4rem; border-radius: 1.8rem; background: var(--sheet); box-shadow: 0 3rem 6rem rgba(0, 0, 0, 0.55), inset 0 0 0 0.08rem var(--glass-strong); color: var(--text); }
  h3 { margin: 0 0 1.1rem; font-size: 1.9rem; }
  .rows { max-height: 34rem; overflow: hidden auto; scrollbar-width: none; }
  .item { display: flex; justify-content: space-between; align-items: center; gap: 1.5rem; width: 100%; text-align: left; margin: 0.25rem 0; padding: 0.95rem 1.2rem; border: 0; border-radius: 0.9rem; background: transparent; color: var(--text); font-family: inherit; font-size: 1.32rem; transition: background 0.2s, transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .item.sel { background: var(--accent); color: #fff; font-weight: 600; transform: scale(1.02); }
  .val { display: flex; align-items: center; gap: 0.6rem; opacity: 0.9; font-size: 1.2rem; white-space: nowrap; max-width: 20rem; overflow: hidden; text-overflow: ellipsis; }
  .val b { font-size: 0.8rem; opacity: 0.6; }
  .val i { width: 1.4rem; height: 1.4rem; border-radius: 50%; box-shadow: 0 0 0 0.12rem rgba(255, 255, 255, 0.7); }
  .hint { margin: 1.2rem 0 0; font-size: 0.95rem; color: var(--muted); }
</style>
