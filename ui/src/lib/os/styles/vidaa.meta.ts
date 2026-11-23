// Style « VIDAA » (téléviseurs Hisense) : bande d'état et raccourcis en haut (zone 'top'), bannière promotionnelle, rangées de grandes tuiles colorées. Inspiré, non officiel.
import { t } from '../../i18n';
import { play } from '../../sounds';
import type { HomeRow } from '../model.svelte';
import type { OsStyleMeta } from '../registry';
import './vidaa.css';
import './vidaa.strings';

/** Raccourcis de la bande du haut (zone 'top'), de gauche à droite. */
export const SHORTCUTS = ['live', 'library', 'search', 'settings'] as const;
export type VidaaShortcut = (typeof SHORTCUTS)[number];

const by = (rows: HomeRow[], key: string) => rows.find((r) => r.key === key);

const meta: OsStyleMeta = {
  id: 'vidaa',
  nameKey: 'os.vidaa',
  descKey: 'os.vidaa.d',
  font: 'Roboto',
  slots: ['status'],
  tokens: {
    '--bg': '#06223d',
    '--bg-rgb': '6, 34, 61',
    '--text': '#ffffff',
    '--muted': 'rgba(255, 255, 255, 0.74)',
    '--glass': 'rgba(255, 255, 255, 0.12)',
    '--glass-strong': 'rgba(255, 255, 255, 0.26)',
    '--sheet': 'rgba(10, 48, 82, 0.97)',
    '--field': 'rgba(0, 0, 0, 0.32)',
    '--ring': '#ffffff',
    '--accent': '#25c2d9',
    '--r': '1.1rem',
    '--font': "'Roboto', 'Segoe UI', system-ui, sans-serif",
    '--focus-scale': '1.12',
    '--focus-ring': '0.25rem',
    '--dock-top': '6rem',
    '--dock-left': '4rem',
    '--dock-right': '4rem',
    '--dock-bottom': '1.4rem',
  },
  rows(base) {
    const cont = by(base, 'continue');
    const apps = base.flatMap((r) => (['video', 'tv', 'music', 'games'].includes(r.key) ? r.tiles : []));
    const pick = (...keys: string[]) => keys.map((k) => by(base, k)).filter((r): r is HomeRow => !!r);
    return [
      ...(apps.length ? [{ key: 'apps', title: t('vidaa.apps'), tiles: apps }] : []),
      ...(cont ? [cont] : []),
      ...pick('video', 'tv', 'music', 'library', 'games'),
    ];
  },
  nav(dir, model) {
    if (model.zone === 'top') {
      const i: number = model.ui.topFocus ?? 0;
      if (dir === 'down') model.setZone('content');
      else if (dir === 'left' || dir === 'right') {
        const n = Math.max(0, Math.min(SHORTCUTS.length - 1, i + (dir === 'left' ? -1 : 1)));
        if (n !== i) {
          model.ui.topFocus = n;
          play('move');
        }
      }
      return true;
    }
    if (dir === 'up' && model.focus.row <= 0) {
      model.ui.topFocus = model.ui.topFocus ?? 0;
      model.setZone('top');
      return true;
    }
    return false;
  },
  activate(model) {
    if (model.zone !== 'top') return false;
    const s = SHORTCUTS[model.ui.topFocus ?? 0];
    play('select');
    if (s === 'live') model.openLive();
    else if (s === 'library') model.openMedia('movies');
    else if (s === 'search') model.openSearch();
    else model.openSettings();
    return true;
  },
};
export default meta;
