<script lang="ts">
  import { fade } from 'svelte/transition';
  import Weather from '../../../Weather.svelte';
  import type { AmbientScreenProps } from '../../screens';

  // Économiseur style tvOS (Aerial) : paysage plein écran (ciel et relief en dégradés de la palette) qui avance très lentement,
  // heure et date en bas à gauche, en blanc fin avec une ombre douce.
  let { time, date, palette, weatherEnabled, status }: AmbientScreenProps = $props();
</script>

<div class="amb" transition:fade={{ duration: 1200 }} style:--c1={palette[0]} style:--c2={palette[1]} style:--c3={palette[2] ?? palette[0]}>
  <div class="sky"></div>
  <div class="land"></div>
  <div class="info"><b>{time}</b><span>{date}</span>{#if weatherEnabled}<Weather />{/if}<small>{status}</small></div>
</div>

<style>
  .amb { position: absolute; inset: 0; z-index: 60; overflow: hidden; background: #000; color: #fff; font-family: var(--font); cursor: none; }
  .sky { position: absolute; inset: -8%; background: linear-gradient(180deg, color-mix(in srgb, var(--c1) 55%, #0a1020) 0%, color-mix(in srgb, var(--c2) 70%, #d9a066) 62%, color-mix(in srgb, var(--c3) 50%, #f2c27e) 100%); animation: pan 46s ease-in-out infinite alternate; }
  .land { position: absolute; left: -6%; right: -6%; bottom: 0; height: 38%; background: radial-gradient(120% 90% at 30% 100%, #07090d 55%, transparent 56%), radial-gradient(120% 80% at 80% 100%, #0b0e14 52%, transparent 53%); animation: pan2 46s ease-in-out infinite alternate; }
  .info { position: absolute; left: 6rem; bottom: 4.4rem; display: grid; gap: 0.2rem; text-shadow: 0 0.2rem 1.4rem rgba(0, 0, 0, 0.5); }
  b { font-size: 5.2rem; font-weight: 300; line-height: 1; letter-spacing: -0.01em; }
  span { font-size: 1.6rem; font-weight: 500; color: rgba(255, 255, 255, 0.9); }
  span::first-letter { text-transform: uppercase; }
  small { font-size: 1.1rem; color: rgba(255, 255, 255, 0.7); }
  @keyframes pan { from { transform: scale(1.02) translateX(-1.5%); } to { transform: scale(1.14) translateX(1.5%); } }
  @keyframes pan2 { from { transform: translateX(-2%); } to { transform: translateX(2%); } }
</style>
