<script lang="ts">
  import { fade } from 'svelte/transition';
  import Weather from '../../../Weather.svelte';
  import type { AmbientScreenProps } from '../../screens';

  // Économiseur style webOS : fond noir avec de grandes taches de couleur douces qui dérivent lentement (palette des tuiles),
  // horloge fine centrée en bas, date et météo dessous.
  let { time, date, palette, weatherEnabled, status }: AmbientScreenProps = $props();
</script>

<div class="amb" transition:fade={{ duration: 900 }} style:--c1={palette[0]} style:--c2={palette[1]} style:--c3={palette[2]} style:--c4={palette[3]}>
  <i class="o o1"></i><i class="o o2"></i><i class="o o3"></i><i class="o o4"></i>
  <div class="info">
    <div class="clock">{time}</div>
    <div class="line"><span>{date}</span>{#if weatherEnabled}<Weather />{/if}</div>
    <div class="st">{status}</div>
  </div>
</div>

<style>
  .amb { position: absolute; inset: 0; z-index: 60; overflow: hidden; background: #020305; color: #fff; font-family: var(--font); cursor: none; }
  .o { position: absolute; width: 60rem; height: 60rem; border-radius: 50%; filter: blur(7rem); opacity: 0.55; }
  .o1 { left: -10rem; top: -14rem; background: var(--c1); animation: d1 40s ease-in-out infinite alternate; }
  .o2 { right: -12rem; top: 4rem; background: var(--c2); animation: d2 52s ease-in-out infinite alternate; }
  .o3 { left: 22rem; bottom: -26rem; background: var(--c3); animation: d1 46s ease-in-out infinite alternate-reverse; }
  .o4 { right: 14rem; bottom: -30rem; background: var(--c4); animation: d2 58s ease-in-out infinite alternate-reverse; }
  .info { position: absolute; left: 0; right: 0; bottom: 5rem; text-align: center; }
  .clock { font-size: 9rem; font-weight: 200; line-height: 1; letter-spacing: -0.02em; text-shadow: 0 0.2rem 1.6rem rgba(0, 0, 0, 0.5); }
  .line { display: flex; justify-content: center; align-items: center; gap: 1.6rem; margin-top: 0.8rem; font-size: 2rem; opacity: 0.9; }
  .line span::first-letter { text-transform: uppercase; }
  .st { margin-top: 0.7rem; font-size: 1.2rem; opacity: 0.55; }
  @keyframes d1 { to { transform: translate(14rem, 10rem) scale(1.15); } }
  @keyframes d2 { to { transform: translate(-16rem, 8rem) scale(0.9); } }
</style>
