<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { ToastScreenProps } from '../../screens';

  // Message style Google TV : bandeau sombre arrondi en bas à gauche, texte clair, pastille d'information.
  let { message }: ToastScreenProps = $props();
</script>

<div class="toast" role="status" in:fly={{ y: 16, duration: 200 }} out:fade={{ duration: 200 }}>
  <svg viewBox="0 0 24 24"><path d="M11 7h2v2h-2zm0 4h2v6h-2zm1-9a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm0 18a8 8 0 1 1 0-16 8 8 0 0 1 0 16Z" /></svg>
  <span>{message}</span>
</div>

<style>
  .toast { position: absolute; left: 5.4rem; bottom: 3rem; z-index: 40; display: flex; align-items: center; gap: 1.1rem; max-width: 60rem; padding: 1.1rem 2rem 1.1rem 1.5rem; border-radius: 1.4rem; background: #303134; color: #e8eaed; font-family: var(--font); font-size: 1.35rem; box-shadow: 0 0.8rem 2.4rem rgba(0, 0, 0, 0.45); }
  svg { flex: none; width: 2.2rem; height: 2.2rem; fill: #8ab4f8; }
</style>
