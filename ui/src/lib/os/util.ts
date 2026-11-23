// Petits utilitaires communs aux mises en page de style.
export const reducedMotion = () => matchMedia('(prefers-reduced-motion: reduce)').matches;

/** Fait défiler l'élément ciblé dans la vue, en douceur sauf si l'utilisateur a demandé moins de mouvement. */
export function scrollFocus(el: Element | null | undefined, block: ScrollLogicalPosition = 'nearest') {
  el?.scrollIntoView({ block, inline: 'nearest', behavior: reducedMotion() ? 'auto' : 'smooth' });
}

/** Couleur de texte lisible sur une tuile (même règle que le style RusTV). */
export const tileFg = (darkText?: boolean) => (darkText ? '#16130a' : '#ffffff');
