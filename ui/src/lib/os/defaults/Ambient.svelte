<script lang="ts">
  import { fade } from 'svelte/transition';
  import Weather from '../../Weather.svelte';
  import type { AmbientScreenProps } from '../screens';

  // Veille ambiante par défaut (RusTV) : dégradés lents tirés de la palette des tuiles, grande horloge, date, météo, téléphones.
  let { time, date, palette, weatherEnabled, status }: AmbientScreenProps = $props();
</script>

<div class="amb" transition:fade={{ duration: 700 }} style:--c1={palette[0]} style:--c2={palette[1]} style:--c3={palette[2]}>
  <i class="b1"></i><i class="b2"></i><i class="b3"></i>
  <div class="txt">
    <div class="clock">{time}</div>
    <div class="date">{date}</div>
    <div class="line">{#if weatherEnabled}<Weather />{/if}<span>{status}</span></div>
  </div>
</div>

<style>
  .amb { position: absolute; inset: 0; z-index: 60; overflow: hidden; background: #05060a; color: #fff; font-family: var(--font); cursor: none; }
  i { position: absolute; width: 90vmax; height: 90vmax; border-radius: 50%; filter: blur(9rem); opacity: 0.55; will-change: transform; }
  .b1 { left: -30vmax; top: -40vmax; background: var(--c1); animation: d1 38s ease-in-out infinite alternate; }
  .b2 { right: -35vmax; top: -10vmax; background: var(--c2); animation: d2 46s ease-in-out infinite alternate; }
  .b3 { left: 10vmax; bottom: -55vmax; background: var(--c3); animation: d3 54s ease-in-out infinite alternate; }
  .txt { position: absolute; inset: 0; display: grid; place-content: center; justify-items: center; text-align: center; background: radial-gradient(60% 60% at 50% 50%, rgba(0, 0, 0, 0.35), transparent 75%); }
  .clock { font-size: 15rem; font-weight: 200; line-height: 1; letter-spacing: -0.02em; text-shadow: 0 0.4rem 3rem rgba(0, 0, 0, 0.4); }
  .date { margin-top: 1rem; font-size: 2.4rem; font-weight: 400; opacity: 0.88; }
  .date::first-letter { text-transform: uppercase; }
  .line { margin-top: 2.2rem; display: flex; align-items: center; gap: 1.6rem; font-size: 1.4rem; opacity: 0.8; }
  @keyframes d1 { to { transform: translate(24vmax, 18vmax) scale(1.15); } }
  @keyframes d2 { to { transform: translate(-22vmax, 26vmax) scale(0.9); } }
  @keyframes d3 { to { transform: translate(18vmax, -22vmax) scale(1.2); } }
</style>
