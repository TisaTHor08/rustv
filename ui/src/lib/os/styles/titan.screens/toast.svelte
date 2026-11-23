<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { ToastScreenProps } from '../../screens';

  // Message style Titan : cartouche bleu nuit arrondi en haut au centre, fin liseré bleu à gauche.
  let { message }: ToastScreenProps = $props();
</script>

<div class="toast" role="status" in:fly={{ y: -14, duration: 200 }} out:fade={{ duration: 200 }}>{message}</div>

<style>
  .toast { position: absolute; left: 50%; top: 6.4rem; transform: translateX(-50%); z-index: 40; max-width: 64rem; padding: 1.1rem 2.4rem; border-radius: 0.8rem; border-left: 0.35rem solid #1d7be5; background: rgba(16, 24, 46, 0.97); color: #fff; font-family: var(--font); font-size: 1.4rem; font-weight: 500; text-align: center; box-shadow: 0 0.6rem 2rem rgba(0, 0, 0, 0.5); }
</style>
