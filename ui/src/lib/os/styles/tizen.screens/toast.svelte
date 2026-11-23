<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { ToastScreenProps } from '../../screens';

  // Message style Tizen : pastille sombre arrondie en bas au centre, bordure claire fine (comme les infobulles One UI).
  let { message }: ToastScreenProps = $props();
</script>

<div class="toast" role="status" in:fly={{ y: 16, duration: 200 }} out:fade={{ duration: 200 }}>{message}</div>

<style>
  .toast { position: absolute; left: 50%; bottom: 3.4rem; transform: translateX(-50%); z-index: 40; max-width: 64rem; padding: 1.1rem 2.6rem; border-radius: 99rem; background: rgba(22, 23, 44, 0.94); color: #f4f5fa; font-family: var(--font); font-size: 1.35rem; text-align: center; box-shadow: inset 0 0 0 0.1rem rgba(255, 255, 255, 0.3), 0 0.8rem 2.4rem rgba(0, 0, 0, 0.5); }
</style>
