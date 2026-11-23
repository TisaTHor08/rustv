<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { LaunchScreenProps } from '../../screens';

  // Lancement style Roku : fond violet, carte de la chaîne à sa couleur, barre de chargement qui défile.
  let { name, color, color2, slow, cancelHint }: LaunchScreenProps = $props();
</script>

<div class="launch" role="status" aria-live="polite" style:--c1={color} style:--c2={color2} transition:fade={{ duration: 160 }}>
  <div class="card"><b>{name}</b></div>
  <div class="bar"><i></i></div>
  <p class:slow>{cancelHint}</p>
</div>

<style>
  .launch { position: fixed; inset: 0; z-index: 200; display: grid; place-content: center; justify-items: center; gap: 2.2rem; background: linear-gradient(160deg, #3b1259 0%, #1b0a2c 70%); color: #fff; font-family: var(--font); cursor: wait; }
  .card { display: grid; place-items: center; width: 36rem; height: 20.25rem; border-radius: var(--r); background: linear-gradient(135deg, var(--c1), var(--c2)); box-shadow: 0 1rem 3rem rgba(0, 0, 0, 0.5); animation: pop 0.35s cubic-bezier(0.2, 0.8, 0.2, 1) both; }
  b { font-size: 3rem; font-weight: 700; text-align: center; padding: 0 2rem; }
  .bar { width: 36rem; height: 0.5rem; border-radius: 0.2rem; background: rgba(255, 255, 255, 0.2); overflow: hidden; position: relative; }
  .bar i { position: absolute; top: 0; bottom: 0; width: 30%; background: #fff; animation: run 1.1s ease-in-out infinite; }
  p { margin: 0; font-size: 1.2rem; opacity: 0.7; }
  @keyframes pop { from { transform: scale(0.85); opacity: 0; } }
  @keyframes run { from { left: -30%; } to { left: 100%; } }
</style>
