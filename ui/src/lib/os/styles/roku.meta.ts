// Style « Roku » : menu vertical à gauche (zone 'menu'), grille de tuiles 4 colonnes. Inspiré, non officiel.
import { play } from '../../sounds';
import type { HomeRow } from '../model.svelte';
import type { OsStyleMeta } from '../registry';
import './roku.css';
import './roku.strings';

export const COLS = 4;
export type RokuEntry = { id: string; kind: 'content' | 'action' };
export const MENU: RokuEntry[] = [
  { id: 'home', kind: 'content' },
  { id: 'search', kind: 'action' },
  { id: 'apps', kind: 'content' },
  { id: 'live', kind: 'action' },
  { id: 'library', kind: 'content' },
  { id: 'music', kind: 'content' },
  { id: 'games', kind: 'content' },
  { id: 'settings', kind: 'action' },
];

const cur = (m: { ui: Record<string, any> }): string => m.ui.menu ?? 'home';
const menuIndex = (id: string) => Math.max(0, MENU.findIndex((e) => e.id === id));

const meta: OsStyleMeta = {
  id: 'roku',
  nameKey: 'os.roku',
  descKey: 'os.roku.d',
  font: 'Montserrat',
  slots: ['status'],
  screens: {
    settings: () => import('./roku.screens/settings.svelte'),
    search: () => import('./roku.screens/search.svelte'),
    tileMenu: () => import('./roku.screens/tileMenu.svelte'),
    pairing: () => import('./roku.screens/pairing.svelte'),
    quick: () => import('./roku.screens/quick.svelte'),
    osd: () => import('./roku.screens/osd.svelte'),
    ambient: () => import('./roku.screens/ambient.svelte'),
    toast: () => import('./roku.screens/toast.svelte'),
    launch: () => import('./roku.screens/launch.svelte'),
  },
  tokens: {
    '--bg': '#13071f',
    '--bg-rgb': '19, 7, 31',
    '--text': '#ffffff',
    '--muted': 'rgba(255, 255, 255, 0.72)',
    '--glass': 'rgba(255, 255, 255, 0.1)',
    '--glass-strong': 'rgba(255, 255, 255, 0.22)',
    '--sheet': 'rgba(42, 16, 68, 0.97)',
    '--field': 'rgba(0, 0, 0, 0.35)',
    '--ring': '#ffffff',
    '--accent': '#6f1ab1',
    '--r': '0.6rem',
    '--font': "'Montserrat', 'Segoe UI', system-ui, sans-serif",
    '--focus-scale': '1.08',
    '--focus-ring': '0.3rem',
    '--dock-top': '1.6rem',
    '--dock-left': '24rem',
    '--dock-right': '3.4rem',
    '--dock-bottom': '3.4rem',
  },
  rows(base, model) {
    const by = (k: string): HomeRow | undefined => base.find((r) => r.key === k);
    const one = (tiles: HomeRow['tiles']): HomeRow[] => (tiles.length ? [{ key: 'grid', title: '', tiles }] : []);
    switch (cur(model)) {
      case 'apps':
        return one(model.allTiles);
      case 'library':
        return one(by('library')?.tiles ?? []);
      case 'music':
        return one(by('music')?.tiles ?? []);
      case 'games':
        return one(by('games')?.tiles ?? []);
      default:
        return one([...(by('continue')?.tiles ?? []), ...model.allTiles]);
    }
  },
  nav(dir, model) {
    if (model.zone === 'menu') {
      const i: number = model.ui.menuFocus ?? menuIndex(cur(model));
      if (dir === 'up' || dir === 'down') {
        const n = Math.max(0, Math.min(MENU.length - 1, i + (dir === 'up' ? -1 : 1)));
        if (n !== i) {
          model.ui.menuFocus = n;
          play('move');
          if (MENU[n].kind === 'content') {
            model.ui.menu = MENU[n].id; // la grille de droite suit le menu
            model.focusAt(0, 0, true);
          }
        }
      } else if (dir === 'right' && MENU[i].kind === 'content' && model.rows.length) model.setZone('content');
      return true;
    }
    const n = model.rows[0]?.tiles.length ?? 0;
    if (!n) return true;
    const c = model.focus.col;
    let t = c;
    if (dir === 'left') {
      if (c % COLS === 0) {
        model.ui.menuFocus = menuIndex(cur(model));
        model.setZone('menu');
        return true;
      }
      t = c - 1;
    } else if (dir === 'right') {
      if (c % COLS < COLS - 1 && c + 1 < n) t = c + 1;
    } else if (dir === 'up') {
      if (c >= COLS) t = c - COLS;
    } else if (c + COLS < n) t = c + COLS;
    else if (Math.floor(c / COLS) < Math.floor((n - 1) / COLS)) t = n - 1;
    model.focusAt(0, t);
    return true;
  },
  activate(model) {
    if (model.zone !== 'menu') return false;
    const i: number = model.ui.menuFocus ?? menuIndex(cur(model));
    const e = MENU[i];
    play('select');
    if (e.id === 'search') model.openSearch();
    else if (e.id === 'live') model.openLive();
    else if (e.id === 'settings') model.openSettings();
    else {
      model.ui.menu = e.id;
      model.focusAt(0, 0, true);
      if (model.rows.length) model.setZone('content');
    }
    return true;
  },
};
export default meta;
