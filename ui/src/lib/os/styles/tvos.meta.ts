// Style « Apple TV » (tvOS) : capsule d'onglets en haut (zone 'nav'), vitrine plein cadre, rangées d'icônes en relief. Inspiré, non officiel.
import { play } from '../../sounds';
import { t } from '../../i18n';
import type { HomeRow } from '../model.svelte';
import type { OsStyleMeta } from '../registry';
import './tvos.css';
import './tvos.strings';

export type TvosEntry = { id: string; kind: 'content' | 'action'; icon?: boolean };
/** Barre du haut (index de focus) : 0..4 dans la capsule, 5 = profil / réglages à droite. */
export const NAV: TvosEntry[] = [
  { id: 'home', kind: 'content' },
  { id: 'apps', kind: 'content' },
  { id: 'library', kind: 'content' },
  { id: 'live', kind: 'action' },
  { id: 'search', kind: 'action', icon: true },
  { id: 'settings', kind: 'action', icon: true },
];
export const GRID_COLS = 6;

const cur = (m: { ui: Record<string, any> }): string => m.ui.tab ?? 'home';
const navIndex = (id: string) => Math.max(0, NAV.findIndex((e) => e.id === id));
const by = (rows: HomeRow[], key: string) => rows.find((r) => r.key === key);
const chunk = (tiles: HomeRow['tiles'], n: number): HomeRow[] => {
  const out: HomeRow[] = [];
  for (let i = 0; i < tiles.length; i += n) out.push({ key: `g${i / n}`, title: '', tiles: tiles.slice(i, i + n) });
  return out;
};

const meta: OsStyleMeta = {
  id: 'tvos',
  nameKey: 'os.tvos',
  descKey: 'os.tvos.d',
  font: 'Inter',
  slots: ['status'],
  tokens: {
    '--bg': '#000000',
    '--bg-rgb': '0, 0, 0',
    '--text': '#ffffff',
    '--muted': 'rgba(235, 235, 245, 0.62)',
    '--glass': 'rgba(118, 118, 128, 0.26)',
    '--glass-strong': 'rgba(118, 118, 128, 0.44)',
    '--sheet': 'rgba(28, 28, 30, 0.96)',
    '--field': 'rgba(118, 118, 128, 0.24)',
    '--ring': '#ffffff',
    '--accent': '#0a84ff',
    '--r': '1.3rem',
    '--font': "-apple-system, 'SF Pro Display', 'SF Pro Text', 'Inter', 'Segoe UI', system-ui, sans-serif",
    '--focus-scale': '1.16',
    '--focus-ring': '0.2rem',
    '--dock-top': '6.6rem',
    '--dock-left': '5rem',
    '--dock-right': '5rem',
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
    const apps: HomeRow = { key: 'apps', title: '', tiles: model.allTiles };
    const up = cont ? [{ ...cont, title: t('os.tvos.upnext') }] : [];
    return [...(apps.tiles.length ? [apps] : []), ...up, ...(rec ? [rec] : []), ...cats];
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
