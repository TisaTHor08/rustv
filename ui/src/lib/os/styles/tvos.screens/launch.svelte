<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { LaunchScreenProps } from '../../screens';

  // Lancement style tvOS : l'icône de l'appli (sa couleur) grossit depuis le centre jusqu'à remplir l'écran, le nom reste net au milieu.
  let { name, color, color2, slow, cancelHint }: LaunchScreenProps = $props();
</script>

<div class="launch" role="status" aria-live="polite" style:--c1={color} style:--c2={color2} transition:fade={{ duration: 220 }}>
  <div class="grow"></div>
  <b>{name}</b>
  <p class:slow>{cancelHint}</p>
</div>

<style>
  .launch { position: fixed; inset: 0; z-index: 200; display: grid; place-content: center; justify-items: center; gap: 1.6rem; background: #000; color: #fff; font-family: var(--font); cursor: wait; overflow: hidden; }
  .grow { position: absolute; left: 50%; top: 50%; width: 30rem; height: 18.75rem; margin: -9.375rem 0 0 -15rem; border-radius: var(--r); background: linear-gradient(145deg, var(--c1), var(--c2)); animation: grow 0.7s cubic-bezier(0.2, 0.8, 0.2, 1) both; }
  b { position: relative; font-size: 3.4rem; font-weight: 600; letter-spacing: -0.01em; text-shadow: 0 0.2rem 1.4rem rgba(0, 0, 0, 0.35); animation: rise 0.5s 0.15s ease-out both; }
  p { position: relative; margin: 0; font-size: 1.2rem; color: rgba(255, 255, 255, 0.7); }
  @keyframes grow { to { transform: scale(7); border-radius: 0; } }
  @keyframes rise { from { opacity: 0; transform: translateY(0.8rem); } }
</style>
