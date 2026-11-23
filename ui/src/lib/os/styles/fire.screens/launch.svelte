<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { LaunchScreenProps } from '../../screens';

  // Lancement style Fire TV : fond presque noir, carte de l'appli à sa couleur, anneau de chargement blanc en dessous.
  let { name, color, color2, slow, cancelHint }: LaunchScreenProps = $props();
</script>

<div class="launch" role="status" aria-live="polite" style:--c1={color} style:--c2={color2} transition:fade={{ duration: 160 }}>
  <div class="card"><b>{name}</b></div>
  <span class="ring"></span>
  <p class:slow>{cancelHint}</p>
</div>

<style>
  .launch { position: fixed; inset: 0; z-index: 200; display: grid; place-content: center; justify-items: center; gap: 2.6rem; background: #0d0d0d; color: #fff; font-family: var(--font); cursor: wait; }
  .card { display: grid; place-items: center; width: 33.6rem; height: 18.9rem; border-radius: var(--r); background: linear-gradient(135deg, var(--c1), var(--c2)); box-shadow: 0 1rem 3rem rgba(0, 0, 0, 0.6); animation: pop 0.3s var(--ease) both; }
  b { font-size: 3rem; font-weight: 500; text-align: center; padding: 0 2rem; }
  .ring { width: 3.6rem; height: 3.6rem; border: 0.3rem solid rgba(255, 255, 255, 0.25); border-top-color: #fff; border-radius: 50%; animation: spin 0.9s linear infinite; }
  p { margin: 0; font-size: 1.2rem; color: rgba(255, 255, 255, 0.6); }
  @keyframes pop { from { transform: scale(0.92); opacity: 0; } }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
