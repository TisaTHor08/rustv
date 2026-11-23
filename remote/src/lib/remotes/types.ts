// Contrat commun des modèles de télécommande (voir docs/REMOTE-MODELS.md).
import type { Component } from 'svelte';

/** Ce que la coque (Remote.svelte) donne à chaque modèle : le comportement partagé, pour que seul l'habillage change. */
export interface RemoteCtx {
  /** Touche OK : valide, ou confirme Alt+Tab s'il est ouvert. */
  ok(): void;
  /** Vrai pendant qu'un sélecteur d'onglets (Alt+Tab) est ouvert sur le PC. */
  altOpen: boolean;
  /** Courte bulle d'information (sans effet sur le PC). */
  toast(msg: string): void;
  /** Fonctions propres au modèle `rustv` (clavier, coffre, Alt+Tab) : les autres modèles les trouvent dans la bande d'outils de la coque. */
  keyboardOn: boolean;
  toggleKeyboard(): void;
  openVault(): void;
  tabSwitch(): void;
  alt(a: 'left' | 'right'): void;
}

export interface RemoteModel {
  /** Identifiant, identique à l'identifiant de style d'accueil TV du PC. */
  id: string;
  /** Nom affiché dans les réglages. */
  name: { fr: string; en: string };
  component: Component<{ ctx: RemoteCtx }>;
  /** Vrai si le modèle dessine lui-même clavier, coffre et Alt+Tab (c'est le cas du modèle `rustv`). */
  ownsUtilities?: boolean;
}
