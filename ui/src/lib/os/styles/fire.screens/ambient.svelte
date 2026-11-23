<script lang="ts">
  import { fade } from 'svelte/transition';
  import Weather from '../../../Weather.svelte';
  import type { AmbientScreenProps } from '../../screens';

  // Économiseur style Fire TV : « photo » plein écran (dégradés de la palette qui dérivent lentement, zoom très lent), heure et date en bas à gauche.
  let { time, date, palette, weatherEnabled, status }: AmbientScreenProps = $props();
</script>

<div class="amb" transition:fade={{ duration: 900 }} style:--c1={palette[0]} style:--c2={palette[1]} style:--c3={palette[2] ?? palette[0]}>
  <div class="photo"></div>
  <div class="shade"></div>
  <div class="info"><b>{time}</b><span>{date}</span>{#if weatherEnabled}<Weather />{/if}<small>{status}</small></div>
</div>

<style>
  .amb { position: absolute; inset: 0; z-index: 60; overflow: hidden; background: #000; color: #fff; font-family: var(--font); cursor: none; }
  .photo { position: absolute; inset: -6%; background: radial-gradient(70rem 40rem at 25% 30%, color-mix(in srgb, var(--c1) 80%, #000), transparent 70%), radial-gradient(60rem 36rem at 80% 70%, color-mix(in srgb, var(--c2) 75%, #000), transparent 70%), radial-gradient(50rem 30rem at 60% 15%, color-mix(in srgb, var(--c3) 60%, #000), transparent 72%), #0b0b0b; animation: drift 38s ease-in-out infinite alternate; }
  .shade { position: absolute; inset: 0; background: linear-gradient(0deg, rgba(0, 0, 0, 0.65), transparent 45%); }
  .info { position: absolute; left: 5.2rem; bottom: 4rem; display: grid; gap: 0.3rem; }
  b { font-size: 6.4rem; font-weight: 300; line-height: 1; }
  span { font-size: 1.7rem; font-weight: 400; color: rgba(255, 255, 255, 0.9); }
  span::first-letter { text-transform: uppercase; }
  small { font-size: 1.1rem; color: rgba(255, 255, 255, 0.65); }
  @keyframes drift { from { transform: scale(1) translate(0, 0); } to { transform: scale(1.12) translate(-2%, 1.5%); } }
</style>
