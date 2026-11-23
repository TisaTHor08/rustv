<script lang="ts">
  import { fade } from 'svelte/transition';
  import Weather from '../../../Weather.svelte';
  import type { AmbientScreenProps } from '../../screens';

  // Mode ambiant style Google TV : image plein cadre (paysage abstrait généré depuis la palette, lent zoom), heure et météo en bas à gauche.
  let { time, date, palette, weatherEnabled, status }: AmbientScreenProps = $props();
</script>

<div class="amb" transition:fade={{ duration: 900 }} style:--c1={palette[0]} style:--c2={palette[1]} style:--c3={palette[2]} style:--c4={palette[3]}>
  <div class="art">
    <i class="sky"></i><i class="sun"></i><i class="h1"></i><i class="h2"></i><i class="h3"></i>
  </div>
  <div class="shade"></div>
  <div class="info">
    <div class="clock">{time}</div>
    <div class="line"><span>{date}</span>{#if weatherEnabled}<Weather />{/if}</div>
    <div class="st">{status}</div>
  </div>
</div>

<style>
  .amb { position: absolute; inset: 0; z-index: 60; overflow: hidden; background: #000; color: #fff; font-family: var(--font); cursor: none; }
  .art { position: absolute; inset: -4%; animation: kb 60s ease-in-out infinite alternate; }
  .sky { position: absolute; inset: 0; background: linear-gradient(180deg, color-mix(in srgb, var(--c2) 55%, #0b0d2a) 0%, color-mix(in srgb, var(--c1) 70%, #1a0f2e) 55%, color-mix(in srgb, var(--c3) 60%, #20102a) 100%); }
  .sun { position: absolute; left: 62%; top: 34%; width: 16vmax; height: 16vmax; border-radius: 50%; background: radial-gradient(circle, rgba(255, 244, 214, 0.95) 0%, rgba(255, 200, 130, 0.55) 35%, transparent 70%); filter: blur(0.6rem); }
  .h1, .h2, .h3 { position: absolute; left: -10%; right: -10%; bottom: 0; border-radius: 50% 50% 0 0 / 100% 100% 0 0; }
  .h1 { height: 42%; background: color-mix(in srgb, var(--c4) 55%, #1a1230); opacity: 0.85; transform: translateX(-6%) scaleX(1.2); }
  .h2 { height: 30%; background: color-mix(in srgb, var(--c3) 45%, #120b22); opacity: 0.95; transform: translateX(10%) scaleX(1.3); }
  .h3 { height: 17%; background: #0a0714; }
  .shade { position: absolute; inset: 0; background: linear-gradient(0deg, rgba(0, 0, 0, 0.75) 0%, rgba(0, 0, 0, 0.1) 40%, transparent 60%); }
  .info { position: absolute; left: 5.4rem; bottom: 4.4rem; }
  .clock { font-size: 8.4rem; font-weight: 300; line-height: 1; letter-spacing: -0.02em; text-shadow: 0 0.2rem 1.6rem rgba(0, 0, 0, 0.5); }
  .line { display: flex; align-items: center; gap: 1.6rem; margin-top: 0.8rem; font-size: 2rem; opacity: 0.92; }
  .line span::first-letter { text-transform: uppercase; }
  .st { margin-top: 0.7rem; font-size: 1.2rem; opacity: 0.6; }
  @keyframes kb { to { transform: scale(1.08) translate(-1.5%, -1%); } }
</style>
