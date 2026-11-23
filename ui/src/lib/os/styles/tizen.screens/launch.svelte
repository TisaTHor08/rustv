<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { LaunchScreenProps } from '../../screens';

  // Lancement style Tizen : l'icône arrondie de l'appli apparaît au centre sur fond bleu nuit, nom dessous et anneau de chargement fin.
  let { name, color, color2, slow, cancelHint }: LaunchScreenProps = $props();
</script>

<div class="launch" role="status" aria-live="polite" style:--c1={color} style:--c2={color2} transition:fade={{ duration: 180 }}>
  <div class="glow"></div>
  <div class="center">
    <span class="ic">{name[0]}</span>
    <b>{name}</b>
    <i class="ring"></i>
    <p class:slow>{cancelHint}</p>
  </div>
</div>

<style>
  .launch { position: fixed; inset: 0; z-index: 200; display: grid; place-items: center; background: linear-gradient(180deg, #0d1022, #05060d); color: #fff; font-family: var(--font); cursor: wait; overflow: hidden; }
  .glow { position: absolute; left: 50%; top: 42%; width: 70rem; height: 40rem; margin: -20rem 0 0 -35rem; background: radial-gradient(closest-side, color-mix(in srgb, var(--c1) 55%, transparent), transparent); animation: fadein 0.6s both; }
  .center { position: relative; display: grid; justify-items: center; gap: 1.6rem; }
  .ic { width: 13rem; height: 13rem; border-radius: 3.6rem; display: grid; place-items: center; font-size: 6rem; font-weight: 700; background: linear-gradient(135deg, var(--c1), var(--c2)); box-shadow: 0 1.6rem 4rem rgba(0, 0, 0, 0.55); animation: pop 0.5s cubic-bezier(0.2, 1.2, 0.3, 1) both; }
  b { font-size: 2.6rem; font-weight: 500; animation: fadein 0.4s 0.2s both; }
  .ring { width: 3rem; height: 3rem; border-radius: 50%; border: 0.28rem solid rgba(255, 255, 255, 0.25); border-top-color: #fff; animation: spin 0.9s linear infinite; }
  p { margin: 0; font-size: 1.2rem; opacity: 0.65; }
  @keyframes pop { from { transform: scale(0.6); opacity: 0; } }
  @keyframes fadein { from { opacity: 0; } }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
