<script lang="ts">
  import { app, send } from './connection.svelte.ts';
  import { repeat, tick, tickSelect } from './haptics.ts';
  import { t } from './i18n.ts';

  // Volume du PC synchronisé : la barre reflète le volume Windows en direct et se règle au doigt.
  let track: HTMLDivElement;
  let drag = $state<number | null>(null);
  let lastSent = 0;
  let lastStep = -1;

  const level = $derived(drag ?? app.volume.level);
  const muted = $derived(app.volume.muted || level === 0);

  function at(e: PointerEvent) {
    const r = track.getBoundingClientRect();
    return Math.round(Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * 100);
  }
  function move(e: PointerEvent) {
    const v = at(e);
    drag = v;
    const step = Math.round(v / 5);
    if (step !== lastStep) {
      lastStep = step;
      tickSelect();
    }
    const now = Date.now();
    if (now - lastSent > 60) {
      lastSent = now;
      send({ t: 'setVolume', level: v });
    }
  }
  function down(e: PointerEvent) {
    e.preventDefault();
    track.setPointerCapture(e.pointerId);
    lastStep = -1;
    move(e);
  }
  function up(e: PointerEvent) {
    if (drag === null) return;
    send({ t: 'setVolume', level: drag });
    drag = null;
    track.releasePointerCapture(e.pointerId);
  }
</script>

<div class="vol">
  <button class="mute" class:off={muted} aria-label={t('vol.mute')} onpointerdown={(e) => { e.preventDefault(); tick(true); send({ t: 'media', a: 'mute' }); }}>
    <svg viewBox="0 0 24 24">
      <path d="M4 9v6h4l5 4V5L8 9z" />
      {#if muted}<path d="M16 9l5 6M21 9l-5 6" />{:else}<path d="M16 8.5a5 5 0 010 7M18.5 6a8.5 8.5 0 010 12" />{/if}
    </svg>
  </button>
  <button class="step" aria-label={t('vol.down')} use:repeat={() => send({ t: 'media', a: 'volDown' })}>−</button>
  <div class="track" bind:this={track} onpointerdown={down} onpointermove={(e) => drag !== null && move(e)} onpointerup={up} onpointercancel={up} role="slider" tabindex="-1" aria-valuenow={level} aria-valuemin="0" aria-valuemax="100">
    <div class="fill" style:width="{level}%"></div>
    <span class="num">{app.volume.known || drag !== null ? level : ''}</span>
  </div>
  <button class="step" aria-label={t('vol.up')} use:repeat={() => send({ t: 'media', a: 'volUp' })}>+</button>
</div>

<style>
  .vol {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px;
    border-radius: 999px;
    background: var(--panel);
  }
  .vol button {
    flex: none;
    width: 46px;
    height: 46px;
    border-radius: 50%;
    padding: 0;
    background: var(--panel-2);
    font-size: 1.5rem;
    line-height: 1;
  }
  .mute svg {
    width: 24px;
    height: 24px;
    fill: var(--text);
    stroke: var(--text);
    stroke-width: 2;
    stroke-linecap: round;
  }
  .mute.off {
    background: var(--warn);
  }
  .mute.off svg {
    fill: #111;
    stroke: #111;
  }
  .track {
    position: relative;
    flex: 1;
    height: 46px;
    border-radius: 999px;
    background: var(--panel-2);
    overflow: hidden;
    touch-action: none;
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    background: var(--accent);
    opacity: 0.85;
  }
  .num {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-weight: 700;
    font-size: 0.95rem;
    text-shadow: 0 1px 4px #0008;
  }
</style>
