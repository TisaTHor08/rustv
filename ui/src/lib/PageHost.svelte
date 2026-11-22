<script lang="ts">
  import { fade } from 'svelte/transition';
  import { attachHost, forwardKey } from './modHost';
  import { t } from './i18n';

  let { modId, src, title, onclose, toast }: { modId: string; src: string; title: string; onclose: () => void; toast: (m: string) => void } = $props();
  let frame = $state<HTMLIFrameElement | null>(null);

  $effect(() => {
    if (!frame) return;
    return attachHost(frame, modId, { toast, close: onclose });
  });

  // L'hôte garde le focus : il transmet les touches de la télécommande à la page, sauf Retour qui ferme.
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' || e.key === 'Backspace') onclose();
    else forwardKey(frame, e.key);
    e.preventDefault();
    e.stopImmediatePropagation();
  }
</script>

<svelte:window onkeydowncapture={onKey} />

<div class="page" transition:fade={{ duration: 180 }}>
  <iframe bind:this={frame} {title} {src} sandbox="allow-scripts"></iframe>
  <div class="bar">{title} · {t('common.back')} ↩</div>
</div>

<style>
  .page { position: absolute; inset: 0; z-index: 25; background: var(--bg); }
  iframe { position: absolute; inset: 0; width: 100%; height: 100%; border: 0; background: var(--bg); }
  .bar { position: absolute; left: 50%; bottom: 1.2rem; transform: translateX(-50%); padding: 0.5rem 1.4rem; border-radius: 99rem; background: var(--sheet); color: var(--muted); font-size: 1rem; backdrop-filter: blur(1rem); pointer-events: none; opacity: 0.85; }
</style>
