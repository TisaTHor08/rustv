<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { OsdScreenProps } from '../../screens';

  // Volume style webOS : bloc sombre à coins peu arrondis en bas au centre, haut-parleur, grand chiffre et barre blanche fine.
  let { level, muted }: OsdScreenProps = $props();
</script>

<div class="osd" role="status" aria-live="polite" in:fly={{ y: 20, duration: 180 }} out:fade={{ duration: 220 }}>
  <svg viewBox="0 0 24 24"><path d="M3 9v6h4l5 5V4L7 9H3Z" />{#if muted}<path d="m16 9 5 6m0-6-5 6" stroke="currentColor" stroke-width="2" fill="none" />{:else}<path d="M15 8.5a5 5 0 0 1 0 7M17.5 6a8.5 8.5 0 0 1 0 12" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" />{/if}</svg>
  <b>{muted ? '0' : level}</b>
  <span class="track"><i style:width="{muted ? 0 : level}%"></i></span>
</div>

<style>
  .osd { position: absolute; left: 50%; bottom: 4rem; transform: translateX(-50%); z-index: 45; width: 46rem; box-sizing: border-box; padding: 1.2rem 2.2rem; display: flex; align-items: center; gap: 1.6rem; border-radius: 0.9rem; background: rgba(14, 16, 22, 0.94); color: #f4f7fb; font-family: var(--font); box-shadow: 0 1rem 3rem rgba(0, 0, 0, 0.55); pointer-events: none; }
  svg { flex: none; width: 3rem; height: 3rem; fill: currentColor; }
  b { min-width: 4.6rem; font-size: 3.2rem; font-weight: 400; font-variant-numeric: tabular-nums; }
  .track { position: relative; flex: 1; height: 0.5rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.25); overflow: hidden; }
  i { position: absolute; left: 0; top: 0; bottom: 0; background: #fff; transition: width 0.15s; }
</style>
