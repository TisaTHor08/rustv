<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { ToastScreenProps } from '../../screens';

  // Message style webOS : bloc sombre à coins peu arrondis en haut à droite, sous la zone d'horloge, avec une pastille d'information.
  let { message }: ToastScreenProps = $props();
</script>

<div class="toast" role="status" in:fly={{ y: -14, duration: 200 }} out:fade={{ duration: 200 }}>
  <i>i</i>
  <span>{message}</span>
</div>

<style>
  .toast { position: absolute; right: 3.4rem; top: 5.4rem; z-index: 40; display: flex; align-items: center; gap: 1.2rem; max-width: 52rem; padding: 1.1rem 2rem 1.1rem 1.3rem; border-radius: 0.8rem; background: rgba(20, 22, 30, 0.96); color: #f4f7fb; font-family: var(--font); font-size: 1.35rem; box-shadow: 0 0.8rem 2.4rem rgba(0, 0, 0, 0.55); }
  i { flex: none; width: 2.2rem; height: 2.2rem; border-radius: 50%; display: grid; place-items: center; background: #3a8cf0; font-style: normal; font-weight: 700; font-size: 1.3rem; }
</style>
