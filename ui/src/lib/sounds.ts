// Petits sons d'interface synthétisés (Web Audio) : aucun fichier, aucun réseau.
import { prefs } from './prefs.svelte';

export type SoundName = 'move' | 'select' | 'back' | 'error';

interface Blip {
  /** Fréquence de départ puis d'arrivée (Hz). */
  from: number;
  to: number;
  /** Durée en secondes. */
  dur: number;
  type: OscillatorType;
  /** Gain de crête (avant volume général). */
  peak: number;
  /** Deuxième note facultative, jouée juste après. */
  next?: Blip;
  delay?: number;
}

const BLIPS: Record<SoundName, Blip> = {
  move: { from: 1900, to: 1500, dur: 0.04, type: 'sine', peak: 0.35 },
  select: { from: 1050, to: 880, dur: 0.075, type: 'triangle', peak: 0.55 },
  back: { from: 900, to: 480, dur: 0.09, type: 'sine', peak: 0.5 },
  error: { from: 300, to: 220, dur: 0.1, type: 'square', peak: 0.2, delay: 0.11, next: { from: 300, to: 200, dur: 0.12, type: 'square', peak: 0.2 } },
};

let ctx: AudioContext | null = null;
let master: GainNode | null = null;

function context(): AudioContext | null {
  if (!ctx) {
    try {
      ctx = new AudioContext();
      master = ctx.createGain();
      master.connect(ctx.destination);
    } catch {
      return null;
    }
  }
  if (ctx.state === 'suspended') void ctx.resume().catch(() => {});
  return ctx;
}

function blip(c: AudioContext, out: AudioNode, b: Blip, at: number) {
  const osc = c.createOscillator();
  const g = c.createGain();
  osc.type = b.type;
  osc.frequency.setValueAtTime(b.from, at);
  osc.frequency.exponentialRampToValueAtTime(b.to, at + b.dur);
  // enveloppe : attaque 4 ms puis décroissance exponentielle (pas de clic parasite)
  g.gain.setValueAtTime(0.0001, at);
  g.gain.linearRampToValueAtTime(b.peak, at + 0.004);
  g.gain.exponentialRampToValueAtTime(0.0001, at + b.dur);
  osc.connect(g).connect(out);
  osc.start(at);
  osc.stop(at + b.dur + 0.02);
  if (b.next) blip(c, out, b.next, at + (b.delay ?? b.dur));
}

/** Joue un son (sans effet si désactivé ou si l'audio est indisponible). */
export function play(name: SoundName) {
  if (!prefs.sounds || prefs.soundVolume <= 0) return;
  const c = context();
  if (!c || !master) return;
  // courbe quadratique : le curseur 0-100 reste perceptivement progressif
  const v = Math.min(1, Math.max(0, prefs.soundVolume / 100));
  master.gain.value = v * v;
  blip(c, master, BLIPS[name], c.currentTime + 0.005);
}

// Les navigateurs exigent un geste utilisateur pour démarrer l'audio : on l'amorce à la première touche.
if (typeof window !== 'undefined') {
  const unlock = () => {
    context();
    window.removeEventListener('keydown', unlock, true);
    window.removeEventListener('pointerdown', unlock, true);
  };
  window.addEventListener('keydown', unlock, true);
  window.addEventListener('pointerdown', unlock, true);
}
