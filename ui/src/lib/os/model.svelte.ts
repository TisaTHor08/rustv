// Contrat entre Home.svelte (logique, touches, fenêtres) et les mises en page de style (src/lib/os/styles/*.svelte).
// Home construit UN objet HomeModel (getters réactifs + callbacks) et le passe à la mise en page active.
import type { ContinueEntry } from '../player.svelte';
import type { DTile } from '../tiles.svelte';

export interface HomeRow {
  key: string;
  title: string;
  tiles: DTile[];
}

export interface HomeHero {
  title: string;
  desc: string;
  /** Libellé de catégorie déjà traduit (vide s'il n'y a pas de tuile ciblée). */
  category: string;
  color: string;
  color2: string;
  /** Tuile ciblée (undefined quand le focus est sur l'icône Réglages). */
  tile?: DTile;
}

export interface HomeModel {
  /** Rangées affichées (après le hook `rows` du style actif). */
  readonly rows: HomeRow[];
  /** Position du focus. row = -1 : icône Réglages (barre du haut du style RusTV). */
  readonly focus: { row: number; col: number };
  readonly focused: DTile | undefined;
  /** Zone de focus : 'content' par défaut ; un style peut définir d'autres zones ('tabs', 'menu'...). */
  readonly zone: string;
  /** Dernière direction (up/down/left/right) : sert à l'inclinaison 3D du style RusTV. */
  readonly tilt: string;
  readonly time: string;
  readonly date: string;
  /** Nombre de téléphones connectés et libellé déjà traduit. */
  readonly phones: number;
  readonly phonesLabel: string;
  /** Réglage « météo activée » : le composant Weather.svelte s'affiche tout seul dans les barres. */
  readonly weatherEnabled: boolean;
  /** Réglage « position des widgets » : 'auto' = le style place lui-même horloge, météo, téléphones et widgets de mods. */
  readonly widgetPos: 'auto' | 'tr' | 'tl' | 'br' | 'bl' | 'hidden';
  /** Vrai quand le style doit afficher SON groupe de widgets (widgetPos === 'auto'). Sinon Home affiche un dock (WidgetDock). */
  readonly widgetsNative: boolean;
  /** Tuiles récemment ouvertes, plus récente d'abord. */
  readonly recent: DTile[];
  /** Entrées « Reprendre la lecture » (position, durée, affiche). */
  readonly continueEntries: ContinueEntry[];
  /** Toutes les tuiles visibles, dans l'ordre des catégories (pour une grille « Apps » par exemple). */
  readonly allTiles: DTile[];
  /** Réglage « ambiance colorée » (prefs.tint) actif et une tuile ciblée. */
  readonly tinted: boolean;
  readonly hero: HomeHero;
  /** Sac d'état propre au style ($state côté Home) : onglet courant, entrée de menu... Lu par nav()/rows(). */
  readonly ui: Record<string, any>;

  /** Déplacement par défaut (rangées + colonnes) : joue le son, borne, met à jour le focus. */
  move(dRow: number, dCol: number, dir: string): void;
  /** Place le focus. `silent` : pas de son (survol souris). Joue le son 'move' si la position change. */
  focusAt(row: number, col: number, silent?: boolean): void;
  /** Change de zone de focus (son 'move'). Retour depuis une zone ramène à 'content'. */
  setZone(zone: string): void;
  /** OK sur la tuile ciblée (reprise d'une lecture, ouverture d'une app...). */
  activate(): void;
  /** Ouvre une tuile (animation de lancement, pages internes, apps). */
  open(tile: DTile): void;
  /** Reprend une lecture de « Reprendre la lecture » par son URL. */
  resume(url: string): void;
  openSearch(): void;
  openSettings(): void;
  /** Pages internes : TV en direct et médiathèque (onglet). */
  openLive(): void;
  openMedia(tab: 'movies' | 'series' | 'photos' | 'music' | 'files'): void;
  /** Menu contextuel (touche ContextMenu) ; pour « Reprendre » : retire l'entrée. */
  contextMenu(tile: DTile): void;
  /** Retrouve une tuile par id (ex. 'live', 'media'). */
  tileById(id: string): DTile | undefined;
  /** Message temporaire (toast). */
  note(msg: string): void;
}
