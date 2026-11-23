<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { LaunchScreenProps } from '../../screens';

  // Lancement style VIDAA : fond bleu nuit, tuile carrée de l'appli à sa couleur (cadre blanc), anneau de chargement.
  let { name, color, color2, slow, cancelHint }: LaunchScreenProps = $props();
</script>

<div class="launch" role="status" aria-live="polite" style:--c1={color} style:--c2={color2} transition:fade={{ duration: 160 }}>
  <div class="tile"><b>{name[0]}</b></div>
  <h1>{name}</h1>
  <span class="ring"></span>
  <p class:slow>{cancelHint}</p>
</div>

<style>
  .launch { position: fixed; inset: 0; z-index: 200; display: grid; place-content: center; justify-items: center; gap: 1.6rem; background: #10121a; color: #fff; font-family: var(--font); cursor: wait; }
  .tile { display: grid; place-items: center; width: 14rem; height: 14rem; border-radius: 0.6rem; background: linear-gradient(135deg, var(--c1), var(--c2)); box-shadow: 0 0 0 0.22rem #fff, 0 1rem 3rem rgba(0, 0, 0, 0.5); animation: pop 0.3s cubic-bezier(0.2, 0.8, 0.2, 1) both; }
  b { font-size: 7rem; font-weight: 700; }
  h1 { margin: 0; font-size: 2.2rem; font-weight: 300; letter-spacing: 0.05em; }
  .ring { width: 3rem; height: 3rem; border: 0.28rem solid rgba(255, 255, 255, 0.2); border-top-color: #fff; border-radius: 50%; animation: s 0.9s linear infinite; }
  p { margin: 0; font-size: 1.1rem; opacity: 0.6; }
  @keyframes pop { from { transform: scale(0.85); opacity: 0; } }
  @keyframes s { to { transform: rotate(360deg); } }
</style>
