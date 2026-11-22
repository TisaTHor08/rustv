<script lang="ts">
  import { app } from './lib/connection.svelte.ts';
  import Pair from './lib/Pair.svelte';
  import Remote from './lib/Remote.svelte';
</script>

{#if app.pairing}
  <Remote />
{:else}
  <Pair />
{/if}
