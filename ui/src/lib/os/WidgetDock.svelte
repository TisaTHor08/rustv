<script lang="ts">
  import ModWidgets from '../ModWidgets.svelte';
  import Weather from '../Weather.svelte';
  import type { HomeModel } from './model.svelte';

  // Groupe de widgets posé par Home quand l'utilisateur a choisi une position (réglage « Position des widgets »).
  // Indépendant du style : même contenu partout (téléphones, météo, widgets de mods, horloge), coin choisi.
  let { model }: { model: HomeModel } = $props();
</script>

{#if model.widgetPos !== 'auto' && model.widgetPos !== 'hidden'}
  <div class="dock {model.widgetPos}" aria-label="widgets">
    <ModWidgets toast={model.note} />
    <Weather />
    <div class="pill" class:on={model.phones > 0}><span class="dot"></span>{model.phonesLabel}</div>
    <div class="clock"><strong>{model.time}</strong><small>{model.date}</small></div>
  </div>
{/if}

<style>
  .dock { position: absolute; z-index: 6; display: flex; align-items: center; gap: 1rem; padding: 0.5rem 1.2rem; border-radius: 99rem; background: rgba(var(--bg-rgb), 0.72); backdrop-filter: blur(1.2rem); box-shadow: inset 0 0 0 0.08rem var(--glass-strong); color: var(--text); font-family: var(--font); max-width: 60%; pointer-events: none; }
  /* Marges par défaut ; chaque style peut les ajuster avec --dock-top/-left/-right/-bottom dans ses jetons. */
  .tr { top: var(--dock-top, 1.6rem); right: var(--dock-right, 2rem); }
  .tl { top: var(--dock-top, 1.6rem); left: var(--dock-left, 2rem); }
  .br { bottom: var(--dock-bottom, 1.6rem); right: var(--dock-right, 2rem); }
  .bl { bottom: var(--dock-bottom, 1.6rem); left: var(--dock-left, 2rem); }
  .pill { display: flex; align-items: center; gap: 0.5rem; font-size: 0.95rem; color: var(--muted); white-space: nowrap; }
  .pill .dot { width: 0.55rem; height: 0.55rem; border-radius: 50%; background: #888; }
  .pill.on { color: var(--text); }
  .pill.on .dot { background: #3ddc84; box-shadow: 0 0 0.6rem #3ddc84; }
  .clock { display: flex; align-items: baseline; gap: 0.6rem; white-space: nowrap; }
  .clock strong { font-size: 1.4rem; font-weight: 600; }
  .clock small { font-size: 0.85rem; color: var(--muted); }
  .clock small::first-letter { text-transform: uppercase; }
</style>
