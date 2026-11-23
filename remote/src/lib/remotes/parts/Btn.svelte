<script lang="ts">
  import type { Snippet } from 'svelte';
  import { longpress, repeat, tick } from '../../haptics.ts';

  // Bouton de télécommande : tap (haptique léger ou fort), répétition en maintenant, ou tap + appui long.
  let {
    kind = 'tap',
    onTap,
    onLong,
    label,
    cls = '',
    strong = false,
    active = false,
    children,
  }: {
    kind?: 'tap' | 'repeat' | 'long';
    onTap: () => void;
    onLong?: () => void;
    label: string;
    cls?: string;
    strong?: boolean;
    active?: boolean;
    children?: Snippet;
  } = $props();
</script>

{#if kind === 'repeat'}
  <button class="k {cls}" class:active aria-label={label} use:repeat={onTap}>{@render children?.()}</button>
{:else if kind === 'long'}
  <button class="k {cls}" class:active aria-label={label} use:longpress={{ onTap, onLong: onLong ?? (() => {}) }}>{@render children?.()}</button>
{:else}
  <button
    class="k {cls}"
    class:active
    aria-label={label}
    onpointerdown={(e) => {
      e.preventDefault();
      tick(strong);
      onTap();
    }}>{@render children?.()}</button
  >
{/if}

<style>
  .k {
    display: grid;
    place-items: center;
    padding: 0;
    min-width: 44px;
    min-height: 44px;
  }
</style>
