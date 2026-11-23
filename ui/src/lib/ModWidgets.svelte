<script lang="ts">
  import { attachHost } from './modHost';
  import { modWidgets } from './mods.svelte';
  import { resolveWidgetSlot, type WidgetSlot } from './os/registry';

  // `at` + `provided` : n'affiche que les widgets dont l'emplacement (demandé par le manifeste, ramené à l'emplacement
  // fourni par le style le plus proche) est `at`. Sans `at` : tous les widgets (comportement d'origine / dock).
  let { toast, at, provided = ['status'] }: { toast: (m: string) => void; at?: WidgetSlot; provided?: WidgetSlot[] } = $props();
  const widgets = $derived(modWidgets().filter((w) => !at || resolveWidgetSlot(w.slot, provided) === at));

  /** Action Svelte : branche l'hôte RPC sur l'iframe du widget. */
  function host(node: HTMLIFrameElement, modId: string) {
    const off = attachHost(node, modId, { toast });
    return { destroy: off };
  }
</script>

{#each widgets as w (w.key)}
  <iframe use:host={w.modId} title={w.key} src={w.src} sandbox="allow-scripts" style:width="{w.width}rem" scrolling="no"></iframe>
{/each}

<style>
  iframe { height: 3.4rem; border: 0; background: transparent; border-radius: 1rem; }
</style>
