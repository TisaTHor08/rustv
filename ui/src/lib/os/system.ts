// Logique pure de la couche système (testable sans interface) : déclenchement de la veille ambiante, palette, volume.

/** Valeurs de « Veille ambiante » proposées dans les réglages (minutes ; 0 = désactivée). */
export const AMBIENT_CHOICES = [0, 2, 5, 10, 30];

/** Vrai quand la veille ambiante doit démarrer : activée, pas déjà affichée, rien ne la bloque, et assez d'inactivité. */
export function ambientDue(o: { minutes: number; idleMs: number; active: boolean; blocked: boolean }): boolean {
  return o.minutes > 0 && !o.active && !o.blocked && o.idleMs >= o.minutes * 60_000;
}

/** Au moins 4 couleurs distinctes tirées des tuiles (ordre stable), complétées par la palette de secours. */
export function paletteOf(tileColors: string[], fallback: string[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  // on prend des couleurs réparties dans la liste plutôt que les 4 premières (souvent voisines)
  const step = Math.max(1, Math.floor(tileColors.length / 6));
  for (let i = 0; i < tileColors.length; i += step) {
    const c = tileColors[i];
    if (c && !seen.has(c)) (seen.add(c), out.push(c));
  }
  for (const c of fallback) if (out.length < 4 && !seen.has(c)) (seen.add(c), out.push(c));
  return out.slice(0, 6);
}

/** Pas de volume des flèches du panneau rapide. */
export const VOLUME_STEP = 5;
export const clampVolume = (n: number) => Math.max(0, Math.min(100, Math.round(n)));
