<script lang="ts">
  import { fade } from 'svelte/transition';
  import Weather from '../../../Weather.svelte';
  import type { AmbientScreenProps } from '../../screens';

  // Économiseur style Roku : fond sombre, carte (heure, date, météo) qui rebondit lentement dans l'écran.
  let { time, date, palette, weatherEnabled, status }: AmbientScreenProps = $props();
</script>

<div class="amb" transition:fade={{ duration: 700 }} style:--c1={palette[0]} style:--c2={palette[1]}>
  <div class="bx"><div class="by">
    <div class="card"><b>{time}</b><span>{date}</span>{#if weatherEnabled}<Weather />{/if}<small>{status}</small></div>
  </div></div>
</div>

<style>
  .amb { position: absolute; inset: 0; z-index: 60; overflow: hidden; background: #0b0412; color: #fff; font-family: var(--font); cursor: none; }
  .bx { position: absolute; inset: 0; animation: bx 41s linear infinite alternate; }
  .by { position: absolute; inset: 0; animation: by 27s linear infinite alternate; }
  .card { position: absolute; left: 0; top: 0; display: grid; gap: 0.5rem; justify-items: center; width: 34rem; padding: 2.2rem 2rem; border-radius: 0.8rem; background: linear-gradient(135deg, color-mix(in srgb, var(--c1) 60%, #3b1259), color-mix(in srgb, var(--c2) 50%, #1b0a2c)); box-shadow: 0 1rem 3rem rgba(0, 0, 0, 0.6); }
  b { font-size: 6rem; font-weight: 700; line-height: 1; }
  span { font-size: 1.8rem; font-weight: 500; }
  span::first-letter { text-transform: uppercase; }
  small { font-size: 1.1rem; opacity: 0.7; }
  @keyframes bx { from { transform: translateX(0); } to { transform: translateX(calc(100vw - 34rem)); } }
  @keyframes by { from { transform: translateY(0); } to { transform: translateY(calc(100vh - 22rem)); } }
</style>
