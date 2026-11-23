<script lang="ts">
  // Smart Remote minimaliste de Samsung (2021+) : corps anthracite, bandeau solaire, anneau + OK, rangée Retour / Hub / Lecture, deux bascules verticales.
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
  const L = labels('tizen');
  const names = { up: L('up'), down: L('down'), left: L('left'), right: L('right'), ok: L('ok') };
</script>

<div class="tz">
  <Btn cls="mic" label={L('assistant')} onTap={search}><Icon name="mic" size={22} /></Btn>

  <Dpad onok={ctx.ok} ok="" {names} />

  <div class="row3">
    <Btn cls="rb" kind="long" label={L('back')} onTap={key('back')} onLong={key('backlong')}><Icon name="backArrow" size={24} /></Btn>
    <Btn cls="rb" strong label={L('home')} onTap={home}><Icon name="home" size={24} /></Btn>
    <Btn cls="rb" label={L('play')} onTap={media('playPause')}><Icon name="playpause" size={22} /></Btn>
  </div>

  <div class="rockers">
    <div class="r">
      <Rocker onUp={media('volUp')} onDown={media('volDown')} upLabel={L('volUp')} downLabel={L('volDown')} mid={app.volume.known ? String(app.volume.level) : 'VOL'} />
    </div>
    <div class="r">
      <Rocker onUp={media('next')} onDown={media('prev')} upLabel={L('chUp')} downLabel={L('chDown')} mid="CH" />
    </div>
  </div>

  <div class="shorts">
    <Shortcut cls="sc" id="netflix" n={1} label={L('shortcut', 1)} />
    <Shortcut cls="sc" id="prime" n={2} label={L('shortcut', 2)} />
    <Shortcut cls="sc" id="disney" n={3} label={L('shortcut', 3)} />
  </div>
  <div class="solar" aria-hidden="true"></div>
</div>

<style>
  .tz {
    --dp-bg: linear-gradient(160deg, #3b3e44, #2d2f34);
    --dp-shadow: inset 0 1px 0 #ffffff1f, inset 0 -3px 8px #0006;
    --dp-ico: #e9ebef;
    --dp-ok: 36%;
    --dp-ok-bg: linear-gradient(160deg, #464a51, #34373d);
    --dp-ok-shadow: 0 3px 8px #0008, inset 0 1px 0 #ffffff24;
    --rk-bg: linear-gradient(90deg, #3a3d43, #2f3237);
    --rk-ink: #e9ebef;
    --rk-w: 70px;
    --rk-h: 132px;
    --rk-r: 36px;
    --rk-shadow: inset 0 1px 0 #ffffff1f, inset 0 -2px 6px #0006;
    width: min(100%, 270px);
    margin: 0 auto;
    padding: 22px 22px 0;
    border-radius: 40px 40px 34px 34px;
    background: linear-gradient(175deg, #34363b, #232529 55%, #1d1f22);
    box-shadow: 0 14px 34px #000a, inset 0 1px 0 #ffffff22;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    flex: none;
    overflow: hidden;
  }
  .tz :global(.mic) {
    width: 46px;
    height: 46px;
    border-radius: 50%;
    background: #3a3d43;
    color: #e9ebef;
    box-shadow: inset 0 1px 0 #ffffff1f, 0 2px 4px #0006;
  }
  .row3 {
    width: 100%;
    display: flex;
    justify-content: space-between;
  }
  .tz :global(.rb) {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: linear-gradient(160deg, #3d4046, #2f3237);
    color: #e9ebef;
    box-shadow: inset 0 1px 0 #ffffff1f, 0 2px 5px #0007;
  }
  .rockers {
    display: flex;
    gap: 22px;
    justify-content: center;
  }
  .shorts {
    width: 100%;
    display: flex;
    gap: 8px;
  }
  .tz :global(.sc) {
    flex: 1;
    height: 46px;
    border-radius: 14px;
    background: #2e3035;
    color: #cfd3da;
    box-shadow: inset 0 1px 0 #ffffff14, 0 2px 4px #0006;
  }
  .tz :global(.k:active) {
    background: color-mix(in srgb, var(--accent) 55%, #2e3035);
  }
  .solar {
    align-self: stretch;
    margin: 0 -22px;
    height: 34px;
    background:
      repeating-linear-gradient(90deg, transparent 0 17px, #0a0b0d 17px 18px),
      linear-gradient(180deg, #1c2430, #0f141c);
    border-top: 1px solid #ffffff14;
    opacity: 0.95;
  }
</style>
