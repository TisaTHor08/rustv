<script lang="ts">
  import { attachHost } from './modHost';
  import { modWidgets } from './mods.svelte';

  let { toast }: { toast: (m: string) => void } = $props();
  const widgets = $derived(modWidgets());

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
