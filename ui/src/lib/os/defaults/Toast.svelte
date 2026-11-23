<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { ToastScreenProps } from '../screens';

  // Toast par défaut (RusTV) : pilule claire en bas au centre.
  let { message }: ToastScreenProps = $props();
</script>

<div class="toast" role="status" transition:fade={{ duration: 200 }}>{message}</div>

<style>
  .toast { position: absolute; left: 50%; bottom: 3rem; transform: translateX(-50%); padding: 0.9rem 1.8rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.95); color: #111; font-size: 1.2rem; font-weight: 600; z-index: 40; }
</style>
