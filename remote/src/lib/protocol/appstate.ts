// Logique d'état pure de la télécommande (sans runes Svelte) : testable sous Node.
import type { PcMsg } from './types.ts';

export interface PcState {
  pinned: boolean;
  textFocus: boolean;
  tvMode: boolean;
  osStyle: string;
}

/** Applique un message `state` du PC. Un `state` est toujours complet : il remplace tout, y compris `osStyle` absent = `rustv`. */
export function applyState(s: PcState, m: Extract<PcMsg, { t: 'state' }>) {
  s.pinned = m.pinned;
  s.textFocus = m.textFocus === true;
  s.tvMode = m.tvMode;
  s.osStyle = m.osStyle || 'rustv';
}

/** Liaison perdue : l'état de saisie du PC n'est plus connu, on ne garde jamais un « champ actif » périmé. */
export function resetOnDisconnect(s: PcState) {
  s.textFocus = false;
}

/**
 * Faut-il afficher la barre de saisie ? `dismissed` = l'utilisateur l'a fermée pour le champ actuel ;
 * il retombe à faux dès que le PC annonce qu'il n'y a plus de champ (voir `nextDismissed`).
 */
export function keyboardVisible(o: { live: boolean; textFocus: boolean; manual: boolean; dismissed: boolean }): boolean {
  return o.live && (o.manual || (o.textFocus && !o.dismissed));
}

export function nextDismissed(dismissed: boolean, textFocus: boolean): boolean {
  return textFocus ? dismissed : false;
}
