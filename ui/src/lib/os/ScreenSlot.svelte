<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import { osState } from './apply.svelte';
  import { screenCache } from './screencache.svelte';
  import type { ScreenName } from './screens';

  // Emplacement d'un écran commun : affiche l'écran du style actif (meta.screens[name]) s'il existe, sinon le contenu par défaut
  // (`fallback`, l'écran RusTV habillé par les jetons). Le composant du style est chargé à la demande puis gardé en cache.
  let { name, props, fallback }: { name: ScreenName; props: Record<string, any>; fallback: Snippet } = $props();

  const key = $derived(`${osState.style.id}/${name}`);
  const declared = $derived(osState.style.screens?.[name]);
  const Comp = $derived(screenCache.map[key]);

  $effect(() => {
    const k = key;
    const d = declared as (() => Promise<{ default: Component<any> }>) | undefined;
    if (!d || screenCache.map[k] || screenCache.failed[k]) return;
    d()
      .then((m) => (screenCache.map[k] = m.default))
      .catch(() => (screenCache.failed[k] = true));
  });
</script>

{#if !declared || screenCache.failed[key]}
  {@render fallback()}
{:else if Comp}
  <Comp {...props} />
{/if}
