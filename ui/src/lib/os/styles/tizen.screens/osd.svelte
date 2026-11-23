<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { OsdScreenProps } from '../../screens';

  // Volume style Tizen : pastille sombre en bas à gauche avec haut-parleur, barre fine bleue et valeur.
  let { level, muted }: OsdScreenProps = $props();
</script>

<div class="osd" role="status" aria-live="polite" in:fly={{ y: 16, duration: 180 }} out:fade={{ duration: 220 }}>
  <svg viewBox="0 0 24 24"><path d="M3 9v6h4l5 5V4L7 9H3Z" />{#if muted}<path d="m16 9 5 6m0-6-5 6" stroke="currentColor" stroke-width="2" fill="none" />{:else}<path d="M15 8.5a5 5 0 0 1 0 7M17.5 6a8.5 8.5 0 0 1 0 12" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" />{/if}</svg>
  <span class="track"><i style:width="{muted ? 0 : level}%"></i></span>
  <b>{muted ? '0' : level}</b>
</div>

<style>
  .osd { position: absolute; left: 9rem; bottom: 3rem; z-index: 45; width: 30rem; box-sizing: border-box; padding: 1.1rem 2rem; display: flex; align-items: center; gap: 1.2rem; border-radius: 99rem; background: rgba(20, 21, 42, 0.92); color: #f4f5fa; font-family: var(--font); box-shadow: inset 0 0 0 0.1rem rgba(255, 255, 255, 0.3), 0 1rem 3rem rgba(0, 0, 0, 0.5); pointer-events: none; }
  svg { flex: none; width: 2.4rem; height: 2.4rem; fill: currentColor; }
  .track { position: relative; flex: 1; height: 0.6rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.25); overflow: hidden; }
  i { position: absolute; left: 0; top: 0; bottom: 0; background: #3d9bff; border-radius: 99rem; transition: width 0.15s; }
  b { min-width: 2.8rem; text-align: right; font-size: 1.6rem; font-weight: 500; font-variant-numeric: tabular-nums; }
</style>
