// Style « webOS » : bannière rotative en haut (zone 'banner'), icônes d'état en haut à droite (zone 'top'),
// lanceur de cartes larges en bas (cartes système en zone 'sys', puis applications en zone 'content'). Inspiré, non officiel.
import { play } from '../../sounds';
import type { DTile } from '../../tiles.svelte';
import type { HomeModel, HomeRow } from '../model.svelte';
import type { Dir, OsStyleMeta } from '../registry';
import './webos.css';
import './webos.strings';

export const SYS = ['live', 'library', 'search', 'settings'] as const;
export const TOP = ['search', 'settings', 'profile'] as const;
export type Promo = { id: string; title: string; desc: string; tile?: DTile; resume?: string; pct?: number; poster?: string | null; color: string; color2: string };

/** Contenus de la bannière : reprises de lecture d'abord, puis quelques tuiles vidéo / séries / médiathèque / musique. */
export function promosOf(model: HomeModel): Promo[] {
  const out: Promo[] = [];
  for (const e of model.continueEntries.slice(0, 3)) {
    out.push({ id: `c:${e.url}`, title: e.title, desc: '', resume: e.url, pct: e.duration > 0 ? Math.min(100, Math.round((e.position / e.duration) * 100)) : 0, poster: e.poster, color: '#3a5bd9', color2: '#1b2250' });
  }
  const seen = new Set<string>();
  for (const x of [...model.recent, ...model.allTiles]) {
    if (out.length >= 6) break;
    if (seen.has(x.id) || !['video', 'tv', 'library', 'music'].includes(x.category)) continue;
    seen.add(x.id);
    out.push({ id: `t:${x.id}`, title: x.name, desc: x.desc, tile: x, color: x.color, color2: x.color2 });
  }
  return out;
}

const clamp = (n: number, a: number, b: number) => Math.max(a, Math.min(b, n));

const meta: OsStyleMeta = {
  id: 'webos',
  nameKey: 'os.webos',
  descKey: 'os.webos.d',
  font: 'Noto Sans',
  slots: ['status'],
  tokens: {
    '--bg': '#0d1220',
    '--bg-rgb': '13, 18, 32',
    '--text': '#f4f7fb',
    '--muted': 'rgba(244, 247, 251, 0.66)',
    '--glass': 'rgba(255, 255, 255, 0.1)',
    '--glass-strong': 'rgba(255, 255, 255, 0.2)',
    '--sheet': 'rgba(24, 31, 50, 0.97)',
    '--field': 'rgba(0, 0, 0, 0.36)',
    '--ring': '#ffffff',
    '--accent': '#6db3ff',
    '--r': '1.1rem',
    '--font': "'Noto Sans', 'Segoe UI', system-ui, sans-serif",
    '--focus-scale': '1.1',
    '--focus-ring': '0.28rem',
    '--dock-top': '5.6rem',
    '--dock-left': '4rem',
    '--dock-right': '4rem',
    '--dock-bottom': '16.5rem',
  },
  rows(base, model) {
    void base;
    const tiles = model.allTiles;
    return tiles.length ? ([{ key: 'apps', title: '', tiles }] as HomeRow[]) : [];
  },
  nav(dir: Dir, model) {
    const ui = model.ui;
    const step = (key: string, max: number, d: number) => {
      const i: number = ui[key] ?? 0;
      const j = clamp(i + d, 0, max - 1);
      if (j !== i) (ui[key] = j, play('move'));
    };
    const n = model.rows[0]?.tiles.length ?? 0;
    const toBanner = () => model.setZone('banner');
    switch (model.zone) {
      case 'top':
        if (dir === 'left' || dir === 'right') step('topFocus', TOP.length, dir === 'left' ? -1 : 1);
        else if (dir === 'down') toBanner();
        return true;
      case 'banner':
        if (dir === 'left' || dir === 'right') {
          const k = promosOf(model).length;
          if (k > 1) (ui.bannerIdx = ((ui.bannerIdx ?? 0) + (dir === 'left' ? k - 1 : 1)) % k, play('move'));
        } else if (dir === 'up') (ui.topFocus = 1, model.setZone('top'));
        else model.setZone('content');
        return true;
      case 'sys':
        if (dir === 'left') step('sysFocus', SYS.length, -1);
        else if (dir === 'right') {
          if ((ui.sysFocus ?? 0) >= SYS.length - 1) {
            if (n) model.setZone('content');
          } else step('sysFocus', SYS.length, 1);
        } else if (dir === 'up') toBanner();
        return true;
    }
    const c = model.focus.col;
    if (dir === 'left') {
      if (c > 0) model.focusAt(0, c - 1);
      else (ui.sysFocus = SYS.length - 1, model.setZone('sys'));
    } else if (dir === 'right') {
      if (c < n - 1) model.focusAt(0, c + 1);
    } else if (dir === 'up') toBanner();
    return true;
  },
  activate(model) {
    const ui = model.ui;
    switch (model.zone) {
      case 'top':
        play('select');
        if ((ui.topFocus ?? 0) === 0) model.openSearch();
        else if ((ui.topFocus ?? 0) === 1) model.openSettings();
        else model.note(model.phonesLabel);
        return true;
      case 'banner': {
        const p = promosOf(model)[ui.bannerIdx ?? 0];
        play('select');
        if (p?.resume) model.resume(p.resume);
        else if (p?.tile) model.open(p.tile);
        return true;
      }
      case 'sys': {
        const id = SYS[ui.sysFocus ?? 0];
        play('select');
        if (id === 'live') model.openLive();
        else if (id === 'library') model.openMedia('movies');
        else if (id === 'search') model.openSearch();
        else model.openSettings();
        return true;
      }
    }
    return false;
  },
};
export default meta;
