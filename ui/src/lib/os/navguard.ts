// Garde-fou de la navigation de l'accueil : logique pure (sans interface) qui ramène la position du focus et la zone à des
// valeurs valides pour le style actif et les rangées affichées. Appelée au changement de style, au retour de focus de la
// fenêtre et quand les rangées changent. Testée par ui/tests/nav.mjs.

export interface NavFocus {
  row: number;
  col: number;
}

/** Plus petit index de rangée valide : -1 (icône Réglages) n'existe que dans le style RusTV d'origine. */
export const minRow = (styleId: string): number => (styleId === 'rustv' ? -1 : 0);

/** Zone valide pour ce style : 'content' dès que la zone demandée n'est pas gérée par lui. */
export function validZone(zone: string, zones: string[] | undefined): string {
  return zone === 'content' || (zones ?? ['content']).includes(zone) ? zone : 'content';
}

/** Borne le focus dans les rangées ; une rangée vide ou absente ramène à la première rangée non vide. */
export function clampFocus(styleId: string, rowLens: number[], f: NavFocus): NavFocus {
  const lo = minRow(styleId);
  if (!rowLens.length) return { row: Math.max(lo, 0), col: 0 };
  let row = Math.max(lo, Math.min(rowLens.length - 1, Number.isFinite(f.row) ? Math.trunc(f.row) : 0));
  if (row >= 0 && rowLens[row] === 0) {
    const alt = rowLens.findIndex((n) => n > 0);
    if (alt >= 0) row = alt;
  }
  const col = row < 0 ? 0 : Math.max(0, Math.min(Math.max(0, rowLens[row] - 1), Number.isFinite(f.col) ? Math.trunc(f.col) : 0));
  return { row, col };
}
