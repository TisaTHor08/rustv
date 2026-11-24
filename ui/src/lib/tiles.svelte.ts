// Tuiles affichées : catalogue + tuiles personnalisées, avec les personnalisations de l'utilisateur
// (ordre, noms, couleurs, icônes, masquage).
import { api, type Tile } from './api';
import { t, tileDesc } from './i18n';
import { modBanner, modIcon, modTiles, type ModTile } from './mods.svelte';
import { prefs, savePrefs, type Category, type CustomTile } from './prefs.svelte';

export interface DTile {
  id: string;
  name: string;
  desc: string;
  category: Category;
  color: string;
  color2: string;
  dark_text?: boolean;
  icon?: string;
  /** Visuel d'écran de l'appli (mod), affiché dans le bandeau du haut. */
  banner?: string;
  custom: boolean;
  source: 'catalog' | 'custom' | 'mod' | 'builtin';
  customized: boolean;
  raw: Tile | CustomTile | ModTile | BuiltinTile;
}

/** Tuiles intégrées à RusTV : pages internes (TV en direct, médiathèque…) et raccourcis Windows. */
export interface BuiltinTile {
  id: string;
  name: string;
  nameKey: string;
  descKey: string;
  category: Category;
  /** Page interne : "live", "media:<onglet>" ; sinon `target` est lancé comme un programme/URI Windows. */
  action?: string;
  target?: string;
  color: string;
  color2: string;
}

export const BUILTIN: BuiltinTile[] = [
  { id: 'live', name: 'Live TV', nameKey: 'b.live', descKey: 'b.live.d', category: 'tv', action: 'live', color: '#d62f4a', color2: '#4d0e1b' },
  { id: 'media', name: 'Library', nameKey: 'b.media', descKey: 'b.media.d', category: 'library', action: 'media:movies', color: '#6d4aff', color2: '#1d1066' },
  { id: 'files', name: 'Files', nameKey: 'b.files', descKey: 'b.files.d', category: 'library', action: 'media:files', color: '#2f80ed', color2: '#0b2a63' },
  { id: 'set-wifi', name: 'Wi-Fi', nameKey: 'b.wifi', descKey: 'b.wifi.d', category: 'system', target: 'ms-settings:network-wifi', color: '#1d9bf0', color2: '#073b66' },
  { id: 'set-bluetooth', name: 'Bluetooth', nameKey: 'b.bt', descKey: 'b.bt.d', category: 'system', target: 'ms-settings:bluetooth', color: '#3b6bff', color2: '#0f1f66' },
  { id: 'set-sound', name: 'Sound', nameKey: 'b.sound', descKey: 'b.sound.d', category: 'system', target: 'ms-settings:sound', color: '#14a05a', color2: '#05361f' },
  { id: 'set-display', name: 'Displays', nameKey: 'b.display', descKey: 'b.display.d', category: 'system', target: 'ms-settings:display', color: '#e5a00d', color2: '#5c3f00' },
  { id: 'set-power', name: 'Power', nameKey: 'b.power', descKey: 'b.power.d', category: 'system', target: 'ms-settings:powersleep', color: '#8a94a6', color2: '#2b3140' },
];

export const CATEGORIES: Category[] = ['video', 'tv', 'music', 'library', 'games', 'system'];

function decorate(raw: Tile | CustomTile | ModTile | BuiltinTile, source: DTile['source']): DTile {
  const custom = source === 'custom';
  const o = prefs.tiles;
  const col = o.colors[raw.id];
  const desc =
    source === 'builtin'
      ? t((raw as BuiltinTile).descKey)
      : source === 'catalog'
      ? tileDesc(raw.id, (raw as Tile).desc)
      : source === 'mod'
        ? ((raw as ModTile).desc ?? '')
        : (raw as CustomTile).kind === 'web'
          ? ((raw as CustomTile).url ?? '')
          : 'Application Windows';
  return {
    id: raw.id,
    name: o.names[raw.id] ?? (source === 'builtin' ? t((raw as BuiltinTile).nameKey) : raw.name),
    desc,
    category: raw.category as Category,
    color: col?.[0] ?? raw.color,
    color2: col?.[1] ?? raw.color2,
    dark_text: (raw as Tile).dark_text && !col,
    icon: o.icons[raw.id] ?? (raw as CustomTile).icon ?? (raw as ModTile).icon ?? modIcon(raw.id),
    banner: modBanner(raw.id),
    custom,
    source,
    customized: !!(o.names[raw.id] || col || o.icons[raw.id]),
    raw,
  };
}

/** Toutes les tuiles d'une catégorie, dans l'ordre choisi par l'utilisateur (masquées incluses si demandé). */
export function tilesOf(catalog: Tile[], cat: Category, includeHidden = false): DTile[] {
  const base = [
    ...BUILTIN.filter((b) => b.category === cat).map((b) => decorate(b, 'builtin')),
    ...catalog.filter((t) => t.category === cat).map((t) => decorate(t, 'catalog')),
    ...prefs.tiles.custom.filter((c) => c.category === cat).map((c) => decorate(c, 'custom')),
    ...modTiles().filter((m) => m.category === cat).map((m) => decorate(m, 'mod')),
  ];
  const order = prefs.tiles.order[cat] ?? [];
  const rank = (id: string) => {
    const i = order.indexOf(id);
    return i === -1 ? 1e6 : i;
  };
  const sorted = base.map((t, i) => ({ t, i })).sort((a, b) => rank(a.t.id) - rank(b.t.id) || a.i - b.i).map((x) => x.t);
  return includeHidden ? sorted : sorted.filter((t) => !prefs.tiles.hidden.includes(t.id));
}

export function findTile(catalog: Tile[], id: string): DTile | undefined {
  for (const c of CATEGORIES) {
    const t = tilesOf(catalog, c, true).find((x) => x.id === id);
    if (t) return t;
  }
}

/** Déplace une tuile d'un cran (-1 gauche, +1 droite) parmi les tuiles visibles de sa rangée. Renvoie le nouvel index. */
export function moveTile(catalog: Tile[], id: string, dir: -1 | 1): number {
  const t = findTile(catalog, id);
  if (!t) return -1;
  const all = tilesOf(catalog, t.category, true).map((x) => x.id);
  const visible = tilesOf(catalog, t.category).map((x) => x.id);
  const vi = visible.indexOf(id);
  const target = visible[vi + dir];
  if (target === undefined) return vi;
  const a = all.indexOf(id), b = all.indexOf(target);
  [all[a], all[b]] = [all[b], all[a]];
  prefs.tiles.order[t.category] = all;
  savePrefs();
  return vi + dir;
}

export function hideTile(id: string) {
  if (!prefs.tiles.hidden.includes(id)) prefs.tiles.hidden = [...prefs.tiles.hidden, id];
  savePrefs();
}

export function deleteCustom(id: string) {
  prefs.tiles.custom = prefs.tiles.custom.filter((c) => c.id !== id);
  delete prefs.tiles.names[id];
  delete prefs.tiles.colors[id];
  delete prefs.tiles.icons[id];
  savePrefs();
}

export function resetTile(id: string) {
  delete prefs.tiles.names[id];
  delete prefs.tiles.colors[id];
  delete prefs.tiles.icons[id];
  savePrefs();
}

// ---------- icônes : les chemins de fichiers passent par le serveur média local ----------

// Cache non réactif (on ne doit jamais modifier un état pendant le rendu) + compteur de version
// incrémenté quand une URL arrive : la lecture de `ver` suffit à rafraîchir les composants.
const cache = new Map<string, string>();
let ver = $state(0);

/** URL utilisable dans <img> pour une icône (data:, http(s) ou chemin local). */
export function iconSrc(v?: string): string {
  if (!v) return '';
  if (v.startsWith('data:') || v.startsWith('http')) return v;
  void ver;
  if (!cache.has(v)) {
    cache.set(v, '');
    api
      .mediaUrl(v)
      .then((u) => {
        cache.set(v, u);
        ver++;
      })
      .catch(() => {});
  }
  return cache.get(v) ?? '';
}