<script lang="ts">
  import { repeat } from '../../haptics.ts';
  import Icon from './Icon.svelte';

  // Bascule + / - (volume ou chaîne) : pastille verticale (ou horizontale), répétition en maintenant.
  // L'habillage vient des variables --rk-* du modèle.
  let {
    onUp,
    onDown,
    upLabel,
    downLabel,
    mid = '',
    horizontal = false,
  }: { onUp: () => void; onDown: () => void; upLabel: string; downLabel: string; mid?: string; horizontal?: boolean } = $props();
</script>

<div class="rk" class:h={horizontal}>
  <button aria-label={upLabel} use:repeat={onUp}><Icon name="plus" size={22} sw={2.4} /></button>
  <small>{mid}</small>
  <button aria-label={downLabel} use:repeat={onDown}><Icon name="minus" size={22} sw={2.4} /></button>
</div>

<style>
  .rk {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    width: var(--rk-w, 56px);
    min-height: var(--rk-h, 150px);
    border-radius: var(--rk-r, 999px);
    background: var(--rk-bg, #333);
    color: var(--rk-ink, var(--text));
    box-shadow: var(--rk-shadow, none);
    overflow: hidden;
    flex: none;
  }
  .rk.h {
    flex-direction: row-reverse;
    width: var(--rk-h, 150px);
    min-height: var(--rk-w, 56px);
  }
  button {
    flex: 1;
    display: grid;
    place-items: center;
    background: transparent;
    color: inherit;
    border-radius: 0;
    padding: 0;
    min-width: 44px;
    min-height: 44px;
  }
  button:active {
    background: var(--rk-press, #ffffff26);
    transform: none;
  }
  small {
    align-self: center;
    font-size: 0.68rem;
    min-height: 1em;
    color: var(--rk-mute, currentColor);
    opacity: 0.7;
    text-align: center;
    flex: none;
  }
</style>
