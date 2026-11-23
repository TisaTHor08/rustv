<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import type { OsdScreenProps } from '../../screens';

  // Volume style tvOS / iOS : pilule en verre dépoli en haut à droite, icône haut-parleur et barre blanche arrondie.
  // (tvOS n'a pas de HUD de volume sur l'appareil ; on reprend le langage visuel de ses bandeaux.)
  let { level, muted, label }: OsdScreenProps = $props();
</script>

<div class="osd" role="status" aria-live="polite" in:fly={{ y: -16, duration: 180 }} out:fade={{ duration: 240 }}>
  <svg viewBox="0 0 24 24"><path d="M4 9.5v5h3.5l4.5 4v-13l-4.5 4Z" fill="currentColor" stroke="none" />{#if muted}<path d="m16 9.5 5 5m0-5-5 5" />{:else}<path d="M15 9.5a3.5 3.5 0 0 1 0 5M17.5 7a7 7 0 0 1 0 10" />{/if}</svg>
  <span class="track" aria-label={label}><i style:width="{muted ? 0 : level}%"></i></span>
</div>

<style>
  .osd { position: absolute; right: 5rem; top: 2.2rem; z-index: 45; display: flex; align-items: center; gap: 1.2rem; width: 24rem; box-sizing: border-box; padding: 1.2rem 1.8rem; border-radius: 99rem; background: rgba(60, 60, 67, 0.6); backdrop-filter: blur(2.4rem) saturate(1.8); color: #fff; font-family: var(--font); box-shadow: inset 0 0 0 0.06rem rgba(255, 255, 255, 0.16), 0 0.8rem 2rem rgba(0, 0, 0, 0.4); pointer-events: none; }
  svg { flex: none; width: 2.4rem; height: 2.4rem; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; }
  .track { flex: 1; height: 0.8rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.28); overflow: hidden; }
  i { display: block; height: 100%; border-radius: 99rem; background: #fff; transition: width 0.12s; }
</style>
