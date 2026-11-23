// Style « Fire TV » : barre de navigation en haut (zone 'nav'), grand héros, rangées de cartes 16:9. Inspiré, non officiel.
import { play } from '../../sounds';
import { t } from '../../i18n';
import type { HomeRow } from '../model.svelte';
import type { OsStyleMeta } from '../registry';
import './fire.css';
import './fire.strings';

export type FireEntry = { id: string; kind: 'content' | 'action'; icon?: boolean };
/** Barre du haut, de gauche à droite (index de focus = position dans ce tableau). */
export const NAV: FireEntry[] = [
  { id: 'search', kind: 'action', icon: true },
  { id: 'home', kind: 'content' },
  { id: 'live', kind: 'action' },
  { id: 'apps', kind: 'content' },
  { id: 'library', kind: 'content' },
  { id: 'settings', kind: 'action', icon: true },
];
export const GRID_COLS = 6;

const cur = (m: { ui: Record<string, any> }): string => m.ui.tab ?? 'home';
const navIndex = (id: string) => Math.max(0, NAV.findIndex((e) => e.id === id));
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
  tokens: {
    '--bg': '#0f171e',
    '--bg-rgb': '15, 23, 30',
    '--text': '#ffffff',
    '--muted': 'rgba(255, 255, 255, 0.68)',
    '--glass': 'rgba(255, 255, 255, 0.09)',
    '--glass-strong': 'rgba(255, 255, 255, 0.18)',
    '--sheet': 'rgba(24, 34, 44, 0.98)',
    '--field': 'rgba(0, 0, 0, 0.35)',
    '--ring': '#ffffff',
    '--accent': '#ff9900',
    '--r': '0.6rem',
    '--font': "'Roboto', 'Segoe UI', system-ui, sans-serif",
    '--focus-scale': '1.1',
    '--focus-ring': '0.25rem',
    '--dock-top': '5.6rem',
    '--dock-left': '4rem',
    '--dock-right': '4rem',
    '--dock-bottom': '1.4rem',
  },
  rows(base, model) {
    const tab = cur(model);
    const cont = by(base, 'continue');
    const rec = by(base, 'recent');
    const lib = by(base, 'library');
    const cats = ['video', 'tv', 'music', 'library', 'games'].map((k) => by(base, k)).filter((r): r is HomeRow => !!r);
    if (tab === 'apps') return chunk(model.allTiles, GRID_COLS);
    if (tab === 'library') return [...(cont ? [cont] : []), ...(lib ? [lib] : [])];
    const apps: HomeRow = { key: 'apps', title: t('os.fire.yourapps'), tiles: rec?.tiles.length ? rec.tiles : model.allTiles.slice(0, 12) };
    return [...(apps.tiles.length ? [apps] : []), ...(cont ? [cont] : []), ...cats];
  },
  nav(dir, model) {
    if (model.zone === 'nav') {
      const i: number = model.ui.navFocus ?? navIndex(cur(model));
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
      model.ui.navFocus = navIndex(cur(model));
      model.setZone('nav');
      return true;
    }
    return false;
  },
  activate(model) {
    if (model.zone !== 'nav') return false;
    const e = NAV[model.ui.navFocus ?? navIndex(cur(model))];
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
