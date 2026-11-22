<script lang="ts">
  import { fade } from 'svelte/transition';
  import { api } from './api';
  import { prefs } from './prefs.svelte';

  let url = $state('');
  let urls = $state<string[]>([]);
  let cur = $state(0);

  $effect(() => {
    const w = prefs.wallpaper;
    url = '';
    urls = [];
    cur = 0;
    if ((w.type === 'image' || w.type === 'video') && w.path) api.mediaUrl(w.path).then((u) => (url = u)).catch(() => {});
    if (w.type === 'folder' && w.path) api.folderImages(w.path).then((u) => (urls = u)).catch(() => {});
  });

  $effect(() => {
    if (prefs.wallpaper.type !== 'folder' || urls.length < 2) return;
    const id = setInterval(() => (cur = (cur + 1) % urls.length), Math.max(5, prefs.wallpaper.interval) * 1000);
    return () => clearInterval(id);
  });

  const w = $derived(prefs.wallpaper);
</script>

<div class="wp" class:aurora={w.type === 'aurora'} style:background={w.type === 'color' ? w.color : ''}>
  {#if w.type === 'aurora'}
    <i class="b1"></i><i class="b2"></i><i class="b3"></i>
  {:else if w.type === 'image' && url}
    <img src={url} alt="" style:filter="blur({w.blur}px)" />
  {:else if w.type === 'folder' && urls.length}
    {#key urls[cur]}
      <img src={urls[cur]} alt="" style:filter="blur({w.blur}px)" transition:fade={{ duration: 1400 }} />
    {/key}
  {:else if w.type === 'video' && url}
    <video src={url} autoplay loop muted playsinline style:filter="blur({w.blur}px)"></video>
  {/if}
  <div class="dim" style:background="rgba(0,0,0,{w.type === 'aurora' ? 0 : w.dim / 100})"></div>
</div>

<style>
  .wp { position: absolute; inset: 0; overflow: hidden; }
  img, video { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; transform: scale(1.06); }
  .dim { position: absolute; inset: 0; }

  /* Aurore : trois halos flous qui dérivent lentement, aux couleurs de l'accent. */
  .aurora i { position: absolute; width: 60rem; height: 60rem; border-radius: 50%; filter: blur(9rem); opacity: 0.55; will-change: transform; }
  .b1 { left: -12rem; top: -18rem; background: var(--accent); animation: d1 38s ease-in-out infinite alternate; }
  .b2 { right: -14rem; top: -10rem; background: color-mix(in srgb, var(--accent) 35%, #3b4bff); animation: d2 46s ease-in-out infinite alternate; }
  .b3 { left: 28%; bottom: -34rem; background: color-mix(in srgb, var(--accent) 40%, #7a1fff); opacity: 0.35; animation: d3 52s ease-in-out infinite alternate; }
  @keyframes d1 { to { transform: translate(24rem, 10rem) scale(1.15); } }
  @keyframes d2 { to { transform: translate(-22rem, 14rem) scale(0.9); } }
  @keyframes d3 { to { transform: translate(-16rem, -10rem) scale(1.2); } }
</style>
