// Tests de la logique d'état de la télécommande (sans réseau) : node scripts/state-check.ts
import assert from 'node:assert/strict';
import { applyState, keyboardVisible, nextDismissed, resetOnDisconnect, type PcState } from '../src/lib/protocol/appstate.ts';

const fresh = (): PcState => ({ pinned: false, textFocus: false, tvMode: false, osStyle: 'rustv' });
const st = (textFocus: boolean, osStyle?: string) => ({ t: 'state' as const, pinned: false, textFocus, tvMode: true, osStyle });

// Champ actif puis plus de champ : l'état suit ; un textFocus absent/invalide vaut faux.
{
  const s = fresh();
  applyState(s, st(true));
  assert.equal(s.textFocus, true);
  applyState(s, st(false));
  assert.equal(s.textFocus, false);
  applyState(s, { ...st(true), textFocus: undefined as unknown as boolean });
  assert.equal(s.textFocus, false);
}

// Déconnexion / reconnexion : jamais de champ périmé ; le nouvel état complet fait foi.
{
  const s = fresh();
  applyState(s, st(true, 'tizen'));
  resetOnDisconnect(s);
  assert.equal(s.textFocus, false);
  assert.equal(s.osStyle, 'tizen'); // le style est gardé pendant la reconnexion (pas de clignotement de modèle)
  applyState(s, st(false));
  assert.equal(s.osStyle, 'rustv');
}

// Bascule de style 10 fois, y compris retour à « absent » : toujours cohérent, textFocus intact.
{
  const s = fresh();
  const styles = ['tizen', 'webos', undefined, 'roku', 'vidaa', 'fire', undefined, 'titan', 'tvos', 'googletv'];
  for (const x of styles) {
    applyState(s, st(false, x));
    assert.equal(s.osStyle, x ?? 'rustv');
    assert.equal(s.textFocus, false);
  }
}

// Reprise après mise en veille : un état tardif (ping) remet l'affichage à jour, dans les deux sens.
{
  const s = fresh();
  applyState(s, st(true));
  applyState(s, st(false)); // réponse au ping de reprise
  assert.equal(keyboardVisible({ live: true, textFocus: s.textFocus, manual: false, dismissed: false }), false);
}

// Barre de saisie : fermeture pour le champ courant, retour au champ suivant, jamais sans liaison.
{
  assert.equal(keyboardVisible({ live: true, textFocus: true, manual: false, dismissed: false }), true);
  assert.equal(keyboardVisible({ live: true, textFocus: true, manual: false, dismissed: true }), false);
  assert.equal(keyboardVisible({ live: true, textFocus: false, manual: true, dismissed: false }), true);
  assert.equal(keyboardVisible({ live: false, textFocus: true, manual: true, dismissed: false }), false);
  let d = true;
  d = nextDismissed(d, true);
  assert.equal(d, true); // même champ : reste fermée
  d = nextDismissed(d, false);
  assert.equal(d, false); // plus de champ : prête pour le suivant
}
console.log('state-check ok');
