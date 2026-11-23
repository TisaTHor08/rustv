<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { LaunchScreenProps } from '../../screens';

  // Lancement style Titan : fond indigo, tuile carrée arrondie de l'appli à sa couleur, trois points qui pulsent.
  let { name, color, color2, slow, cancelHint }: LaunchScreenProps = $props();
</script>

<div class="launch" role="status" aria-live="polite" style:--c1={color} style:--c2={color2} transition:fade={{ duration: 160 }}>
  <div class="tile"><b>{name[0]}</b></div>
  <h1>{name}</h1>
  <div class="dots"><i></i><i></i><i></i></div>
  <p class:slow>{cancelHint}</p>
</div>

<style>
  .launch { position: fixed; inset: 0; z-index: 200; display: grid; place-content: center; justify-items: center; gap: 1.6rem; background: radial-gradient(50rem 36rem at 50% 40%, color-mix(in srgb, var(--c1) 35%, #0d0b1d), #070713 75%); color: #fff; font-family: var(--font); cursor: wait; }
  .tile { display: grid; place-items: center; width: 14rem; height: 14rem; border-radius: 1.4rem; background: linear-gradient(135deg, var(--c1), var(--c2)); box-shadow: 0 0 0 0.2rem rgba(223, 231, 255, 0.9), 0 0 4rem color-mix(in srgb, var(--c1) 60%, transparent); animation: pop 0.35s cubic-bezier(0.2, 0.8, 0.2, 1) both; }
  b { font-size: 7rem; font-weight: 700; }
  h1 { margin: 0; font-size: 2.2rem; font-weight: 500; }
  .dots { display: flex; gap: 0.7rem; }
  .dots i { width: 0.8rem; height: 0.8rem; border-radius: 50%; background: #fff; animation: p 1s ease-in-out infinite; }
  .dots i:nth-child(2) { animation-delay: 0.15s; }
  .dots i:nth-child(3) { animation-delay: 0.3s; }
  p { margin: 0; font-size: 1.1rem; opacity: 0.6; }
  @keyframes pop { from { transform: scale(0.85); opacity: 0; } }
  @keyframes p { 0%, 100% { opacity: 0.25; transform: scale(0.8); } 50% { opacity: 1; transform: scale(1.1); } }
</style>
