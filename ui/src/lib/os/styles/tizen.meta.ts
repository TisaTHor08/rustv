// Style « Tizen » (Samsung One UI Tizen 2024-2025) : rail d'icônes à gauche, héros ambiant avec pastille d'onglets
// (Pour vous / Direct / Applis), rangées de cartes 16:9 puis rangée d'icônes d'applications, comme l'accueil réel.
// Zones : 'content' (rangées), 'tabs' (onglets + bouton du héros), 'rail' (rail de gauche), 'ambient'. Inspiré, non officiel.
import { t } from '../../i18n';
import { play } from '../../sounds';
import type { DTile } from '../../tiles.svelte';
import type { HomeRow } from '../model.svelte';
import type { Dir, OsStyleMeta } from '../registry';
import './tizen.css';
import './tizen.strings';

/** Rail de gauche, de haut en bas (le dernier groupe est collé en bas de l'écran). */
export const RAIL = ['profile', 'search', 'ambient', 'media', 'live', 'home', 'settings', 'alerts'] as const;
export const RAIL_BOTTOM = 6; // index du premier bouton du groupe du bas
export type RailId = (typeof RAIL)[number];
export const TABS = ['foryou', 'live', 'apps'] as const;
/** Onglets (0..2) puis bouton d'action du héros (3). */
export const TABS_COUNT = TABS.length + 1;
export const APPS_PER_ROW = 9;

const clamp = (n: number, a: number, b: number) => Math.max(a, Math.min(b, n));
const tabOf = (m: { ui: Record<string, any> }): (typeof TABS)[number] => (m.ui.tab === 'apps' ? 'apps' : 'foryou');
/** Rangée d'icônes d'applications (clé « apps » ou « apps0 »...). */
export const isAppsRow = (key: string) => key.startsWith('apps');

const meta: OsStyleMeta = {
  id: 'tizen',
  nameKey: 'os.tizen',
  descKey: 'os.tizen.d',
  font: 'Inter',
  slots: ['status'],
  screens: {
    settings: () => import('./tizen.screens/settings.svelte'),
    search: () => import('./tizen.screens/search.svelte'),
    tileMenu: () => import('./tizen.screens/tileMenu.svelte'),
    pairing: () => import('./tizen.screens/pairing.svelte'),
    quick: () => import('./tizen.screens/quick.svelte'),
    osd: () => import('./tizen.screens/osd.svelte'),
    ambient: () => import('./tizen.screens/ambient.svelte'),
    toast: () => import('./tizen.screens/toast.svelte'),
    launch: () => import('./tizen.screens/launch.svelte'),
  },
  tokens: {
    '--bg': '#0a0b17',
    '--bg-rgb': '10, 11, 23',
    '--text': '#f4f5fa',
    '--muted': 'rgba(244, 245, 250, 0.62)',
    '--glass': 'rgba(255, 255, 255, 0.1)',
    '--glass-strong': 'rgba(255, 255, 255, 0.2)',
    '--sheet': 'rgba(24, 25, 44, 0.97)',
    '--field': 'rgba(0, 0, 0, 0.38)',
    '--ring': '#ffffff',
    '--accent': '#3d7bff',
    '--r': '1.4rem',
    '--font': "'Inter', 'Segoe UI', system-ui, sans-serif",
    '--focus-scale': '1.06',
    '--focus-ring': '0.2rem',
    '--dock-top': '4.6rem',
    '--dock-left': '9rem',
    '--dock-right': '3rem',
    '--dock-bottom': '2rem',
  },
  rows(base, model) {
    const by = (k: string) => base.find((r) => r.key === k);
    if (tabOf(model) === 'apps') {
      const out: HomeRow[] = [];
      for (let i = 0; i < model.allTiles.length; i += APPS_PER_ROW) out.push({ key: `apps${i / APPS_PER_ROW}`, title: i === 0 ? t('os.tizen.allapps') : '', tiles: model.allTiles.slice(i, i + APPS_PER_ROW) });
      return out;
    }
    const seen = new Set<string>();
    const apps: DTile[] = [];
    for (const x of [...model.recent, ...model.allTiles]) if (!seen.has(x.id)) (seen.add(x.id), apps.push(x));
    const cards = [by('continue'), by('video'), by('tv'), by('music'), by('library'), by('games')].filter((r): r is HomeRow => !!r && r.tiles.length > 0);
    const out: HomeRow[] = [];
    if (cards[0]) out.push(cards[0]);
    if (apps.length) out.push({ key: 'apps', title: t('os.tizen.yourapps'), tiles: apps });
    out.push(...cards.slice(1));
    return out;
  },
  nav(dir: Dir, model) {
    const ui = model.ui;
    const step = (key: string, max: number, d: number) => {
      const i: number = ui[key] ?? 0;
      const j = clamp(i + d, 0, max - 1);
      if (j !== i) (ui[key] = j, play('move'));
    };
    switch (model.zone) {
      case 'ambient':
        model.setZone('content');
        return true;
      case 'rail':
        if (dir === 'up' || dir === 'down') step('railFocus', RAIL.length, dir === 'up' ? -1 : 1);
        else if (dir === 'right') model.setZone('content');
        return true;
      case 'tabs':
        if (dir === 'left') {
          if ((ui.tabFocus ?? 0) === 0) (ui.railFocus = RAIL.indexOf('home'), model.setZone('rail'));
          else step('tabFocus', TABS_COUNT, -1);
        } else if (dir === 'right') step('tabFocus', TABS_COUNT, 1);
        else if (dir === 'down') model.setZone('content');
        return true;
    }
    if (dir === 'left' && model.focus.col === 0) {
      ui.railFocus = RAIL.indexOf('home');
      model.setZone('rail');
      return true;
    }
    if (dir === 'up' && model.focus.row <= 0) {
      ui.tabFocus = TABS.indexOf(tabOf(model));
      model.setZone('tabs');
      return true;
    }
    return false;
  },
  activate(model) {
    const ui = model.ui;
    switch (model.zone) {
      case 'ambient':
        model.setZone('content');
        return true;
      case 'rail': {
        const id: RailId = RAIL[ui.railFocus ?? 0];
        play('select');
        if (id === 'profile') model.openQuick();
        else if (id === 'search') model.openSearch();
        else if (id === 'ambient') model.setZone('ambient');
        else if (id === 'media') model.openMedia('movies');
        else if (id === 'live') model.openLive();
        else if (id === 'home') (ui.tab = 'foryou', model.focusAt(0, 0, true), model.setZone('content'));
        else if (id === 'settings') model.openSettings();
        else model.note(t('os.tizen.nonotif'));
        return true;
      }
      case 'tabs': {
        const i: number = ui.tabFocus ?? 0;
        play('select');
        if (i === 1) model.openLive();
        else if (i === 3) {
          if (model.focused) model.open(model.focused);
        } else {
          ui.tab = TABS[i];
          model.focusAt(0, 0, true);
          model.setZone('content');
        }
        return true;
      }
    }
    return false;
  },
};
export default meta;
