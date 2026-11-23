<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { OsdScreenProps } from '../../screens';

  // Volume style Fire TV : pilule sombre translucide en bas au centre, icône haut-parleur, barre blanche fine et valeur numérique.
  let { level, muted, label }: OsdScreenProps = $props();
</script>

<div class="osd" role="status" aria-live="polite" in:fly={{ y: 24, duration: 160 }} out:fade={{ duration: 200 }}>
  <svg viewBox="0 0 24 24"><path d="M4 9.5v5h3.5l4.5 4v-13l-4.5 4Z" fill="currentColor" stroke="none" />{#if muted}<path d="m16 9.5 5 5m0-5-5 5" />{:else}<path d="M15 9.5a3.5 3.5 0 0 1 0 5M17.5 7a7 7 0 0 1 0 10" />{/if}</svg>
  <span class="track" aria-label={label}><i style:width="{muted ? 0 : level}%"></i></span>
  <b>{muted ? 0 : level}</b>
</div>

<style>
  .osd { position: absolute; left: 50%; bottom: 4.4rem; transform: translateX(-50%); z-index: 45; display: flex; align-items: center; gap: 1.4rem; width: 44rem; box-sizing: border-box; padding: 1.2rem 2.2rem; border-radius: 3rem; background: rgba(30, 30, 30, 0.94); color: #fff; font-family: var(--font); box-shadow: 0 0.6rem 2.4rem rgba(0, 0, 0, 0.55); pointer-events: none; }
  svg { flex: none; width: 2.6rem; height: 2.6rem; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; }
  .track { flex: 1; height: 0.5rem; border-radius: 0.3rem; background: rgba(255, 255, 255, 0.28); overflow: hidden; }
  i { display: block; height: 100%; background: #fff; transition: width 0.12s; }
  b { min-width: 3rem; text-align: right; font-size: 1.6rem; font-weight: 500; font-variant-numeric: tabular-nums; }
</style>
