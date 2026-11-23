<script lang="ts">
  import { fade } from 'svelte/transition';
  import { registerStrings, t } from './i18n';

  registerStrings('fr', { 'launch.opening': 'Ouverture de {name}', 'launch.cancel': 'Retour pour annuler', 'launch.slow': 'Le site met du temps à répondre…' });
  registerStrings('en', { 'launch.opening': 'Opening {name}', 'launch.cancel': 'Press Back to cancel', 'launch.slow': 'The site is taking a while to answer…' });

  let { name, color, color2, slow }: { name: string; color: string; color2: string; slow: boolean } = $props();
</script>

<!-- Animation de lancement : couvre l'accueil et capte les clics (les touches sont gérées par l'accueil). -->
<div class="launch" role="status" aria-live="polite" style:--c1={color} style:--c2={color2} transition:fade={{ duration: 160 }}>
  <div class="glow"></div>
  <div class="center">
    <div class="ring"><i></i><i></i><i></i></div>
    <h2>{t('launch.opening', { name })}</h2>
    <p>{slow ? t('launch.slow') : t('launch.cancel')}</p>
  </div>
</div>

<style>
  .launch {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, var(--c1), var(--c2));
    color: #fff;
    cursor: wait;
    animation: rise 0.28s ease-out both;
  }
  .glow {
    position: absolute;
    inset: 0;
    background: radial-gradient(60% 60% at 50% 50%, rgba(255, 255, 255, 0.18), transparent 70%), rgba(0, 0, 0, 0.35);
    animation: breathe 2.4s ease-in-out infinite;
  }
  .center {
    position: relative;
    display: grid;
    justify-items: center;
    gap: 1.2rem;
    text-align: center;
  }
  h2 {
    margin: 0;
    font-size: 2.6rem;
    font-weight: 800;
    letter-spacing: -0.01em;
    text-shadow: 0 0.2rem 1.4rem rgba(0, 0, 0, 0.35);
  }
  p {
    margin: 0;
    font-size: 1.3rem;
    opacity: 0.8;
  }
  .ring {
    position: relative;
    width: 7rem;
    height: 7rem;
  }
  .ring i {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 0.45rem solid transparent;
    border-top-color: #fff;
    animation: spin 1.1s linear infinite;
  }
  .ring i:nth-child(2) {
    inset: 0.9rem;
    border-top-color: rgba(255, 255, 255, 0.65);
    animation-duration: 1.7s;
    animation-direction: reverse;
  }
  .ring i:nth-child(3) {
    inset: 1.8rem;
    border-top-color: rgba(255, 255, 255, 0.4);
    animation-duration: 2.3s;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes breathe {
    50% {
      opacity: 0.7;
    }
  }
  @keyframes rise {
    from {
      transform: scale(1.04);
    }
  }
</style>
