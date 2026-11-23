// Libellés accessibles (aria-label) des modèles, en français et en anglais.
import { lang } from '../i18n.ts';

type L = { fr: string; en: string };
const BASE: Record<string, L> = {
  up: { fr: 'Haut', en: 'Up' },
  down: { fr: 'Bas', en: 'Down' },
  left: { fr: 'Gauche', en: 'Left' },
  right: { fr: 'Droite', en: 'Right' },
  ok: { fr: 'OK', en: 'OK' },
  back: { fr: 'Retour', en: 'Back' },
  home: { fr: 'Accueil', en: 'Home' },
  menu: { fr: 'Menu', en: 'Menu' },
  options: { fr: 'Options', en: 'Options' },
  settings: { fr: 'Réglages', en: 'Settings' },
  assistant: { fr: 'Assistant vocal (recherche)', en: 'Voice assistant (search)' },
  mute: { fr: 'Muet', en: 'Mute' },
  volUp: { fr: 'Volume plus', en: 'Volume up' },
  volDown: { fr: 'Volume moins', en: 'Volume down' },
  chUp: { fr: 'Chaîne suivante (média suivant)', en: 'Channel up (next media)' },
  chDown: { fr: 'Chaîne précédente (média précédent)', en: 'Channel down (previous media)' },
  play: { fr: 'Lecture / pause', en: 'Play / pause' },
  rew: { fr: 'Retour rapide (précédent)', en: 'Rewind (previous)' },
  ff: { fr: 'Avance rapide (suivant)', en: 'Fast forward (next)' },
  replay: { fr: 'Relecture (précédent)', en: 'Instant replay (previous)' },
  pointer: { fr: 'Mode pointeur', en: 'Pointer mode' },
  arrows: { fr: 'Mode flèches', en: 'Arrow mode' },
  wheel: { fr: 'Molette : glisser pour défiler, toucher pour valider', en: 'Wheel: swipe to scroll, tap to select' },
  clickpad: { fr: 'Pavé tactile : glisser pour naviguer, toucher pour valider', en: 'Clickpad: swipe to navigate, tap to select' },
  input: { fr: 'Source (accueil)', en: 'Input (home)' },
  red: { fr: 'Rouge (arrêt)', en: 'Red (stop)' },
  green: { fr: 'Vert (lecture / pause)', en: 'Green (play / pause)' },
  yellow: { fr: 'Jaune (précédent)', en: 'Yellow (previous)' },
  blue: { fr: 'Bleu (suivant)', en: 'Blue (next)' },
  ambilight: { fr: 'Ambilight (sans effet pour l’instant)', en: 'Ambilight (no effect for now)' },
  keyboard: { fr: 'Clavier', en: 'Keyboard' },
  vault: { fr: 'Coffre Bitwarden', en: 'Bitwarden vault' },
  tabs: { fr: 'Onglets', en: 'Tabs' },
  shortcut: { fr: 'Raccourci streaming {n}', en: 'Streaming shortcut {n}' },
};

/** Particularités de chaque modèle (le bouton accueil ne s'appelle pas pareil partout). */
const OVERRIDE: Record<string, Record<string, L>> = {
  tizen: { home: { fr: 'Accueil (hub)', en: 'Home hub' } },
  tvos: { home: { fr: 'Accueil (TV)', en: 'Home (TV)' } },
  fire: { menu: { fr: 'Menu (trois traits)', en: 'Menu (three lines)' } },
  roku: { options: { fr: 'Options (astérisque)', en: 'Options (star)' } },
  vidaa: { settings: { fr: 'Réglages rapides', en: 'Quick settings' } },
};

export function labels(model: string): (k: string, n?: number) => string {
  return (k, n) => {
    const e = OVERRIDE[model]?.[k] ?? BASE[k];
    if (!e) return k;
    return e[lang].replace('{n}', String(n ?? ''));
  };
}

/** Texte de la bulle du bouton Ambilight (Titan OS). */
export const AMBILIGHT_TOAST: L = {
  fr: 'Ambilight : disponible quand Hyperion est activé sur le PC',
  en: 'Ambilight: available when Hyperion is enabled on the PC',
};
export const toastText = (l: L) => l[lang];
