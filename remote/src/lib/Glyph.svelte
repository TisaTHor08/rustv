<script lang="ts">
  import { GLYPHS } from './glyphs.ts';
  let { name, size = 24 }: { name: string; size?: number } = $props();
  const paths = $derived(name && Object.hasOwn(GLYPHS, name) ? GLYPHS[name] : null);
</script>

{#if paths}
  <svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    {#each paths as d}<path {d} />{/each}
  </svg>
{/if}
