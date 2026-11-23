<script lang="ts">
  import { fade } from 'svelte/transition';
  import Weather from '../../../Weather.svelte';
  import type { AmbientScreenProps } from '../../screens';

  // Mode Ambiance style Tizen : montagnes en couches rose-violet-bleu (comme les fonds « Ambient » de Samsung), horloge en grand
  // en haut à gauche, date et météo dessous, très lente dérive.
  let { time, date, palette, weatherEnabled, status }: AmbientScreenProps = $props();
</script>

<div class="amb" transition:fade={{ duration: 900 }} style:--c1={palette[0]} style:--c2={palette[1]}>
  <div class="art">
    <i class="sky"></i><i class="m1"></i><i class="m2"></i><i class="m3"></i><i class="m4"></i>
  </div>
  <div class="shade"></div>
  <div class="info">
    <div class="clock">{time}</div>
    <div class="line"><span>{date}</span>{#if weatherEnabled}<Weather />{/if}</div>
    <div class="st">{status}</div>
  </div>
</div>

<style>
  .amb { position: absolute; inset: 0; z-index: 60; overflow: hidden; background: #0a0b17; color: #fff; font-family: var(--font); cursor: none; }
  .art { position: absolute; inset: -3%; animation: drift 70s ease-in-out infinite alternate; }
  .sky { position: absolute; inset: 0; background: linear-gradient(180deg, #f6b2c4 0%, #c06fc9 28%, #6a4bc0 55%, #241b57 100%); }
  .m1, .m2, .m3, .m4 { position: absolute; left: -10%; right: -10%; bottom: 0; clip-path: polygon(0 100%, 0 60%, 14% 38%, 26% 56%, 40% 24%, 56% 52%, 70% 30%, 84% 50%, 100% 34%, 100% 100%); }
  .m1 { height: 62%; background: linear-gradient(180deg, color-mix(in srgb, var(--c1, #5f6fe0) 30%, #b04fb8), #5a3fb5); opacity: 0.8; transform: translateX(6%); }
  .m2 { height: 50%; background: linear-gradient(180deg, color-mix(in srgb, var(--c2, #2f8fe8) 25%, #7a45c0), #3a2c88); opacity: 0.9; transform: translateX(-8%) scaleX(1.1); }
  .m3 { height: 36%; background: linear-gradient(180deg, #4a35a0, #1c1a52); transform: translateX(12%); }
  .m4 { height: 22%; background: linear-gradient(180deg, #1f1b55, #0c0d26); transform: translateX(-4%) scaleX(1.2); }
  .shade { position: absolute; inset: 0; background: linear-gradient(180deg, rgba(0, 0, 0, 0.45) 0%, transparent 40%, rgba(0, 0, 0, 0.35) 100%); }
  .info { position: absolute; left: 7rem; top: 5rem; }
  .clock { font-size: 9rem; font-weight: 300; line-height: 1; letter-spacing: -0.02em; text-shadow: 0 0.2rem 1.6rem rgba(0, 0, 0, 0.35); }
  .line { display: flex; align-items: center; gap: 1.6rem; margin-top: 0.8rem; font-size: 2rem; opacity: 0.92; }
  .line span::first-letter { text-transform: uppercase; }
  .st { margin-top: 0.7rem; font-size: 1.2rem; opacity: 0.65; }
  @keyframes drift { to { transform: scale(1.06) translate(-1.2%, -0.8%); } }
</style>
