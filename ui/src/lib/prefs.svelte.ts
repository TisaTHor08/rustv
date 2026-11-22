// Préférences de l'interface : un seul objet réactif, persisté côté Rust (prefs.json).
import { api } from './api';

export type Category = 'video' | 'tv' | 'music' | 'library' | 'games' | 'system';

export interface CustomTile {
  id: string;
  name: string;
  kind: 'web' | 'native';
  url?: string;
  target?: string;
  category: Category;
  color: string;
  color2: string;
  icon?: string;
  spatnav?: boolean;
}

export interface Prefs {
  lang: 'fr' | 'en';
  theme: 'dark' | 'light' | 'auto';
  accent: string;
  /** Thème apporté par un mod, au format « idDuMod/idDuThème » (vide = aucun). */
  modTheme: string;
  tint: boolean;
  clock24: boolean;
  /** Taille de l'interface en % (accessibilité / distance d'écran). */
  uiScale: number;
  autoUpdate: boolean;
  lastUpdateCheck: number;
  wallpaper: {
    type: 'aurora' | 'color' | 'image' | 'folder' | 'video';
    path: string;
    color: string;
    blur: number;
    dim: number;
    interval: number;
  };
  weather: { enabled: boolean; city: string; lat: number | null; lon: number | null; unit: 'c' | 'f' };
  live: { sources: { id: string; name: string; m3u: string; xmltv: string }[]; active: string };
  library: { movies: string[]; series: string[]; photos: string[]; music: string[] };
  tiles: {
    order: Record<string, string[]>;
    hidden: string[];
    names: Record<string, string>;
    colors: Record<string, [string, string]>;
    icons: Record<string, string>;
    custom: CustomTile[];
  };
}

export const ACCENTS = ['#ff6a1a', '#ff3b5c', '#a855f7', '#3b82f6', '#06b6d4', '#22c55e', '#eab308', '#94a3b8'];
export const TILE_COLORS: [string, string][] = [
  ['#ff6a1a', '#7a2d00'],
  ['#e50914', '#5c0409'],
  ['#1b4fd8', '#0a1f5c'],
  ['#00a8e1', '#04485f'],
  ['#5b2bff', '#1d0a66'],
  ['#14a05a', '#05361f'],
  ['#9146ff', '#3a1a80'],
  ['#e5a00d', '#6e4d00'],
  ['#2a2a2e', '#0b0b0d'],
  ['#d6336c', '#6b0f33'],
];

function defaults(): Prefs {
  return {
    lang: 'fr',
    theme: 'dark',
    accent: ACCENTS[0],
    modTheme: '',
    tint: true,
    clock24: true,
    uiScale: 100,
    autoUpdate: true,
    lastUpdateCheck: 0,
    wallpaper: { type: 'aurora', path: '', color: '#0b0e14', blur: 0, dim: 30, interval: 30 },
    weather: { enabled: false, city: '', lat: null, lon: null, unit: 'c' },
    live: { sources: [], active: '' },
    library: { movies: [], series: [], photos: [], music: [] },
    tiles: { order: {}, hidden: [], names: {}, colors: {}, icons: {}, custom: [] },
  };
}

export const prefs: Prefs = $state(defaults());
export const ready = $state({ loaded: false });

function merge(dst: Record<string, any>, src: Record<string, any>) {
  for (const k of Object.keys(src)) {
    const v = src[k];
    if (v && typeof v === 'object' && !Array.isArray(v) && dst[k] && typeof dst[k] === 'object' && !Array.isArray(dst[k])) merge(dst[k], v);
    else dst[k] = v;
  }
}

export async function loadPrefs() {
  try {
    const p = await api.prefsGet();
    if (p && typeof p === 'object') merge(prefs as any, p);
  } catch {}
  ready.loaded = true;
}

let timer: number | undefined;
/** Enregistre les préférences (regroupe les changements rapprochés). */
export function savePrefs() {
  clearTimeout(timer);
  timer = window.setTimeout(() => {
    void api.prefsSet($state.snapshot(prefs)).catch(() => {});
  }, 300);
}

// Thème et couleur d'accent appliqués au document (réagit aux préférences et au thème Windows).
$effect.root(() => {
  $effect(() => {
    const th = prefs.theme;
    const mq = matchMedia('(prefers-color-scheme: light)');
    const apply = () => (document.documentElement.dataset.theme = th === 'auto' ? (mq.matches ? 'light' : 'dark') : th);
    apply();
    mq.addEventListener('change', apply);
    return () => mq.removeEventListener('change', apply);
  });
  $effect(() => {
    document.documentElement.style.setProperty('--accent', prefs.accent);
  });
  $effect(() => {
    document.documentElement.style.setProperty('--ui-scale', String(prefs.uiScale / 100));
  });
});

/** Remet tous les réglages de tuiles à zéro. */
export function resetTiles() {
  prefs.tiles = defaults().tiles;
  savePrefs();
}
