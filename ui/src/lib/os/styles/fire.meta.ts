// Style « Fire TV » : barre de navigation en haut (zone 'nav'), grand héros, rangées de cartes 16:9. Inspiré, non officiel.
import { play } from '../../sounds';
import { t } from '../../i18n';
import type { DTile } from '../../tiles.svelte';
import type { HomeRow } from '../model.svelte';
import type { OsStyleMeta } from '../registry';
import './fire.css';
import './fire.strings';

export type FireEntry = { id: 'profile' | 'search' | 'home' | 'live' | 'library' | 'app' | 'apps' | 'settings'; tile?: DTile };
/** Nombre de raccourcis d'applis dans la barre (à droite des icônes, comme sur l'accueil Fire TV). */
export const SHORTCUTS = 6;
export const GRID_COLS = 5;

/** Barre du haut, de gauche à droite : profil, recherche, accueil, direct, bibliothèque, raccourcis d'applis, toutes les applis, réglages. */
export function navEntries(model: { recent: DTile[]; allTiles: DTile[] }): FireEntry[] {
  const seen = new Set<string>();
  const apps: DTile[] = [];
  for (const tl of [...model.recent, ...model.allTiles]) {
    if (apps.length >= SHORTCUTS) break;
    if (!seen.has(tl.id)) (seen.add(tl.id), apps.push(tl));
  }
  return [
    { id: 'profile' },
    { id: 'search' },
    { id: 'home' },
    { id: 'live' },
    { id: 'library' },
    ...apps.map((tile): FireEntry => ({ id: 'app', tile })),
    { id: 'apps' },
    { id: 'settings' },
  ];
}

const cur = (m: { ui: Record<string, any> }): string => m.ui.tab ?? 'home';
const navIndex = (list: FireEntry[], id: string) => Math.max(0, list.findIndex((e) => e.id === id));
const by = (rows: HomeRow[], key: string) => rows.find((r) => r.key === key);
const chunk = (tiles: HomeRow['tiles'], n: number): HomeRow[] => {
  const out: HomeRow[] = [];
  for (let i = 0; i < tiles.length; i += n) out.push({ key: `g${i / n}`, title: i === 0 ? t('os.fire.allapps') : '', tiles: tiles.slice(i, i + n) });
  return out;
};

const meta: OsStyleMeta = {
  id: 'fire',
  nameKey: 'os.fire',
  descKey: 'os.fire.d',
  font: 'Roboto',
  slots: ['status'],
  /** Zones de focus que ce style sait gérer (voir navguard.ts) : toute autre zone est ramenée à 'content'. */
  zones: ['content', 'nav'],
  screens: {
    settings: () => import('./fire.screens/settings.svelte'),
    search: () => import('./fire.screens/search.svelte'),
    tileMenu: () => import('./fire.screens/tileMenu.svelte'),
    pairing: () => import('./fire.screens/pairing.svelte'),
    quick: () => import('./fire.screens/quick.svelte'),
    osd: () => import('./fire.screens/osd.svelte'),
    ambient: () => import('./fire.screens/ambient.svelte'),
    toast: () => import('./fire.screens/toast.svelte'),
    launch: () => import('./fire.screens/launch.svelte'),
  },
  tokens: {
    '--bg': '#171717',
    '--bg-rgb': '23, 23, 23',
    '--text': '#ffffff',
    '--muted': 'rgba(255, 255, 255, 0.68)',
    '--glass': 'rgba(255, 255, 255, 0.09)',
    '--glass-strong': 'rgba(255, 255, 255, 0.18)',
    '--sheet': 'rgba(34, 34, 34, 0.98)',
    '--field': 'rgba(0, 0, 0, 0.35)',
    '--ring': '#ffffff',
    '--accent': '#1a98ff',
    '--r': '0.3rem',
    '--font': "'Roboto', 'Segoe UI', system-ui, sans-serif",
    '--focus-scale': '1.06',
    '--focus-ring': '0.2rem',
    '--dock-top': '1.4rem',
    '--dock-left': '4rem',
    '--dock-right': '4rem',
    '--dock-bottom': '1.4rem',
  },
  rows(base, model) {
    const tab = cur(model);
    const cont = by(base, 'continue');
    const lib = by(base, 'library');
    const cats = ['video', 'tv', 'music', 'library', 'games'].map((k) => by(base, k)).filter((r): r is HomeRow => !!r);
    if (tab === 'apps') return chunk(model.allTiles, GRID_COLS);
    if (tab === 'library') return [...(cont ? [cont] : []), ...(lib ? [lib] : [])];
    return [...(cont ? [cont] : []), ...cats];
  },
  nav(dir, model) {
    if (model.zone === 'nav') {
      const list = navEntries(model);
      const i: number = Math.min(list.length - 1, model.ui.navFocus ?? navIndex(list, cur(model)));
      if (dir === 'down') model.setZone('content');
      else if (dir === 'left' || dir === 'right') {
        const n = Math.max(0, Math.min(list.length - 1, i + (dir === 'left' ? -1 : 1)));
        if (n !== i) {
          model.ui.navFocus = n;
          play('move');
        }
      }
      return true;
    }
    if (dir === 'up' && model.focus.row <= 0) {
      model.ui.navFocus = navIndex(navEntries(model), cur(model));
      model.setZone('nav');
      return true;
    }
    return false;
  },
  activate(model) {
    if (model.zone !== 'nav') return false;
    const list = navEntries(model);
    const e = list[Math.min(list.length - 1, model.ui.navFocus ?? navIndex(list, cur(model)))];
    play('select');
    if (e.id === 'search') model.openSearch();
    else if (e.id === 'live') model.openLive();
    else if (e.id === 'settings') model.openSettings();
    else if (e.id === 'profile') model.openQuick();
    else if (e.id === 'app' && e.tile) model.open(e.tile);
    else {
      model.ui.tab = e.id;
      model.focusAt(0, 0, true);
      model.setZone('content');
    }
    return true;
  },
};
export default meta;
