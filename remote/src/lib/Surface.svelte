<script lang="ts">
  import { onDestroy } from 'svelte';
  import { t } from './i18n.ts';
  import { send } from './connection.svelte.ts';
  import { tick, tickSelect } from './haptics.ts';

  // Surface tactile façon Siri Remote : un glissement déplace le focus pas à pas (un cran de
  // haptique par pas), un geste rapide continue avec de l'inertie, un tap valide.
  // `clickpad` (modèle tvOS) : pavé rond ; un clic sur le bord = flèche, au centre = OK.
  let { variant = 'default' }: { variant?: 'default' | 'clickpad' } = $props();
  const STEP = 54; // pixels de glissement par pas de focus
  type Dir = 'up' | 'down' | 'left' | 'right';

  let el: HTMLDivElement;
  let down = false;
  let last = { x: 0, y: 0, t: 0 };
  let acc = { x: 0, y: 0 };
  let v = { x: 0, y: 0 }; // vitesse lissée (px/ms)
  let moved = 0;
  let t0 = 0;
  let dot = $state<{ x: number; y: number } | null>(null);
  let inertiaTimer: number | undefined;
  let longTimer: number | undefined;
  let longFired = false;

  function step(k: Dir) {
    send({ t: 'key', k });
    tickSelect();
  }

  function flush() {
    // On avance sur l'axe dominant tant qu'on a accumulé au moins un pas.
    for (;;) {
      const ax = Math.abs(acc.x), ay = Math.abs(acc.y);
      if (ax >= STEP && ax >= ay) {
        step(acc.x > 0 ? 'right' : 'left');
        acc.x -= Math.sign(acc.x) * STEP;
        acc.y *= 0.4;
      } else if (ay >= STEP) {
        step(acc.y > 0 ? 'down' : 'up');
        acc.y -= Math.sign(acc.y) * STEP;
        acc.x *= 0.4;
      } else break;
    }
  }

  function pdown(e: PointerEvent) {
    clearTimeout(inertiaTimer);
    el.setPointerCapture(e.pointerId);
    down = true;
    acc = { x: 0, y: 0 };
    v = { x: 0, y: 0 };
    moved = 0;
    t0 = performance.now();
    last = { x: e.clientX, y: e.clientY, t: t0 };
    const r = el.getBoundingClientRect();
    dot = { x: e.clientX - r.left, y: e.clientY - r.top };
    // Appui long immobile = touche « Menu » (options de la tuile sur l'accueil TV).
    longFired = false;
    clearTimeout(longTimer);
    longTimer = window.setTimeout(() => {
      if (down && moved < 10) {
        longFired = true;
        tick(true);
        send({ t: 'key', k: 'menu' });
      }
    }, 500);
  }

  function pmove(e: PointerEvent) {
    if (!down) return;
    const now = performance.now();
    const dx = e.clientX - last.x, dy = e.clientY - last.y;
    const dt = Math.max(1, now - last.t);
    v = { x: v.x * 0.7 + (dx / dt) * 0.3, y: v.y * 0.7 + (dy / dt) * 0.3 };
    last = { x: e.clientX, y: e.clientY, t: now };
    moved += Math.abs(dx) + Math.abs(dy);
    acc.x += dx;
    acc.y += dy;
    const r = el.getBoundingClientRect();
    dot = { x: e.clientX - r.left, y: e.clientY - r.top };
    flush();
  }

  function pup() {
    if (!down) return;
    down = false;
    dot = null;
    clearTimeout(longTimer);
    if (longFired) return;
    if (moved < 10 && performance.now() - t0 < 300) {
      tick(true);
      if (variant === 'clickpad') {
        const r = el.getBoundingClientRect();
        const dx = last.x - (r.left + r.width / 2), dy = last.y - (r.top + r.height / 2);
        if (Math.hypot(dx, dy) > r.width * 0.34) {
          send({ t: 'key', k: Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? 'right' : 'left') : dy > 0 ? 'down' : 'up' });
          return;
        }
      }
      send({ t: 'key', k: 'enter' });
      return;
    }
    // Inertie : un geste rapide continue quelques pas en ralentissant.
    const speed = Math.hypot(v.x, v.y);
    if (speed < 1.1) return;
    const k: Dir = Math.abs(v.x) > Math.abs(v.y) ? (v.x > 0 ? 'right' : 'left') : v.y > 0 ? 'down' : 'up';
    let n = Math.min(6, Math.floor(speed * 2.2));
    let delay = 70;
    const run = () => {
      if (n-- <= 0) return;
      step(k);
      delay *= 1.25;
      inertiaTimer = window.setTimeout(run, delay);
    };
    run();
  }
  // Changement de modèle de télécommande en cours de geste : plus aucun minuteur ne doit envoyer de touche après le démontage.
  onDestroy(() => {
    down = false;
    clearTimeout(longTimer);
    clearTimeout(inertiaTimer);
  });
</script>

<div bind:this={el} class="surface" class:clickpad={variant === 'clickpad'} onpointerdown={pdown} onpointermove={pmove} onpointerup={pup} onpointercancel={pup} role="application" aria-label="Surface tactile">
  {#if dot}<i style:left="{dot.x}px" style:top="{dot.y}px"></i>{/if}
  <span>{t('surface.1')}<br />{t('surface.2')}<br />{t('surface.3')}</span>
</div>

<style>
  .surface {
    position: relative;
    flex: 1;
    min-height: 220px;
    border-radius: 28px;
    background: radial-gradient(120% 90% at 50% 0%, #2a3243, #161b25);
    box-shadow: inset 0 0 0 1px #ffffff14, 0 10px 30px #0006;
    display: grid;
    place-items: center;
    overflow: hidden;
    touch-action: none;
  }
  .clickpad {
    flex: none;
    min-height: 0;
    width: var(--cp-size, min(100%, 280px));
    aspect-ratio: 1;
    margin: 0 auto;
    border-radius: 50%;
    background: var(--cp-bg, radial-gradient(120% 90% at 50% 0%, #2a3243, #161b25));
    box-shadow: var(--cp-shadow, inset 0 0 0 1px #ffffff14, 0 10px 30px #0006);
  }
  .clickpad span {
    color: var(--cp-ink, #8a94a6);
  }
  span {
    color: #8a94a6;
    text-align: center;
    font-size: 0.9rem;
    line-height: 1.5;
    pointer-events: none;
  }
  i {
    position: absolute;
    width: 74px;
    height: 74px;
    margin: -37px 0 0 -37px;
    border-radius: 50%;
    background: radial-gradient(circle, #ffffff40, #ffffff00 70%);
    pointer-events: none;
  }
</style>
