// Style « Google TV » : onglets en haut (zone 'tabs'), héros plein cadre, rangées de cartes. Inspiré, non officiel.
import { t } from '../../i18n';
import { play } from '../../sounds';
import type { HomeRow } from '../model.svelte';
import type { OsStyleMeta } from '../registry';
import './googletv.css';
import './googletv.strings';

export const TABS = ['foryou', 'movies', 'shows', 'apps', 'library'] as const;
export type GtvTab = (typeof TABS)[number];
/** Nombre de cartes par ligne dans la grille « Applis ». */
export const GRID_COLS = 6;
/** Index de focus de la barre du haut, de gauche à droite : 0 profil, 1 recherche, 2..6 onglets. */
export const TOP_COUNT = TABS.length + 2;

const tabOf = (m: { ui: Record<string, any> }): GtvTab => m.ui.tab ?? 'foryou';
const by = (rows: HomeRow[], key: string) => rows.find((r) => r.key === key);
const chunk = (tiles: HomeRow['tiles'], key: string, n: number): HomeRow[] => {
  const out: HomeRow[] = [];
  for (let i = 0; i < tiles.length; i += n) out.push({ key: `${key}${i / n}`, title: '', tiles: tiles.slice(i, i + n) });
  return out;
};

const meta: OsStyleMeta = {
  id: 'googletv',
  nameKey: 'os.googletv',
  descKey: 'os.googletv.d',
  font: 'Roboto',
  slots: ['status'],
  /** Zones de focus que ce style sait gérer (voir navguard.ts) : toute autre zone est ramenée à 'content'. */
  zones: ['content', 'tabs'],
  // Écrans propres au style (voir docs/OS-STYLES.md, « Écrans »)
  screens: {
    settings: () => import('./googletv.screens/settings.svelte'),
    search: () => import('./googletv.screens/search.svelte'),
    tileMenu: () => import('./googletv.screens/tileMenu.svelte'),
    pairing: () => import('./googletv.screens/pairing.svelte'),
    quick: () => import('./googletv.screens/quick.svelte'),
    osd: () => import('./googletv.screens/osd.svelte'),
    ambient: () => import('./googletv.screens/ambient.svelte'),
    toast: () => import('./googletv.screens/toast.svelte'),
    launch: () => import('./googletv.screens/launch.svelte'),
  },
  tokens: {
    '--bg': '#131314',
    '--bg-rgb': '19, 19, 20',
    '--text': '#e8eaed',
    '--muted': 'rgba(232, 234, 237, 0.66)',
    '--glass': 'rgba(255, 255, 255, 0.08)',
    '--glass-strong': 'rgba(255, 255, 255, 0.16)',
    '--sheet': 'rgba(32, 33, 36, 0.97)',
    '--field': 'rgba(0, 0, 0, 0.35)',
    '--ring': '#ffffff',
    '--accent': '#8ab4f8',
    '--r': '0.9rem',
    '--font': "'Roboto', 'Segoe UI', system-ui, sans-serif",
    '--focus-scale': '1.08',
    '--focus-ring': '0.2rem',
    '--dock-top': '5.4rem',
    '--dock-left': '4.2rem',
    '--dock-right': '4.2rem',
    '--dock-bottom': '1.4rem',
  },
  rows(base, model) {
    const tab = tabOf(model);
    const cont = by(base, 'continue');
    const apps = base.flatMap((r) => (['video', 'tv', 'music', 'games'].includes(r.key) ? r.tiles : []));
    const pick = (...keys: string[]) => keys.map((k) => by(base, k)).filter((r): r is HomeRow => !!r);
    switch (tab) {
      case 'movies':
        return [...(cont ? [cont] : []), ...pick('video')];
      case 'shows':
        return [...pick('tv', 'video')];
      case 'apps':
        return chunk(model.allTiles, 'g', GRID_COLS);
      case 'library':
        return pick('library');
      default:
        return [...(cont ? [cont] : []), ...(apps.length ? [{ key: 'apps', title: t('gtv.yourapps'), tiles: apps }] : []), ...pick('video', 'tv', 'music', 'library', 'games')];
    }
  },
  nav(dir, model) {
    if (model.zone === 'tabs') {
      const i: number = model.ui.tabFocus ?? 2;
      if (dir === 'down') model.setZone('content');
      else if (dir === 'left' || dir === 'right') {
        const n = Math.max(0, Math.min(TOP_COUNT - 1, i + (dir === 'left' ? -1 : 1)));
        if (n !== i) {
          model.ui.tabFocus = n;
          play('move');
        }
      }
      return true;
    }
    if (dir === 'up' && model.focus.row <= 0) {
      model.ui.tabFocus = TABS.indexOf(tabOf(model)) + 2;
      model.setZone('tabs');
      return true;
    }
    return false;
  },
  activate(model) {
    if (model.zone !== 'tabs') return false;
    const i: number = model.ui.tabFocus ?? 2;
    play('select');
    if (i === 1) model.openSearch();
    else if (i === 0) model.openQuick();
    else {
      model.ui.tab = TABS[i - 2];
      model.focusAt(0, 0, true);
      model.setZone('content');
    }
    return true;
  },
};
export default meta;
