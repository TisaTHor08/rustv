<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { LaunchScreenProps } from '../../screens';

  // Lancement style webOS : écran noir, icône carrée arrondie au centre (couleur de la tuile), nom et trois points de chargement.
  let { name, color, color2, slow, cancelHint }: LaunchScreenProps = $props();
</script>

<div class="launch" role="status" aria-live="polite" style:--c1={color} style:--c2={color2} transition:fade={{ duration: 160 }}>
  <div class="center">
    <span class="ic">{name[0]}</span>
    <b>{name}</b>
    <div class="dots"><i></i><i></i><i></i></div>
    <p class:slow>{cancelHint}</p>
  </div>
</div>

<style>
  .launch { position: fixed; inset: 0; z-index: 200; display: grid; place-items: center; background: #030406; color: #fff; font-family: var(--font); cursor: wait; }
  .center { display: grid; justify-items: center; gap: 1.6rem; }
  .ic { width: 12rem; height: 12rem; border-radius: 1.8rem; display: grid; place-items: center; font-size: 5.6rem; font-weight: 700; background: linear-gradient(135deg, var(--c1), var(--c2)); animation: pop 0.35s ease-out both; }
  b { font-size: 2.2rem; font-weight: 500; }
  .dots { display: flex; gap: 0.8rem; }
  .dots i { width: 0.9rem; height: 0.9rem; border-radius: 50%; background: #fff; animation: pulse 1.2s ease-in-out infinite; }
  .dots i:nth-child(2) { animation-delay: 0.2s; }
  .dots i:nth-child(3) { animation-delay: 0.4s; }
  p { margin: 0; font-size: 1.2rem; opacity: 0.6; }
  @keyframes pop { from { transform: scale(0.8); opacity: 0; } }
  @keyframes pulse { 0%, 80%, 100% { opacity: 0.25; transform: scale(0.8); } 40% { opacity: 1; transform: scale(1.1); } }
</style>
