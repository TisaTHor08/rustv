<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { LaunchScreenProps } from '../../screens';

  // Lancement style Google TV : la couleur de la tuile s'étend depuis le centre jusqu'au plein écran, puis le nom et trois points qui pulsent.
  let { name, color, color2, slow, cancelHint }: LaunchScreenProps = $props();
</script>

<div class="launch" role="status" aria-live="polite" style:--c1={color} style:--c2={color2} transition:fade={{ duration: 180 }}>
  <div class="grow"></div>
  <div class="center">
    <b>{name}</b>
    <div class="dots"><i></i><i></i><i></i></div>
    <p class:slow>{cancelHint}</p>
  </div>
</div>

<style>
  .launch { position: fixed; inset: 0; z-index: 200; display: grid; place-items: center; background: #0e0f12; color: #fff; font-family: var(--font); cursor: wait; overflow: hidden; }
  .grow { position: absolute; left: 50%; top: 50%; width: 25.6rem; height: 14.4rem; margin: -7.2rem 0 0 -12.8rem; border-radius: var(--r); background: linear-gradient(135deg, var(--c1), var(--c2)); animation: grow 0.5s cubic-bezier(0.2, 0.8, 0.2, 1) both; }
  .center { position: relative; display: grid; justify-items: center; gap: 1.6rem; animation: fadein 0.4s 0.25s both; }
  b { font-size: 3.4rem; font-weight: 400; letter-spacing: -0.01em; text-shadow: 0 0.2rem 1.2rem rgba(0, 0, 0, 0.4); }
  .dots { display: flex; gap: 0.9rem; }
  .dots i { width: 1rem; height: 1rem; border-radius: 50%; background: #fff; animation: pulse 1.2s ease-in-out infinite; }
  .dots i:nth-child(2) { animation-delay: 0.2s; }
  .dots i:nth-child(3) { animation-delay: 0.4s; }
  p { margin: 0; font-size: 1.2rem; opacity: 0.7; }
  @keyframes grow { to { left: 0; top: 0; width: 100%; height: 100%; margin: 0; border-radius: 0; } }
  @keyframes fadein { from { opacity: 0; } }
  @keyframes pulse { 0%, 80%, 100% { opacity: 0.25; transform: scale(0.8); } 40% { opacity: 1; transform: scale(1.1); } }
</style>
