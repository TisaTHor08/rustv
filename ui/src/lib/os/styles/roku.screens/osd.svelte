<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { OsdScreenProps } from '../../screens';

  // Volume style Roku : barre horizontale en bas au centre, icône à gauche, remplissage blanc sur fond violet.
  let { level, muted, label }: OsdScreenProps = $props();
</script>

<div class="osd" role="status" aria-live="polite" in:fly={{ y: 20, duration: 160 }} out:fade={{ duration: 200 }}>
  <svg viewBox="0 0 24 24"><path d="M3 9v6h4l5 5V4L7 9H3Z" />{#if muted}<path d="m16 9 5 6m0-6-5 6" stroke="currentColor" stroke-width="2" fill="none" />{:else}<path d="M15 8.5a5 5 0 0 1 0 7M17.5 6a8.5 8.5 0 0 1 0 12" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" />{/if}</svg>
  <span class="lab">{label}</span>
  <span class="track"><i style:width="{muted ? 0 : level}%"></i></span>
  <b>{muted ? 0 : level}</b>
</div>

<style>
  .osd { position: absolute; left: 50%; bottom: 4rem; transform: translateX(-50%); z-index: 45; display: flex; align-items: center; gap: 1.4rem; width: 52rem; box-sizing: border-box; padding: 1.2rem 2rem; border-radius: 0.6rem; background: rgba(43, 15, 69, 0.94); color: #fff; font-family: var(--font); box-shadow: 0 0.6rem 2.4rem rgba(0, 0, 0, 0.55); pointer-events: none; }
  svg { flex: none; width: 2.6rem; height: 2.6rem; fill: currentColor; }
  .lab { font-size: 1.3rem; font-weight: 600; }
  .track { flex: 1; height: 0.8rem; border-radius: 0.2rem; background: rgba(255, 255, 255, 0.28); overflow: hidden; }
  i { display: block; height: 100%; background: #fff; transition: width 0.12s; }
  b { min-width: 3.4rem; text-align: right; font-size: 1.6rem; font-variant-numeric: tabular-nums; }
</style>
