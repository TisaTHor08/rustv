// Tuiles affichées : catalogue + tuiles personnalisées, avec les personnalisations de l'utilisateur
// (ordre, noms, couleurs, icônes, masquage).
import { api, type Tile } from './api';
import { tileDesc } from './i18n';
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
  custom: boolean;
  customized: boolean;
  raw: Tile | CustomTile;
}

export const CATEGORIES: Category[] = ['video', 'tv', 'music', 'library', 'games'];

function decorate(raw: Tile | CustomTile, custom: boolean): DTile {
  const o = prefs.tiles;
  const col = o.colors[raw.id];
  const desc = custom
    ? (raw as CustomTile).kind === 'web'
      ? (raw as CustomTile).url ?? ''
      : 'Application Windows'
    : tileDesc(raw.id, (raw as Tile).desc);
  return {
    id: raw.id,
    name: o.names[raw.id] ?? raw.name,
    desc,
    category: raw.category as Category,
    color: col?.[0] ?? raw.color,
    color2: col?.[1] ?? raw.color2,
    dark_text: (raw as Tile).dark_text && !col,
    icon: o.icons[raw.id] ?? (raw as CustomTile).icon,
    custom,
    customized: !!(o.names[raw.id] || col || o.icons[raw.id]),
    raw,
  };
}

/** Toutes les tuiles d'une catégorie, dans l'ordre choisi par l'utilisateur (masquées incluses si demandé). */
export function tilesOf(catalog: Tile[], cat: Category, includeHidden = false): DTile[] {
  const base = [...catalog.filter((t) => t.category === cat).map((t) => decorate(t, false)), ...prefs.tiles.custom.filter((c) => c.category === cat).map((c) => decorate(c, true))];
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

const urls = $state<Record<string, string>>({});

/** URL utilisable dans <img> pour une icône (data:, http(s) ou chemin local). */
export function iconSrc(v?: string): string {
  if (!v) return '';
  if (v.startsWith('data:') || v.startsWith('http')) return v;
  if (urls[v] === undefined) {
    urls[v] = '';
    api.mediaUrl(v).then((u) => (urls[v] = u)).catch(() => {});
  }
  return urls[v];
}
