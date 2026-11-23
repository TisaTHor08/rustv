<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { OsdScreenProps } from '../screens';

  // OSD de volume par défaut (RusTV) : barre fine en bas au centre.
  let { level, muted, label }: OsdScreenProps = $props();
</script>

<div class="osd" role="status" aria-live="polite" in:fly={{ y: 16, duration: 180 }} out:fade={{ duration: 220 }}>
  <span class="ic" class:muted>
    <svg viewBox="0 0 24 24"><path d="M3 9v6h4l5 5V4L7 9H3Z" />{#if muted}<path d="m16 9 5 6m0-6-5 6" stroke="currentColor" stroke-width="2" fill="none" />{:else}<path d="M15 8.5a5 5 0 0 1 0 7M17.5 6a8.5 8.5 0 0 1 0 12" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" />{/if}</svg>
  </span>
  <span class="bar"><i style:width="{muted ? 0 : level}%"></i></span>
  <b>{muted ? label : level}</b>
</div>

<style>
  .osd { position: absolute; left: 50%; bottom: 2.6rem; transform: translateX(-50%); z-index: 45; display: flex; align-items: center; gap: 1.1rem; min-width: 26rem; padding: 0.9rem 1.5rem; border-radius: 99rem; background: rgba(var(--bg-rgb), 0.88); backdrop-filter: blur(1.2rem); box-shadow: inset 0 0 0 0.08rem var(--glass-strong), 0 1rem 2.4rem rgba(0, 0, 0, 0.4); color: var(--text); font-family: var(--font); pointer-events: none; }
  .ic svg { display: block; width: 1.8rem; height: 1.8rem; fill: currentColor; }
  .ic.muted { opacity: 0.6; }
  .bar { flex: 1; height: 0.5rem; border-radius: 99rem; background: var(--glass-strong); overflow: hidden; }
  .bar i { display: block; height: 100%; border-radius: 99rem; background: var(--accent); transition: width 0.15s; }
  b { min-width: 2.6rem; text-align: right; font-size: 1.2rem; font-weight: 700; font-variant-numeric: tabular-nums; }
</style>
