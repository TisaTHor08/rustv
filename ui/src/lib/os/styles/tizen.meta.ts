// Style « Tizen » : barre de lancement en bas (groupe de boutons ronds + tuiles), aperçus au-dessus de la barre.
// Zones : 'content' (tuiles), 'sys' (boutons ronds), 'preview' (aperçus), 'top' (cloche, profil), 'ambient'. Inspiré, non officiel.
import { t } from '../../i18n';
import { play } from '../../sounds';
import type { DTile } from '../../tiles.svelte';
import type { HomeModel, HomeRow } from '../model.svelte';
import type { Dir, OsStyleMeta } from '../registry';
import './tizen.css';
import './tizen.strings';

export const SYS = ['settings', 'source', 'search', 'ambient', 'menu'] as const;
export const TOP = ['bell', 'profile'] as const;
export type Preview = { id: string; title: string; sub: string; tile: DTile; resume?: string; pct?: number; poster?: string | null };
const MAX_PREVIEWS = 8;

/** Aperçus de la tuile ciblée : reprises de lecture (vidéo, séries, médiathèque) puis autres tuiles de la même catégorie. */
export function previewsOf(model: HomeModel): Preview[] {
  const f = model.focused;
  if (!f) return [];
  const out: Preview[] = [];
  if (['video', 'tv', 'library'].includes(f.category) || f.id === 'live' || f.id === 'media') {
    for (const e of model.continueEntries.slice(0, 4)) {
      out.push({ id: `c:${e.url}`, title: e.title, sub: '', tile: f, resume: e.url, pct: e.duration > 0 ? Math.min(100, Math.round((e.position / e.duration) * 100)) : 0, poster: e.poster });
    }
  }
  for (const tile of model.allTiles) {
    if (out.length >= MAX_PREVIEWS) break;
    if (tile.id !== f.id && tile.category === f.category) out.push({ id: `t:${tile.id}`, title: tile.name, sub: tile.desc, tile });
  }
  return out;
}

const clamp = (n: number, a: number, b: number) => Math.max(a, Math.min(b, n));

const meta: OsStyleMeta = {
  id: 'tizen',
  nameKey: 'os.tizen',
  descKey: 'os.tizen.d',
  font: 'Inter',
  slots: ['status', 'bar'],
  tokens: {
    '--bg': '#0a0c14',
    '--bg-rgb': '10, 12, 20',
    '--text': '#f2f4f8',
    '--muted': 'rgba(242, 244, 248, 0.62)',
    '--glass': 'rgba(255, 255, 255, 0.09)',
    '--glass-strong': 'rgba(255, 255, 255, 0.18)',
    '--sheet': 'rgba(20, 23, 34, 0.97)',
    '--field': 'rgba(0, 0, 0, 0.38)',
    '--ring': '#ffffff',
    '--accent': '#4d8dff',
    '--r': '1rem',
    '--font': "'Inter', 'Segoe UI', system-ui, sans-serif",
    '--focus-scale': '1.12',
    '--focus-ring': '0.25rem',
    '--dock-top': '5.6rem',
    '--dock-left': '3rem',
    '--dock-right': '3rem',
    '--dock-bottom': '14rem',
  },
  rows(base, model) {
    // Une seule rangée : récentes d'abord, puis toutes les tuiles (sans doublon). « Reprendre » vit dans les aperçus.
    void base;
    const seen = new Set<string>();
    const tiles: DTile[] = [];
    for (const x of [...model.recent, ...model.allTiles]) if (!seen.has(x.id)) (seen.add(x.id), tiles.push(x));
    return tiles.length ? ([{ key: 'apps', title: '', tiles }] as HomeRow[]) : [];
  },
  nav(dir: Dir, model) {
    const ui = model.ui;
    const z = model.zone;
    const n = model.rows[0]?.tiles.length ?? 0;
    const step = (key: string, max: number, d: number) => {
      const i: number = ui[key] ?? 0;
      const j = clamp(i + d, 0, max - 1);
      if (j !== i) (ui[key] = j, play('move'));
      return j !== i;
    };
    const hasPrev = () => previewsOf(model).length > 0;
    switch (z) {
      case 'ambient':
        model.setZone('content');
        return true;
      case 'sys':
        if (dir === 'left' || dir === 'right') {
          if (dir === 'right' && (ui.sysFocus ?? 0) >= SYS.length - 1) {
            if (n) model.setZone('content');
          } else step('sysFocus', SYS.length, dir === 'left' ? -1 : 1);
        } else if (dir === 'up') (ui.topFocus = 1, model.setZone('top'));
        return true;
      case 'preview':
        if (dir === 'left' || dir === 'right') step('pvFocus', previewsOf(model).length, dir === 'left' ? -1 : 1);
        else if (dir === 'up') (ui.topFocus = 1, model.setZone('top'));
        else model.setZone('content');
        return true;
      case 'top':
        if (dir === 'left' || dir === 'right') step('topFocus', TOP.length, dir === 'left' ? -1 : 1);
        else if (dir === 'down') {
          if (hasPrev()) (ui.pvFocus = 0, model.setZone('preview'));
          else model.setZone('content');
        }
        return true;
    }
    // zone 'content' : la barre d'applications
    const c = model.focus.col;
    if (dir === 'left') {
      if (c > 0) model.focusAt(0, c - 1);
      else (ui.sysFocus = SYS.length - 1, model.setZone('sys'));
    } else if (dir === 'right') {
      if (c < n - 1) model.focusAt(0, c + 1);
    } else if (dir === 'up') {
      if (hasPrev()) (ui.pvFocus = 0, model.setZone('preview'));
      else (ui.topFocus = 1, model.setZone('top'));
    }
    return true;
  },
  activate(model) {
    const ui = model.ui;
    switch (model.zone) {
      case 'ambient':
        model.setZone('content');
        return true;
      case 'sys': {
        const id = SYS[ui.sysFocus ?? 0];
        play('select');
        if (id === 'settings') model.openSettings();
        else if (id === 'source') model.openMedia('files');
        else if (id === 'search') model.openSearch();
        else if (id === 'ambient') model.setZone('ambient');
        else if (model.focused) model.contextMenu(model.focused);
        return true;
      }
      case 'preview': {
        const p = previewsOf(model)[ui.pvFocus ?? 0];
        play('select');
        if (p?.resume) model.resume(p.resume);
        else if (p) model.open(p.tile);
        return true;
      }
      case 'top':
        play('select');
        if ((ui.topFocus ?? 0) === 0) model.note(t('os.tizen.nonotif'));
        else model.openSettings();
        return true;
    }
    return false;
  },
};
export default meta;
