// Style « webOS » (LG, accueil 2024-2025) : colonne d'icônes à gauche (profil, notifications, réglages rapides, recherche),
// bannière pleine largeur avec bouton à contour, rangée de cartes de raccourcis colorées (zone 'sys'), rangée d'icônes
// d'applications puis rangées de contenus 16:9. Zones : 'content', 'sys', 'banner', 'top' (colonne de gauche). Inspiré, non officiel.
import { t } from '../../i18n';
import { play } from '../../sounds';
import type { DTile } from '../../tiles.svelte';
import type { HomeModel, HomeRow } from '../model.svelte';
import type { Dir, OsStyleMeta } from '../registry';
import './webos.css';
import './webos.strings';

export const SYS = ['live', 'movies', 'music', 'photos'] as const;
export const TOP = ['profile', 'alerts', 'settings', 'search'] as const;
export type Promo = { id: string; title: string; desc: string; tile?: DTile; resume?: string; pct?: number; poster?: string | null; color: string; color2: string };
/** Rangée d'icônes d'applications (la première rangée). */
export const isAppsRow = (key: string) => key === 'apps';

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
  screens: {
    settings: () => import('./webos.screens/settings.svelte'),
    search: () => import('./webos.screens/search.svelte'),
    tileMenu: () => import('./webos.screens/tileMenu.svelte'),
    pairing: () => import('./webos.screens/pairing.svelte'),
    quick: () => import('./webos.screens/quick.svelte'),
    osd: () => import('./webos.screens/osd.svelte'),
    ambient: () => import('./webos.screens/ambient.svelte'),
    toast: () => import('./webos.screens/toast.svelte'),
    launch: () => import('./webos.screens/launch.svelte'),
  },
  tokens: {
    '--bg': '#07090d',
    '--bg-rgb': '7, 9, 13',
    '--text': '#f4f7fb',
    '--muted': 'rgba(244, 247, 251, 0.66)',
    '--glass': 'rgba(255, 255, 255, 0.1)',
    '--glass-strong': 'rgba(255, 255, 255, 0.2)',
    '--sheet': 'rgba(30, 36, 56, 0.97)',
    '--field': 'rgba(0, 0, 0, 0.4)',
    '--ring': '#ffffff',
    '--accent': '#3a8cf0',
    '--r': '1rem',
    '--font': "'Noto Sans', 'Segoe UI', system-ui, sans-serif",
    '--focus-scale': '1.08',
    '--focus-ring': '0.25rem',
    '--dock-top': '3rem',
    '--dock-left': '9rem',
    '--dock-right': '4rem',
    '--dock-bottom': '2rem',
  },
  rows(base, model) {
    const by = (k: string) => base.find((r) => r.key === k);
    const seen = new Set<string>();
    const apps: DTile[] = [];
    for (const x of [...model.recent, ...model.allTiles]) if (!seen.has(x.id)) (seen.add(x.id), apps.push(x));
    const out: HomeRow[] = [];
    if (apps.length) out.push({ key: 'apps', title: '', tiles: apps });
    for (const k of ['continue', 'video', 'tv', 'music', 'library', 'games']) {
      const r = by(k);
      if (r && r.tiles.length) out.push(r);
    }
    return out;
  },
  nav(dir: Dir, model) {
    const ui = model.ui;
    const step = (key: string, max: number, d: number) => {
      const i: number = ui[key] ?? 0;
      const j = clamp(i + d, 0, max - 1);
      if (j !== i) (ui[key] = j, play('move'));
    };
    const toTop = () => {
      ui.topFocus = Math.min(ui.topFocus ?? 2, TOP.length - 1);
      model.setZone('top');
    };
    switch (model.zone) {
      case 'top':
        if (dir === 'up' || dir === 'down') step('topFocus', TOP.length, dir === 'up' ? -1 : 1);
        else if (dir === 'right') model.setZone('content');
        return true;
      case 'banner':
        if (dir === 'left' || dir === 'right') {
          const k = promosOf(model).length;
          if (k > 1) (ui.bannerIdx = ((ui.bannerIdx ?? 0) + (dir === 'left' ? k - 1 : 1)) % k, play('move'));
        } else if (dir === 'down') model.setZone('sys');
        return true;
      case 'sys':
        if (dir === 'left') {
          if ((ui.sysFocus ?? 0) === 0) toTop();
          else step('sysFocus', SYS.length, -1);
        } else if (dir === 'right') step('sysFocus', SYS.length, 1);
        else if (dir === 'up') model.setZone('banner');
        else if (dir === 'down') (model.setZone('content'), model.focusAt(0, 0, true));
        return true;
    }
    if (dir === 'left' && model.focus.col === 0) {
      toTop();
      return true;
    }
    if (dir === 'up' && model.focus.row <= 0) {
      model.setZone('sys');
      return true;
    }
    return false;
  },
  activate(model) {
    const ui = model.ui;
    switch (model.zone) {
      case 'top': {
        const id = TOP[ui.topFocus ?? 0];
        play('select');
        if (id === 'search') model.openSearch();
        else if (id === 'settings') model.openQuick();
        else if (id === 'alerts') model.note(t('os.webos.nonotif'));
        else model.note(model.phonesLabel);
        return true;
      }
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
        else if (id === 'movies') model.openMedia('movies');
        else if (id === 'music') model.openMedia('music');
        else model.openMedia('photos');
        return true;
      }
    }
    return false;
  },
};
export default meta;
