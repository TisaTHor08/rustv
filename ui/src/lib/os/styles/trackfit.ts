// Utilitaire commun (vidaa, titan) : calcule le décalage horizontal d'une piste pour que la carte ciblée reste visible.
// offsetLeft/offsetWidth ignorent les transformations (scale), le calcul reste donc stable pendant l'animation de focus.
export function fitTrack(track: HTMLElement | null | undefined, card: HTMLElement | null | undefined, cur: number, view: number, edge = 0): number {
  if (!track || !card) return cur;
  const left = card.offsetLeft;
  const right = left + card.offsetWidth;
  const first = (track.firstElementChild as HTMLElement | null)?.offsetLeft ?? 0;
  if (left <= first + 1) return 0;
  let tx = cur;
  if (right + edge > -tx + view) tx = -(right + edge - view);
  if (left - edge < -tx) tx = -(left - edge);
  return Math.min(0, tx);
}
