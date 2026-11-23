<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { ToastScreenProps } from '../../screens';

  // Bandeau style tvOS : pilule en verre dépoli en haut au centre, texte blanc moyen, entre par le haut avec un léger ressort.
  let { message }: ToastScreenProps = $props();
</script>

<div class="toast" role="status" in:fly={{ y: -24, duration: 260 }} out:fade={{ duration: 220 }}>{message}</div>

<style>
  .toast { position: absolute; left: 50%; top: 2.2rem; transform: translateX(-50%); z-index: 40; max-width: 64rem; padding: 1.2rem 2.6rem; border-radius: 99rem; background: rgba(60, 60, 67, 0.62); backdrop-filter: blur(2.4rem) saturate(1.8); color: #fff; font-family: var(--font); font-size: 1.45rem; font-weight: 500; text-align: center; box-shadow: inset 0 0 0 0.06rem rgba(255, 255, 255, 0.16), 0 0.8rem 2.2rem rgba(0, 0, 0, 0.45); }
</style>
