import { Haptics, ImpactStyle } from '@capacitor/haptics';
import { isNative } from './native.ts';

export function tick(strong = false) {
  if (!isNative) return;
  void Haptics.impact({ style: strong ? ImpactStyle.Medium : ImpactStyle.Light }).catch(() => {});
}

/** Action Svelte : appui + répétition automatique en maintenant (flèches). */
export function repeat(node: HTMLElement, fire: () => void) {
  let t1: number | undefined;
  let t2: number | undefined;
  const stop = () => {
    clearTimeout(t1);
    clearInterval(t2);
  };
  const down = (e: PointerEvent) => {
    e.preventDefault();
    fire();
    tick();
    t1 = window.setTimeout(() => {
      t2 = window.setInterval(() => {
        fire();
        tick();
      }, 90);
    }, 400);
  };
  node.addEventListener('pointerdown', down);
  for (const ev of ['pointerup', 'pointerleave', 'pointercancel']) node.addEventListener(ev, stop);
  return {
    destroy() {
      stop();
      node.removeEventListener('pointerdown', down);
    },
  };
}

/** Action Svelte : tap court = `onTap`, appui long (>500 ms) = `onLong`. */
export function longpress(node: HTMLElement, o: { onTap: () => void; onLong: () => void }) {
  let timer: number | undefined;
  let fired = false;
  const down = (e: PointerEvent) => {
    e.preventDefault();
    fired = false;
    timer = window.setTimeout(() => {
      fired = true;
      tick(true);
      o.onLong();
    }, 500);
  };
  const up = () => {
    clearTimeout(timer);
    if (!fired && timer !== undefined) {
      tick();
      o.onTap();
    }
    timer = undefined;
  };
  const cancel = () => {
    clearTimeout(timer);
    timer = undefined;
  };
  node.addEventListener('pointerdown', down);
  node.addEventListener('pointerup', up);
  node.addEventListener('pointerleave', cancel);
  node.addEventListener('pointercancel', cancel);
  return {
    destroy() {
      cancel();
      node.removeEventListener('pointerdown', down);
      node.removeEventListener('pointerup', up);
      node.removeEventListener('pointerleave', cancel);
      node.removeEventListener('pointercancel', cancel);
    },
  };
}
