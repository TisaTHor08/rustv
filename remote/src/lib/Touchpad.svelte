<script lang="ts">
  import { send } from './connection.svelte.ts';
  import { tick } from './haptics.ts';

  // Suivi des doigts : 1 doigt = déplacement, tap = clic gauche, tap à 2 doigts = clic droit,
  // glisser à 2 doigts = défilement.
  let last = new Map<number, { x: number; y: number }>();
  let moved = 0;
  let maxFingers = 0;
  let startedAt = 0;
  let acc = { x: 0, y: 0 };
  let sentAt = 0;

  function down(e: PointerEvent) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    if (last.size === 0) {
      moved = 0;
      maxFingers = 0;
      startedAt = performance.now();
    }
    last.set(e.pointerId, { x: e.clientX, y: e.clientY });
    maxFingers = Math.max(maxFingers, last.size);
  }

  function move(e: PointerEvent) {
    const p = last.get(e.pointerId);
    if (!p) return;
    const dx = e.clientX - p.x;
    const dy = e.clientY - p.y;
    last.set(e.pointerId, { x: e.clientX, y: e.clientY });
    moved += Math.abs(dx) + Math.abs(dy);
    // légère accélération pour les mouvements rapides
    const speed = Math.hypot(dx, dy);
    const gain = speed > 12 ? 2.2 : speed > 5 ? 1.5 : 1;
    acc.x += dx * gain;
    acc.y += dy * gain;
    const now = performance.now();
    if (now - sentAt < 16) return; // ~60 messages/s maximum
    sentAt = now;
    const ix = Math.trunc(acc.x);
    const iy = Math.trunc(acc.y);
    acc.x -= ix;
    acc.y -= iy;
    if (last.size >= 2) {
      if (iy) send({ t: 'mouse', dx: 0, dy: 0, scroll: Math.round(-iy * 4) });
    } else if (ix || iy) send({ t: 'mouse', dx: ix, dy: iy });
  }

  function up(e: PointerEvent) {
    if (!last.has(e.pointerId)) return;
    last.delete(e.pointerId);
    if (last.size === 0) {
      const short = performance.now() - startedAt < 250;
      if (short && moved < 10) {
        tick();
        send({ t: 'mouse', dx: 0, dy: 0, click: maxFingers >= 2 ? 'right' : 'left' });
      }
      acc = { x: 0, y: 0 };
    }
  }
</script>

<div class="pad" onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} role="application" aria-label="Pavé tactile">
  <span>Glisser = déplacer · Tap = clic · 2 doigts = clic droit / défiler</span>
</div>
<div class="row">
  <button onpointerdown={(e) => { e.preventDefault(); tick(); send({ t: 'mouse', dx: 0, dy: 0, click: 'left' }); }}>Clic gauche</button>
  <button onpointerdown={(e) => { e.preventDefault(); tick(); send({ t: 'mouse', dx: 0, dy: 0, click: 'right' }); }}>Clic droit</button>
</div>

<style>
  .pad {
    flex: 1;
    background: var(--panel);
    border-radius: 20px;
    display: grid;
    place-items: center;
    touch-action: none;
    padding: 20px;
    text-align: center;
  }
  span {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .row {
    display: flex;
    gap: 10px;
  }
  .row button {
    flex: 1;
    padding: 18px;
  }
</style>
