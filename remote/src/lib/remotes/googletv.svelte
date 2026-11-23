<script lang="ts">
  // Télécommande vocale du boîtier Google TV : corps blanc « neige », grand anneau directionnel, boutons ronds, bascule de volume sur le côté.
  import { app } from '../connection.svelte.ts';
  import { home, key, media, search } from './actions.ts';
  import { labels } from './labels.ts';
  import type { RemoteCtx } from './types.ts';
  import Btn from './parts/Btn.svelte';
  import Dpad from './parts/Dpad.svelte';
  import Icon from './parts/Icon.svelte';
  import Rocker from './parts/Rocker.svelte';
  import Shortcut from './parts/Shortcut.svelte';

  let { ctx }: { ctx: RemoteCtx } = $props();
  const L = labels('googletv');
  const names = { up: L('up'), down: L('down'), left: L('left'), right: L('right'), ok: L('ok') };
</script>

<div class="gtv">
  <div class="top">
    <Btn cls="rb" label={L('assistant')} onTap={search}><Icon name="mic" size={22} /></Btn>
    <Btn cls="rb" label={L('input')} onTap={home}><Icon name="input" size={22} /></Btn>
  </div>

  <Dpad onok={ctx.ok} ok="" {names} />

  <div class="lower">
    <div class="lcol">
      <div class="pair">
        <Btn cls="rb big" kind="long" label={L('back')} onTap={key('back')} onLong={key('backlong')}><Icon name="back" size={26} sw={2.4} /></Btn>
        <Btn cls="rb big" strong label={L('home')} onTap={home}><Icon name="home" size={26} /></Btn>
      </div>
      <Btn cls="pill" label={L('mute')} onTap={media('mute')}><Icon name={app.volume.muted ? 'mute' : 'vol'} size={22} /></Btn>
      <Shortcut cls="pill" id="youtube" n={1} label={L('shortcut', 1)} />
      <Shortcut cls="pill" id="netflix" n={2} label={L('shortcut', 2)} />
    </div>
    <Rocker onUp={media('volUp')} onDown={media('volDown')} upLabel={L('volUp')} downLabel={L('volDown')} mid={app.volume.known ? String(app.volume.level) : 'VOL'} />
  </div>
</div>

<style>
  .gtv {
    --rm-body: #f1efeb;
    --rm-key: #e2dfd9;
    --rm-ink: #3b3d41;
    --dp-bg: linear-gradient(160deg, #e9e6e0, #dcd8d1);
    --dp-shadow: inset 0 2px 5px #0000001f, inset 0 -2px 3px #ffffffcc;
    --dp-ico: var(--rm-ink);
    --dp-press: #00000018;
    --dp-ok: 36%;
    --dp-ok-bg: var(--rm-body);
    --dp-ok-shadow: 0 3px 8px #00000030, inset 0 1px 0 #fff;
    --dp-ok-fg: var(--rm-ink);
    --rk-bg: var(--rm-key);
    --rk-ink: var(--rm-ink);
    --rk-w: 58px;
    --rk-h: 168px;
    --rk-shadow: inset 0 2px 4px #0000001a, inset 0 -1px 0 #fff;
    --rk-press: #00000018;
    width: min(100%, 286px);
    margin: 0 auto;
    padding: 20px 20px 24px;
    border-radius: 46px 46px 56px 56px;
    background: linear-gradient(170deg, #fbfaf8, var(--rm-body) 40%, #e8e5df);
    box-shadow: 0 14px 34px #0009, inset 0 1px 0 #fff, inset 0 -3px 6px #00000014;
    display: flex;
    flex-direction: column;
    gap: 16px;
    flex: none;
    color: var(--rm-ink);
  }
  .top {
    display: flex;
    justify-content: space-between;
    padding: 0 6px;
  }
  .gtv :global(.rb) {
    width: 46px;
    height: 46px;
    border-radius: 50%;
    background: var(--rm-key);
    color: var(--rm-ink);
    box-shadow: inset 0 2px 3px #00000014, 0 1px 0 #fff;
  }
  .gtv :global(.rb.big) {
    width: 58px;
    height: 58px;
  }
  .gtv :global(.pill) {
    width: 100%;
    height: 46px;
    border-radius: 999px;
    background: var(--rm-key);
    color: var(--rm-ink);
    box-shadow: inset 0 2px 3px #00000014, 0 1px 0 #fff;
  }
  .gtv :global(.k:active) {
    background: color-mix(in srgb, var(--accent) 35%, var(--rm-key));
    transform: scale(0.97);
  }
  .lower {
    display: flex;
    gap: 14px;
    align-items: stretch;
  }
  .lcol {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }
  .pair {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .lower :global(.rk) {
    min-height: 0;
    height: auto;
    align-self: stretch;
  }
</style>
