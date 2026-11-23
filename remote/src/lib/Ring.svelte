<script lang="ts">
  import { send } from './connection.svelte.ts';
  import { longpress, repeat } from './haptics.ts';
  import type { KeyName } from './protocol/types.ts';
  import { t } from './i18n.ts';

  // Anneau directionnel façon télécommande Google TV : quatre quartiers + OK au centre.
  let { onok, altOpen = false }: { onok: () => void; altOpen?: boolean } = $props();
  const key = (k: KeyName) => () => send({ t: 'key', k });
</script>

<div class="ring">
  <button class="dir up" aria-label="Haut" use:repeat={key('up')}>
    <svg viewBox="0 0 24 24"><path d="M7 15l5-5 5 5" /></svg>
  </button>
  <button class="dir right" aria-label="Droite" use:repeat={key('right')}>
    <svg viewBox="0 0 24 24"><path d="M9 7l5 5-5 5" /></svg>
  </button>
  <button class="dir down" aria-label="Bas" use:repeat={key('down')}>
    <svg viewBox="0 0 24 24"><path d="M7 9l5 5 5-5" /></svg>
  </button>
  <button class="dir left" aria-label="Gauche" use:repeat={key('left')}>
    <svg viewBox="0 0 24 24"><path d="M15 7l-5 5 5 5" /></svg>
  </button>
  <span class="cut a"></span><span class="cut b"></span>
  <button class="ok" aria-label="OK" use:longpress={{ onTap: onok, onLong: key('menu') }}>
    {altOpen ? t('validate') : 'OK'}
  </button>
</div>

<style>
  .ring {
    position: relative;
    width: min(100%, 300px);
    aspect-ratio: 1;
    margin: 4px auto;
    border-radius: 50%;
    background: radial-gradient(circle at 50% 35%, #2c3444, #1d2230 70%);
    box-shadow: 0 10px 30px #0008, inset 0 1px 0 #ffffff14;
    flex: none;
  }
  .dir {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: transparent;
    display: flex;
    padding: 0;
  }
  .dir svg {
    width: 15%;
    fill: none;
    stroke: var(--text);
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
    pointer-events: none;
  }
  .dir:active {
    background: #ffffff1f;
  }
  .up {
    clip-path: polygon(50% 50%, 0 0, 100% 0);
    align-items: flex-start;
    justify-content: center;
    padding-top: 9%;
  }
  .down {
    clip-path: polygon(50% 50%, 0 100%, 100% 100%);
    align-items: flex-end;
    justify-content: center;
    padding-bottom: 9%;
  }
  .left {
    clip-path: polygon(50% 50%, 0 0, 0 100%);
    align-items: center;
    justify-content: flex-start;
    padding-left: 9%;
  }
  .right {
    clip-path: polygon(50% 50%, 100% 0, 100% 100%);
    align-items: center;
    justify-content: flex-end;
    padding-right: 9%;
  }
  .cut {
    position: absolute;
    left: 6%;
    right: 6%;
    top: calc(50% - 1px);
    height: 2px;
    background: var(--bg);
    opacity: 0.55;
    pointer-events: none;
  }
  .a {
    transform: rotate(45deg);
  }
  .b {
    transform: rotate(-45deg);
  }
  .ok {
    position: absolute;
    left: 31%;
    top: 31%;
    width: 38%;
    height: 38%;
    border-radius: 50%;
    background: linear-gradient(160deg, #3a4357, #262d3c);
    box-shadow: 0 4px 14px #0009, inset 0 1px 0 #ffffff1c;
    font-size: 1.05rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: 0;
  }
  .ok:active {
    background: var(--accent);
  }
</style>
