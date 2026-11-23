// Style « Titan » (téléviseurs Philips / TP Vision) : barre d'icônes en haut (zone 'nav'), héros carrousel, rangée d'applis carrées, rails. Inspiré, non officiel.
import { t } from '../../i18n';
import { play } from '../../sounds';
import type { HomeRow } from '../model.svelte';
import type { OsStyleMeta } from '../registry';
import './titan.css';
import './titan.strings';

export type TitanEntry = { id: string; kind: 'tab' | 'action' };
/** Entrées de la barre du haut, de gauche à droite (l'engrenage est rendu à droite). */
export const NAV: TitanEntry[] = [
  { id: 'search', kind: 'action' },
  { id: 'home', kind: 'tab' },
  { id: 'apps', kind: 'tab' },
  { id: 'live', kind: 'action' },
  { id: 'library', kind: 'tab' },
  { id: 'settings', kind: 'action' },
];
export const GRID_COLS = 10;

const tabOf = (m: { ui: Record<string, any> }): string => m.ui.tab ?? 'home';
const by = (rows: HomeRow[], key: string) => rows.find((r) => r.key === key);
const chunk = (tiles: HomeRow['tiles'], key: string, n: number): HomeRow[] => {
  const out: HomeRow[] = [];
  for (let i = 0; i < tiles.length; i += n) out.push({ key: `${key}${i / n}`, title: '', tiles: tiles.slice(i, i + n) });
  return out;
};
const navIndexOfTab = (id: string) => Math.max(0, NAV.findIndex((e) => e.id === id));

const meta: OsStyleMeta = {
  id: 'titan',
  nameKey: 'os.titan',
  descKey: 'os.titan.d',
  font: 'Poppins',
  slots: ['status'],
  /** Zones de focus que ce style sait gérer (voir navguard.ts) : toute autre zone est ramenée à 'content'. */
  zones: ['content', 'nav'],
  screens: {
    settings: () => import('./titan.screens/settings.svelte'),
    search: () => import('./titan.screens/search.svelte'),
    tileMenu: () => import('./titan.screens/tileMenu.svelte'),
    pairing: () => import('./titan.screens/pairing.svelte'),
    quick: () => import('./titan.screens/quick.svelte'),
    osd: () => import('./titan.screens/osd.svelte'),
    ambient: () => import('./titan.screens/ambient.svelte'),
    toast: () => import('./titan.screens/toast.svelte'),
    launch: () => import('./titan.screens/launch.svelte'),
  },
  tokens: {
    '--bg': '#0d0b1d',
    '--bg-rgb': '13, 11, 29',
    '--text': '#f4f2ff',
    '--muted': 'rgba(244, 242, 255, 0.68)',
    '--glass': 'rgba(255, 255, 255, 0.1)',
    '--glass-strong': 'rgba(255, 255, 255, 0.22)',
    '--sheet': 'rgba(24, 20, 52, 0.97)',
    '--field': 'rgba(0, 0, 0, 0.38)',
    '--ring': '#dfe7ff',
    '--accent': '#6f7bff',
    '--r': '0.9rem',
    '--font': "'Poppins', 'Segoe UI', system-ui, sans-serif",
    '--focus-scale': '1.1',
    '--focus-ring': '0.2rem',
    '--dock-top': '6.2rem',
    '--dock-left': '5rem',
    '--dock-right': '5rem',
    '--dock-bottom': '1.4rem',
  },
  rows(base, model) {
    const pick = (...keys: string[]) => keys.map((k) => by(base, k)).filter((r): r is HomeRow => !!r);
    const cont = by(base, 'continue');
    switch (tabOf(model)) {
      case 'apps':
        return chunk(model.allTiles, 'g', GRID_COLS);
      case 'library':
        return pick('library', 'music');
      default:
        return [{ key: 'apps', title: t('titan.fav'), tiles: model.allTiles }, ...(cont ? [cont] : []), ...pick('video', 'tv', 'music', 'library', 'games')];
    }
  },
  nav(dir, model) {
    if (model.zone === 'nav') {
      const i: number = model.ui.navFocus ?? navIndexOfTab(tabOf(model));
      if (dir === 'down') model.setZone('content');
      else if (dir === 'left' || dir === 'right') {
        const n = Math.max(0, Math.min(NAV.length - 1, i + (dir === 'left' ? -1 : 1)));
        if (n !== i) {
          model.ui.navFocus = n;
          play('move');
        }
      }
      return true;
    }
    if (dir === 'up' && model.focus.row <= 0) {
      model.ui.navFocus = navIndexOfTab(tabOf(model));
      model.setZone('nav');
      return true;
    }
    return false;
  },
  activate(model) {
    if (model.zone !== 'nav') return false;
    const e = NAV[model.ui.navFocus ?? navIndexOfTab(tabOf(model))];
    play('select');
    if (e.id === 'search') model.openSearch();
    else if (e.id === 'live') model.openLive();
    else if (e.id === 'settings') model.openSettings();
    else {
      model.ui.tab = e.id;
      model.focusAt(0, 0, true);
      model.setZone('content');
    }
    return true;
  },
};
export default meta;
