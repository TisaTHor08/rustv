<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { ToastScreenProps } from '../../screens';

  // Message style Roku : bandeau violet en haut au centre, texte blanc gras, angles peu arrondis.
  let { message }: ToastScreenProps = $props();
</script>

<div class="toast" role="status" in:fly={{ y: -16, duration: 200 }} out:fade={{ duration: 200 }}>{message}</div>

<style>
  .toast { position: absolute; left: 50%; top: 2.6rem; transform: translateX(-50%); z-index: 40; max-width: 64rem; padding: 1.1rem 2.4rem; border-radius: 0.5rem; background: #662d91; color: #fff; font-family: var(--font); font-size: 1.4rem; font-weight: 600; text-align: center; box-shadow: 0 0.6rem 2rem rgba(0, 0, 0, 0.5); }
</style>
