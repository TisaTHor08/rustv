<script lang="ts">
  import { fade } from 'svelte/transition';
  import Weather from '../../../Weather.svelte';
  import type { AmbientScreenProps } from '../../screens';

  // Économiseur style VIDAA (approximation) : fond bleu nuit, grande heure fine, date et météo, dérive lente pour l'OLED.
  let { time, date, palette, weatherEnabled, status }: AmbientScreenProps = $props();
</script>

<div class="amb" transition:fade={{ duration: 700 }} style:--c1={palette[0]} style:--c2={palette[1]}>
  <div class="glow"></div>
  <div class="drift">
    <b>{time}</b><span>{date}</span>{#if weatherEnabled}<Weather />{/if}<small>{status}</small>
  </div>
</div>

<style>
  .amb { position: absolute; inset: 0; z-index: 60; overflow: hidden; background: #0a0b11; color: #fff; font-family: var(--font); cursor: none; }
  .glow { position: absolute; inset: -20%; background: radial-gradient(40rem 28rem at 30% 70%, color-mix(in srgb, var(--c1) 35%, transparent), transparent 70%), radial-gradient(40rem 28rem at 75% 30%, color-mix(in srgb, var(--c2) 30%, transparent), transparent 70%); animation: g 60s ease-in-out infinite alternate; }
  .drift { position: absolute; left: 50%; top: 50%; display: grid; gap: 0.6rem; justify-items: center; transform: translate(-50%, -50%); animation: d 90s ease-in-out infinite alternate; }
  b { font-size: 11rem; font-weight: 300; line-height: 1; letter-spacing: 0.04em; }
  span { font-size: 2rem; font-weight: 300; opacity: 0.85; }
  span::first-letter { text-transform: uppercase; }
  small { font-size: 1.2rem; opacity: 0.55; }
  @keyframes g { to { transform: translate(4rem, -3rem); } }
  @keyframes d { from { transform: translate(-60%, -56%); } to { transform: translate(-40%, -44%); } }
</style>
