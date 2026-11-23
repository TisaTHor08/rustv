// Registre des styles d'interface « façon OS TV ». Un style = deux fichiers dans ./styles :
//   <id>.svelte  : le composant de mise en page (reçoit { model: HomeModel })
//   <id>.meta.ts : ses métadonnées (clés de nom, jetons CSS, police, navigation)
// Les deux sont découverts avec import.meta.glob : ajouter un style ne demande AUCUNE autre modification.
// Un style dont le fichier .svelte n'existe pas encore est « indisponible » et caché du sélecteur.
import type { Component } from 'svelte';
import type { HomeModel, HomeRow } from './model.svelte';

export type OsStyleId = 'rustv' | 'googletv' | 'tizen' | 'webos' | 'vidaa' | 'titan' | 'fire' | 'tvos' | 'roku';
export type Dir = 'up' | 'down' | 'left' | 'right';

/**
 * Emplacements de widgets : 'status' = bande compacte en haut à droite (horloge, météo, téléphones, widgets de mods),
 * 'hero' = par-dessus la zone héros, 'bar' = à côté de la barre de lancement, 'corner-bl' / 'corner-br' = coins du bas.
 * Le champ `slot` d'un widget de mod vaut 'topbar' (= 'status') ou l'un de ces noms.
 */
export type WidgetSlot = 'status' | 'hero' | 'bar' | 'corner-bl' | 'corner-br';
export const WIDGET_SLOTS: WidgetSlot[] = ['status', 'hero', 'bar', 'corner-bl', 'corner-br'];

/** Emplacement demandé par un mod -> emplacement réel : lui-même si le style le fournit, sinon 'status', sinon le premier fourni. */
export function resolveWidgetSlot(requested: string | undefined, provided: WidgetSlot[]): WidgetSlot {
  const r = (requested === 'topbar' || !requested ? 'status' : requested) as WidgetSlot;
  if (provided.includes(r)) return r;
  return provided.includes('status') ? 'status' : (provided[0] ?? 'status');
}

export interface OsStyle {
  id: OsStyleId;
  /** Clé i18n du nom affiché (ex. « Style Google TV »). */
  nameKey: string;
  /** Clé i18n de la description d'une ligne. */
  descKey: string;
  /** Charge le composant de mise en page. */
  load: () => Promise<{ default: Component<{ model: HomeModel }> }>;
  /**
   * Variables CSS posées sur html[data-os="<id>"]. Jetons de palette (--bg, --bg-rgb, --text, --muted, --glass,
   * --glass-strong, --sheet, --field, --ring) : ignorés quand le thème clair de RusTV est choisi. Autres jetons
   * (--accent, --r, --font, --focus-scale, --focus-ring, ...) : toujours appliqués.
   */
  tokens: Record<string, string>;
  /** Famille de police à précharger (déclarée en @font-face dans le css du style). */
  font?: string;
  /** Vrai si le style respecte déjà prefers-reduced-motion de lui-même (sinon app.css le force quand même). */
  reducedMotionSafe?: boolean;
  /** Emplacements de widgets fournis par la mise en page (défaut : ['status']). Voir WidgetSlot et docs/OS-STYLES.md. */
  slots?: WidgetSlot[];
  /** Surcharge de la navigation : renvoie true si la touche est traitée (sinon model.move par défaut). */
  nav?: (dir: Dir, model: HomeModel) => boolean;
  /** Surcharge de la touche OK : renvoie true si traitée (sinon model.activate()). */
  activate?: (model: HomeModel) => boolean;
  /**
   * Transforme les rangées de base (reprendre, récentes, catégories) en rangées propres au style (onglets, grille...).
   * Appelée dans un $derived : peut lire model.ui de façon réactive. Les index de focus portent sur ce résultat.
   */
  rows?: (base: HomeRow[], model: HomeModel) => HomeRow[];
}

/** Ce qu'exporte (par défaut) un fichier <id>.meta.ts. */
export type OsStyleMeta = Omit<OsStyle, 'load'>;

export const STYLE_IDS: OsStyleId[] = ['rustv', 'googletv', 'tizen', 'webos', 'vidaa', 'titan', 'fire', 'tvos', 'roku'];

const metas = import.meta.glob<{ default: OsStyleMeta }>('./styles/*.meta.ts', { eager: true });
const loaders = import.meta.glob<{ default: Component<{ model: HomeModel }> }>('./styles/*.svelte');

const metaOf = (id: string) => metas[`./styles/${id}.meta.ts`]?.default;
const loaderOf = (id: string) => loaders[`./styles/${id}.svelte`];

export const STYLES: OsStyle[] = STYLE_IDS.map((id) => {
  const m = metaOf(id);
  const l = loaderOf(id);
  return {
    ...m,
    id,
    tokens: m?.tokens ?? {},
    nameKey: m?.nameKey ?? `os.${id}`,
    descKey: m?.descKey ?? `os.${id}.d`,
    load: l ?? (() => Promise.reject(new Error(`style ${id} not available`))),
  };
});

/** Vrai si le composant du style existe (fichier styles/<id>.svelte présent). */
export const isAvailable = (id: string) => !!loaderOf(id);

/** Styles proposés dans le sélecteur : ceux dont le fichier existe. */
export const availableStyles = (): OsStyle[] => STYLES.filter((s) => isAvailable(s.id));

/** Style demandé, ou « rustv » s'il n'est pas disponible (préférence venant d'une version plus récente, par exemple). */
export function resolveStyle(id: string): OsStyle {
  return availableStyles().find((s) => s.id === id) ?? STYLES[0];
}
