<script lang="ts">
  import type { Snippet } from 'svelte';
  import { longpress, repeat } from '../../haptics.ts';
  import { key, menu } from '../actions.ts';
  import Icon from './Icon.svelte';

  // Croix directionnelle commune : quatre quartiers (répétition en maintenant) + OK au centre
  // (appui long = touche Menu, comme l'anneau d'origine). L'habillage vient des variables --dp-* du modèle.
  let {
    onok,
    ok = 'OK',
    names,
    center,
  }: {
    onok: () => void;
    ok?: string;
    /** Libellés accessibles : up, down, left, right, ok. */
    names: { up: string; down: string; left: string; right: string; ok: string };
    /** Remplace le bouton OK (molette webOS). */
    center?: Snippet;
  } = $props();
</script>

<div class="dp">
  <button class="dir up" aria-label={names.up} use:repeat={key('up')}><Icon name="up" sw={2.4} /></button>
  <button class="dir right" aria-label={names.right} use:repeat={key('right')}><Icon name="right" sw={2.4} /></button>
  <button class="dir down" aria-label={names.down} use:repeat={key('down')}><Icon name="down" sw={2.4} /></button>
  <button class="dir left" aria-label={names.left} use:repeat={key('left')}><Icon name="left" sw={2.4} /></button>
  <span class="cut a"></span><span class="cut b"></span>
  {#if center}
    <div class="mid">{@render center()}</div>
  {:else}
    <button class="ok" aria-label={names.ok} use:longpress={{ onTap: onok, onLong: menu }}>{ok}</button>
  {/if}
</div>

<style>
  .dp {
    position: relative;
    width: var(--dp-size, min(100%, 260px));
    aspect-ratio: 1;
    margin: 0 auto;
    flex: none;
    border-radius: var(--dp-r, 50%);
    background: var(--dp-bg, #222);
    box-shadow: var(--dp-shadow, none);
    color: var(--dp-ico, var(--text));
  }
  .dir {
    position: absolute;
    inset: 0;
    display: flex;
    padding: 0;
    background: transparent;
    color: inherit;
    border-radius: var(--dp-r, 50%);
  }
  .dir :global(svg) {
    width: var(--dp-arrow, 15%);
    height: auto;
    pointer-events: none;
  }
  .dir:active {
    background: var(--dp-press, #ffffff22);
    transform: none;
  }
  .up {
    clip-path: polygon(50% 50%, 0 0, 100% 0);
    align-items: flex-start;
    justify-content: center;
    padding-top: var(--dp-pad, 9%);
  }
  .down {
    clip-path: polygon(50% 50%, 0 100%, 100% 100%);
    align-items: flex-end;
    justify-content: center;
    padding-bottom: var(--dp-pad, 9%);
  }
  .left {
    clip-path: polygon(50% 50%, 0 0, 0 100%);
    align-items: center;
    justify-content: flex-start;
    padding-left: var(--dp-pad, 9%);
  }
  .right {
    clip-path: polygon(50% 50%, 100% 0, 100% 100%);
    align-items: center;
    justify-content: flex-end;
    padding-right: var(--dp-pad, 9%);
  }
  .cut {
    position: absolute;
    left: var(--dp-cut-in, 8%);
    right: var(--dp-cut-in, 8%);
    top: calc(50% - 1px);
    height: 2px;
    background: var(--dp-cut, transparent);
    pointer-events: none;
  }
  .a {
    transform: rotate(45deg);
  }
  .b {
    transform: rotate(-45deg);
  }
  .ok,
  .mid {
    position: absolute;
    left: calc(50% - var(--dp-ok, 38%) / 2);
    top: calc(50% - var(--dp-ok, 38%) / 2);
    width: var(--dp-ok, 38%);
    height: var(--dp-ok, 38%);
    border-radius: 50%;
    padding: 0;
  }
  .ok {
    background: var(--dp-ok-bg, #333);
    color: var(--dp-ok-fg, var(--text));
    box-shadow: var(--dp-ok-shadow, none);
    font-size: 1.05rem;
    font-weight: 700;
    letter-spacing: 0.04em;
  }
  .ok:active {
    background: var(--accent);
    color: #fff;
    transform: none;
  }
</style>
