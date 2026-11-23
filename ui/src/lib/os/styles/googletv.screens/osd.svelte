<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { OsdScreenProps } from '../../screens';

  // Volume style Google TV : colonne verticale à droite (icône haut-parleur, remplissage, valeur).
  let { level, muted }: OsdScreenProps = $props();
</script>

<div class="osd" role="status" aria-live="polite" in:fly={{ x: 24, duration: 180 }} out:fade={{ duration: 220 }}>
  <svg viewBox="0 0 24 24"><path d="M3 9v6h4l5 5V4L7 9H3Z" />{#if muted}<path d="m16 9 5 6m0-6-5 6" stroke="currentColor" stroke-width="2" fill="none" />{:else}<path d="M15 8.5a5 5 0 0 1 0 7M17.5 6a8.5 8.5 0 0 1 0 12" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" />{/if}</svg>
  <span class="track"><i style:height="{muted ? 0 : level}%"></i></span>
  <b>{muted ? '0' : level}</b>
</div>

<style>
  .osd { position: absolute; right: 3rem; top: 50%; transform: translateY(-50%); z-index: 45; width: 5.6rem; box-sizing: border-box; padding: 1.3rem 0 1.1rem; display: grid; justify-items: center; gap: 1rem; border-radius: 99rem; background: #2d2f33; color: #e8eaed; font-family: var(--font); box-shadow: 0 1rem 3rem rgba(0, 0, 0, 0.5); pointer-events: none; }
  svg { width: 2.4rem; height: 2.4rem; fill: currentColor; }
  .track { position: relative; width: 0.8rem; height: 16rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.22); overflow: hidden; }
  i { position: absolute; left: 0; right: 0; bottom: 0; background: #8ab4f8; border-radius: 99rem; transition: height 0.15s; }
  b { font-size: 1.5rem; font-weight: 500; font-variant-numeric: tabular-nums; }
</style>
