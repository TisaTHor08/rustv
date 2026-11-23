<script lang="ts">
  import { send } from '../../connection.svelte.ts';
  import { tick, tickSelect } from '../../haptics.ts';

  // Molette centrale (Magic Remote) : glisser verticalement = défiler (flèches haut / bas, un cran de haptique par pas),
  // tap = OK, appui long immobile = touche Menu.
  let { onok, label }: { onok: () => void; label: string } = $props();
  const STEP = 26;
  let down = false;
  let lastY = 0;
  let acc = 0;
  let moved = 0;
  let t0 = 0;
  let longTimer: number | undefined;
  let longFired = false;

  function pdown(e: PointerEvent) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    down = true;
    lastY = e.clientY;
    acc = 0;
    moved = 0;
    longFired = false;
    t0 = performance.now();
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
    const dy = e.clientY - lastY;
    lastY = e.clientY;
    moved += Math.abs(dy);
    acc += dy;
    while (Math.abs(acc) >= STEP) {
      send({ t: 'key', k: acc > 0 ? 'down' : 'up' });
      tickSelect();
      acc -= Math.sign(acc) * STEP;
    }
  }
  function pup() {
    if (!down) return;
    down = false;
    clearTimeout(longTimer);
    if (!longFired && moved < 10 && performance.now() - t0 < 300) {
      tick(true);
      onok();
    }
  }
</script>

<div class="wheel" onpointerdown={pdown} onpointermove={pmove} onpointerup={pup} onpointercancel={pup} role="application" aria-label={label}>
  <i>OK</i>
</div>

<style>
  .wheel {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    touch-action: none;
    background:
      repeating-conic-gradient(from 0deg, #ffffff1c 0 2deg, transparent 2deg 6deg),
      radial-gradient(circle at 50% 30%, #4a4e55, #2c2f34);
    box-shadow: 0 4px 12px #000a, inset 0 1px 0 #ffffff26;
  }
  i {
    position: absolute;
    inset: 24%;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-style: normal;
    font-weight: 700;
    letter-spacing: 0.05em;
    color: #e9ebef;
    background: radial-gradient(circle at 50% 30%, #3b3e44, #25272b);
    box-shadow: inset 0 2px 6px #000a, 0 1px 0 #ffffff1c;
    pointer-events: none;
  }
  .wheel:active i {
    background: var(--accent);
  }
</style>
