// Clavier à l'écran AZERTY/QWERTY partagé par les écrans de recherche Tizen et webOS (logique pure, sans interface).
export const KB_ROWS = ['1234567890', 'qwertyuiop', 'asdfghjkl', 'zxcvbnm'];
export const KB_ACTIONS = ['space', 'delete', 'clear'] as const;
export type KbAction = (typeof KB_ACTIONS)[number];
export interface KbPos {
  r: number;
  c: number;
}
/** Nombre de touches d'une rangée (la dernière rangée est celle des actions). */
export const rowLen = (r: number) => (r >= KB_ROWS.length ? KB_ACTIONS.length : KB_ROWS[r].length);
/** Libellé ou caractère de la touche (r, c). */
export const keyAt = (p: KbPos): string => (p.r >= KB_ROWS.length ? KB_ACTIONS[p.c] : KB_ROWS[p.r][p.c]);
/**
 * Déplace le curseur du clavier. `edge` indique qu'on sort du clavier (vers les résultats à droite ou au-dessus, ou vers le champ).
 * La colonne est proportionnelle entre rangées de longueurs différentes.
 */
export function kbMove(p: KbPos, dir: 'ArrowUp' | 'ArrowDown' | 'ArrowLeft' | 'ArrowRight'): { pos: KbPos; edge?: 'up' | 'right' } {
  const last = KB_ROWS.length;
  if (dir === 'ArrowLeft') return { pos: p.c > 0 ? { r: p.r, c: p.c - 1 } : p };
  if (dir === 'ArrowRight') return p.c < rowLen(p.r) - 1 ? { pos: { r: p.r, c: p.c + 1 } } : { pos: p, edge: 'right' };
  const nr = p.r + (dir === 'ArrowUp' ? -1 : 1);
  if (nr < 0) return { pos: p, edge: 'up' };
  if (nr > last) return { pos: p };
  const ratio = rowLen(p.r) > 1 ? p.c / (rowLen(p.r) - 1) : 0;
  return { pos: { r: nr, c: Math.round(ratio * (rowLen(nr) - 1)) } };
}
/** Applique une touche à la requête. */
export function kbApply(query: string, p: KbPos): string {
  if (p.r < KB_ROWS.length) return query + keyAt(p);
  const a = KB_ACTIONS[p.c];
  if (a === 'space') return query.endsWith(' ') || !query ? query : query + ' ';
  if (a === 'delete') return query.slice(0, -1);
  return '';
}
