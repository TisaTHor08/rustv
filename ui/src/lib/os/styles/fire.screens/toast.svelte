<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { ToastScreenProps } from '../../screens';

  // Notification style Fire TV : carte sombre en haut à droite, entre par la droite, filet bleu à gauche.
  let { message }: ToastScreenProps = $props();
</script>

<div class="toast" role="status" in:fly={{ x: 40, duration: 220 }} out:fade={{ duration: 200 }}><i></i>{message}</div>

<style>
  .toast { position: absolute; right: 4rem; top: 3rem; z-index: 40; display: flex; align-items: center; gap: 1.2rem; max-width: 44rem; padding: 1.3rem 2rem; border-radius: var(--r); background: rgba(36, 36, 36, 0.97); color: #fff; font-family: var(--font); font-size: 1.35rem; box-shadow: 0 0.6rem 2rem rgba(0, 0, 0, 0.55); }
  i { flex: none; width: 0.3rem; align-self: stretch; border-radius: 0.2rem; background: var(--accent); }
</style>
