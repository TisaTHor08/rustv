// Points d'extension « screens » : écrans communs que chaque style d'interface peut redessiner (voir docs/OS-STYLES.md).
// La LOGIQUE (touches, données, actions) reste dans les composants partagés (Settings.svelte, Search.svelte, TileMenu.svelte,
// Pairing.svelte, os/QuickHost.svelte, os/SystemLayer.svelte...). Un écran de style ne fait que DESSINER les props reçues et
// appeler les callbacks fournis : il ne gère pas les touches (sauf pour le clavier à l'écran de la recherche, voir `Search`).
import type { Component } from 'svelte';
import type { DTile } from '../tiles.svelte';

export type ScreenName = 'settings' | 'search' | 'tileMenu' | 'pairing' | 'quick' | 'osd' | 'ambient' | 'toast' | 'launch';
export const SCREEN_NAMES: ScreenName[] = ['settings', 'search', 'tileMenu', 'pairing', 'quick', 'osd', 'ambient', 'toast', 'launch'];

/** Ligne de Réglages (copie en lecture seule de `Row` dans Settings.svelte). */
export interface SettingsRowView {
  label: string;
  value?: string;
  swatch?: string;
  /** Vrai si les flèches gauche/droite changent la valeur. */
  arrows?: boolean;
  /** Vrai si la ligne a une action (Entrée) ou un réglage (flèches). */
  actionable: boolean;
}

export interface SettingsScreenProps {
  /** Titre de la page courante (déjà traduit). */
  title: string;
  /** 'root' (liste des catégories) ou clé de la page ouverte ('appearance', 'display'...). */
  page: string;
  /** Catégories de la racine, dans l'ordre. `active` = catégorie qui contient la page ouverte. */
  categories: { key: string; label: string; active: boolean }[];
  /** Lignes de la page ouverte, ou (à la racine) aperçu des lignes de la catégorie surlignée (vide pour une action). */
  rows: SettingsRowView[];
  /** Index surligné : dans `categories` si `inCategories`, sinon dans `rows`. */
  index: number;
  /** Vrai si le focus est dans la colonne des catégories (page === 'root'). */
  inCategories: boolean;
  hint: string;
  /** Souris : survole / active une catégorie ou une ligne. */
  hoverCategory(i: number): void;
  openCategory(i: number): void;
  hoverRow(i: number): void;
  activateRow(i: number): void;
  /** Retour : page précédente, ou fermeture à la racine. */
  back(): void;
}

/** Ligne de résultat de recherche (déjà mise en forme par Search.svelte). */
export interface SearchRowView {
  section: string;
  title: string;
  sub?: string;
  badge?: string;
  color?: string;
  /** Source d'image prête à l'emploi (affiche, icône de tuile ou de chaîne), vide s'il n'y en a pas. */
  image?: string;
  /** Une affiche de film (hauteur > largeur) plutôt qu'une icône carrée. */
  poster?: boolean;
  chips?: { label: string; sub?: string; enabled: boolean }[];
}

export interface SearchScreenProps {
  query: string;
  /** Modifie la requête (clavier à l'écran). La saisie au clavier du téléphone arrive aussi dans `query`. */
  setQuery(v: string): void;
  rows: SearchRowView[];
  /** Index de la ligne ciblée et du bouton (chip) ciblé dans cette ligne. */
  index: number;
  chipIndex: number;
  loading: boolean;
  /** Libellé d'une section (clé `section` -> texte traduit). */
  sectionLabel(section: string): string;
  /** Textes d'état déjà traduits (vide s'il n'y a rien à dire). */
  notes: string[];
  placeholder: string;
  hint: string;
  /** Menu contextuel de programme TV ouvert par-dessus (null sinon). */
  menu: { title: string; items: string[]; index: number } | null;
  /** Le champ de saisie réel (visible ou caché) : l'écran DOIT appeler `bindInput(el)` pour que le clavier du téléphone marche. */
  bindInput(el: HTMLInputElement): void;
  hoverRow(i: number): void;
  activateRow(i: number): void;
  activateChip(row: number, chip: number): void;
  close(): void;
  /**
   * Zone des touches fléchées : 'results' (comportement par défaut, listes verticales) ou 'keyboard' (l'écran gère un clavier
   * à l'écran). Un écran avec clavier met `onKeyboardKey` pour recevoir les flèches/OK pendant que `keyboardActive()` est vrai.
   */
  setKeyboardHandler(h: ((e: KeyboardEvent) => boolean) | null): void;
}

export interface TileMenuItemView {
  label: string;
  value?: string;
  swatch?: string;
  /** Vrai si les flèches gauche/droite changent la valeur. */
  arrows: boolean;
}

export interface TileMenuScreenProps {
  tile: DTile;
  items: TileMenuItemView[];
  index: number;
  hint: string;
  /** Source d'icône prête à l'emploi (vide si la tuile n'a pas d'icône). */
  icon: string;
  hover(i: number): void;
  activate(i: number): void;
  close(): void;
}

export interface PairingScreenProps {
  /** SVG du QR code (HTML produit par le shell) ou vide pendant le chargement. */
  svg: string;
  paired: boolean;
  bluetooth: boolean;
  /** Téléphones déjà appairés (hors celui en cours d'appairage). */
  devices: { id: string; name: string; connected: boolean; armed: boolean }[];
  /** Éléments sélectionnables, dans l'ordre : appareils, « nouveau QR », puis « fermer » si `embedded`. */
  index: number;
  embedded: boolean;
  hover(i: number): void;
  activate(i: number): void;
  newCode(): void;
  close(): void;
}

export interface QuickItemView {
  kind: 'slider' | 'choice' | 'action' | 'info';
  id: 'volume' | 'output' | 'display' | 'phones' | 'settings' | 'exit' | string;
  label: string;
  /** Texte de valeur (nom de la sortie, « 40 % », état...). */
  value?: string;
  /** Pour un slider : 0 à 100. */
  level?: number;
  muted?: boolean;
  /** Sous-texte. */
  sub?: string;
}

export interface QuickScreenProps {
  items: QuickItemView[];
  index: number;
  time: string;
  date: string;
  phonesLabel: string;
  hint: string;
  title: string;
  hover(i: number): void;
  activate(i: number): void;
  /** -1 / +1 : change un slider ou un choix. */
  adjust(i: number, dir: -1 | 1): void;
  close(): void;
}

export interface OsdScreenProps {
  /** 0 à 100. */
  level: number;
  muted: boolean;
  /** Libellé traduit (« Volume », « Muet »). */
  label: string;
}

export interface AmbientScreenProps {
  time: string;
  date: string;
  /** Couleurs de la palette des tuiles (au moins 4), pour les dégradés. */
  palette: string[];
  weatherEnabled: boolean;
  phones: number;
  /** Ligne d'état traduite (ex. « 2 téléphones connectés »). */
  status: string;
}

export interface ToastScreenProps {
  message: string;
}

export interface LaunchScreenProps {
  name: string;
  color: string;
  color2: string;
  slow: boolean;
  opening: string;
  cancelHint: string;
}

export interface ScreenProps {
  settings: SettingsScreenProps;
  search: SearchScreenProps;
  tileMenu: TileMenuScreenProps;
  pairing: PairingScreenProps;
  quick: QuickScreenProps;
  osd: OsdScreenProps;
  ambient: AmbientScreenProps;
  toast: ToastScreenProps;
  launch: LaunchScreenProps;
}

export type ScreenLoader<K extends ScreenName> = () => Promise<{ default: Component<ScreenProps[K]> }>;
/** Écrans fournis par un style (chargement différé). Tout ce qui est absent retombe sur l'écran RusTV habillé par les jetons. */
export type OsScreens = { [K in ScreenName]?: ScreenLoader<K> };
