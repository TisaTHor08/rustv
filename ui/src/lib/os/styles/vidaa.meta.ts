// Style « VIDAA » (téléviseurs Hisense) : bande d'état et raccourcis en haut (zone 'top'), bannière promotionnelle, rangées de grandes tuiles colorées. Inspiré, non officiel.
import { t } from '../../i18n';
import { play } from '../../sounds';
import type { HomeRow } from '../model.svelte';
import type { OsStyleMeta } from '../registry';
import './vidaa.css';
import './vidaa.strings';

/** Rail d'icônes à gauche (zone 'rail'), de haut en bas : profil, recherche, notifications, entrées, réglages. */
export const RAIL = ['profile', 'search', 'notif', 'input', 'settings'] as const;
export type VidaaRail = (typeof RAIL)[number];

const by = (rows: HomeRow[], key: string) => rows.find((r) => r.key === key);

const meta: OsStyleMeta = {
  id: 'vidaa',
  nameKey: 'os.vidaa',
  descKey: 'os.vidaa.d',
  font: 'Roboto',
  slots: ['status'],
  tokens: {
    '--bg': '#10121a',
    '--bg-rgb': '16, 18, 26',
    '--text': '#ffffff',
    '--muted': 'rgba(255, 255, 255, 0.66)',
    '--glass': 'rgba(255, 255, 255, 0.09)',
    '--glass-strong': 'rgba(255, 255, 255, 0.2)',
    '--sheet': 'rgba(22, 25, 36, 0.98)',
    '--field': 'rgba(0, 0, 0, 0.4)',
    '--ring': '#ffffff',
    '--accent': '#2fb5c8',
    '--r': '0.5rem',
    '--font': "'Roboto', 'Segoe UI', system-ui, sans-serif",
    '--focus-scale': '1.06',
    '--focus-ring': '0.22rem',
    '--dock-top': '0.8rem',
    '--dock-left': '6rem',
    '--dock-right': '4.5rem',
    '--dock-bottom': '1.4rem',
  },
  screens: {
    settings: () => import('./vidaa.screens/settings.svelte'),
    search: () => import('./vidaa.screens/search.svelte'),
    tileMenu: () => import('./vidaa.screens/tileMenu.svelte'),
    pairing: () => import('./vidaa.screens/pairing.svelte'),
    quick: () => import('./vidaa.screens/quick.svelte'),
    osd: () => import('./vidaa.screens/osd.svelte'),
    ambient: () => import('./vidaa.screens/ambient.svelte'),
    toast: () => import('./vidaa.screens/toast.svelte'),
    launch: () => import('./vidaa.screens/launch.svelte'),
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
    if (model.zone === 'rail') {
      const i: number = model.ui.railFocus ?? 1;
      if (dir === 'right') model.setZone('content');
      else if (dir === 'up' || dir === 'down') {
        const n = Math.max(0, Math.min(RAIL.length - 1, i + (dir === 'up' ? -1 : 1)));
        if (n !== i) {
          model.ui.railFocus = n;
          play('move');
        }
      }
      return true;
    }
    if (model.zone === 'banner') {
      if (dir === 'down') model.setZone('content');
      else if (dir === 'left') {
        model.ui.railFocus = model.ui.railFocus ?? 1;
        model.setZone('rail');
      }
      return true;
    }
    if (dir === 'up' && model.focus.row <= 0) {
      model.setZone('banner');
      return true;
    }
    if (dir === 'left' && model.focus.col <= 0) {
      model.ui.railFocus = model.ui.railFocus ?? 1;
      model.setZone('rail');
      return true;
    }
    return false;
  },
  activate(model) {
    if (model.zone !== 'rail') return false;
    const s = RAIL[model.ui.railFocus ?? 1];
    play('select');
    if (s === 'search') model.openSearch();
    else if (s === 'notif') model.note(t('vidaa.nonotif'));
    else if (s === 'input') model.openMedia('movies');
    else model.openSettings();
    return true;
  },
};
export default meta;
