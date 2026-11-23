<script lang="ts">
  import { fade } from 'svelte/transition';
  import Weather from '../../../Weather.svelte';
  import type { AmbientScreenProps } from '../../screens';

  // Économiseur style Titan (approximation) : écran noir bordé d'un halo coloré qui change lentement (clin d'oeil à Ambilight),
  // heure et date au centre, dérive lente.
  let { time, date, palette, weatherEnabled, status }: AmbientScreenProps = $props();
</script>

<div class="amb" transition:fade={{ duration: 700 }} style:--c1={palette[0]} style:--c2={palette[1]} style:--c3={palette[2] ?? palette[0]}>
  <div class="glow"></div>
  <div class="drift">
    <b>{time}</b><span>{date}</span>{#if weatherEnabled}<Weather />{/if}<small>{status}</small>
  </div>
</div>

<style>
  .amb { position: absolute; inset: 0; z-index: 60; overflow: hidden; background: #020208; color: #fff; font-family: var(--font); cursor: none; }
  .glow { position: absolute; inset: 0; box-shadow: inset 0 0 22rem 4rem var(--c1); animation: hue 24s ease-in-out infinite alternate; opacity: 0.75; }
  .drift { position: absolute; left: 50%; top: 50%; display: grid; gap: 0.5rem; justify-items: center; transform: translate(-50%, -50%); animation: d 80s ease-in-out infinite alternate; }
  b { font-size: 10rem; font-weight: 600; line-height: 1; }
  span { font-size: 2rem; font-weight: 400; opacity: 0.85; }
  span::first-letter { text-transform: uppercase; }
  small { font-size: 1.2rem; opacity: 0.55; }
  @keyframes hue { 0% { box-shadow: inset 0 0 22rem 4rem var(--c1); } 50% { box-shadow: inset 0 0 22rem 4rem var(--c2); } 100% { box-shadow: inset 0 0 22rem 4rem var(--c3); } }
  @keyframes d { from { transform: translate(-58%, -56%); } to { transform: translate(-42%, -44%); } }
</style>
