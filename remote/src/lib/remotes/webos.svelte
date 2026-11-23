<script lang="ts">
  // Magic Remote de LG : corps arrondi sombre, molette centrale (glisser = défiler, tap = OK) entourée des flèches,
  // Retour / Accueil / Réglages, bascules volume et chaîne, raccourcis streaming, et mode pointeur (pavé tactile).
  import { app } from '../connection.svelte.ts';
  import { home, key, media, menu } from './actions.ts';
  import { labels } from './labels.ts';
  import type { RemoteCtx } from './types.ts';
  import Touchpad from '../Touchpad.svelte';
  import Btn from './parts/Btn.svelte';
  import Dpad from './parts/Dpad.svelte';
  import Icon from './parts/Icon.svelte';
  import Rocker from './parts/Rocker.svelte';
  import Shortcut from './parts/Shortcut.svelte';
  import Wheel from './parts/Wheel.svelte';

  let { ctx }: { ctx: RemoteCtx } = $props();
  const L = labels('webos');
  const names = { up: L('up'), down: L('down'), left: L('left'), right: L('right'), ok: L('ok') };
  let pointer = $state(false);
</script>

<div class="wo">
  <div class="top">
    <span class="dotled" aria-hidden="true"></span>
    <Btn cls="rb" active={pointer} label={pointer ? L('arrows') : L('pointer')} onTap={() => (pointer = !pointer)}><Icon name="pointer" size={22} /></Btn>
  </div>

  {#if pointer}
    <div class="padbox"><Touchpad /></div>
  {:else}
    <Dpad onok={ctx.ok} {names}>
      {#snippet center()}<Wheel onok={ctx.ok} label={L('wheel')} />{/snippet}
    </Dpad>
  {/if}

  <div class="row3">
    <Btn cls="rb" kind="long" label={L('back')} onTap={key('back')} onLong={key('backlong')}><Icon name="back" size={26} sw={2.4} /></Btn>
    <Btn cls="rb" strong label={L('home')} onTap={home}><Icon name="home" size={24} /></Btn>
    <Btn cls="rb" label={L('settings')} onTap={menu}><Icon name="gear" size={24} /></Btn>
  </div>

  <div class="rk2">
    <Rocker onUp={media('volUp')} onDown={media('volDown')} upLabel={L('volUp')} downLabel={L('volDown')} mid={app.volume.known ? String(app.volume.level) : 'VOL'} />
    <Btn cls="rb" label={L('mute')} onTap={media('mute')}><Icon name={app.volume.muted ? 'mute' : 'vol'} size={22} /></Btn>
    <Rocker onUp={media('next')} onDown={media('prev')} upLabel={L('chUp')} downLabel={L('chDown')} mid="CH" />
  </div>

  <div class="shorts">
    <Shortcut cls="sc" id="netflix" n={1} label={L('shortcut', 1)} />
    <Shortcut cls="sc" id="prime" n={2} label={L('shortcut', 2)} />
    <Shortcut cls="sc" id="disney" n={3} label={L('shortcut', 3)} />
  </div>
</div>

<style>
  .wo {
    --dp-bg: linear-gradient(160deg, #34373c, #26282c);
    --dp-shadow: inset 0 1px 0 #ffffff1c, inset 0 -3px 8px #0007;
    --dp-ico: #e9ebef;
    --dp-ok: 50%;
    --dp-pad: 6%;
    --dp-arrow: 13%;
    --rk-bg: linear-gradient(90deg, #35383d, #2a2c31);
    --rk-ink: #e9ebef;
    --rk-w: 64px;
    --rk-h: 132px;
    --rk-r: 30px;
    --rk-shadow: inset 0 1px 0 #ffffff1c, inset 0 -2px 6px #0007;
    width: min(100%, 280px);
    margin: 0 auto;
    padding: 18px 22px 22px;
    border-radius: 44px 44px 60px 60px;
    background: linear-gradient(175deg, #2d3035, #1c1e21 60%, #17181b);
    box-shadow: 0 14px 34px #000b, inset 0 1px 0 #ffffff22;
    display: flex;
    flex-direction: column;
    gap: 16px;
    flex: none;
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .dotled {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #a50034;
    margin-left: 8px;
    box-shadow: 0 0 8px #a5003488;
  }
  .wo :global(.rb) {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: linear-gradient(160deg, #3b3e44, #2c2e33);
    color: #e9ebef;
    box-shadow: inset 0 1px 0 #ffffff1c, 0 2px 5px #0008;
  }
  .wo :global(.rb.active) {
    background: var(--accent);
    color: #fff;
  }
  .padbox {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 250px;
  }
  .padbox :global(.pad) {
    background: #1f2125;
    border-radius: 32px;
  }
  .padbox :global(.row button) {
    padding: 12px;
    background: #2c2e33;
  }
  .row3 {
    display: flex;
    justify-content: space-between;
  }
  .rk2 {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .shorts {
    display: flex;
    gap: 8px;
  }
  .wo :global(.sc) {
    flex: 1;
    height: 46px;
    border-radius: 14px;
    background: #2c2e33;
    color: #cfd3da;
    box-shadow: inset 0 1px 0 #ffffff14, 0 2px 4px #0007;
  }
  .wo :global(.k:active) {
    background: color-mix(in srgb, var(--accent) 55%, #2c2e33);
  }
</style>
