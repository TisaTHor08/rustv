<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { ToastScreenProps } from '../../screens';

  // Message style VIDAA : cartouche sombre, bord fin clair, en bas au centre.
  let { message }: ToastScreenProps = $props();
</script>

<div class="toast" role="status" in:fly={{ y: 14, duration: 200 }} out:fade={{ duration: 200 }}>{message}</div>

<style>
  .toast { position: absolute; left: 50%; bottom: 4rem; transform: translateX(-50%); z-index: 40; max-width: 64rem; padding: 1.1rem 2.6rem; border-radius: 0.4rem; background: rgba(27, 30, 43, 0.97); border: 0.1rem solid rgba(255, 255, 255, 0.25); color: #fff; font-family: var(--font); font-size: 1.4rem; text-align: center; box-shadow: 0 0.6rem 2rem rgba(0, 0, 0, 0.5); }
</style>
