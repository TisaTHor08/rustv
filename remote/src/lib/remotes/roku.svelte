<script lang="ts">
  // Télécommande Roku (vocale) : corps noir à reflets violets, anneau + OK, Retour / Accueil, Relecture / Options (*),
  // rembobiner / lecture / avance, volume sur le côté, quatre raccourcis streaming larges.
  // Relecture : pas de message « saut en arrière » dans le protocole, elle envoie donc « précédent » (média).
  import { app } from '../connection.svelte.ts';
  import { home, key, media, menu, search } from './actions.ts';
  import { labels } from './labels.ts';
  import type { RemoteCtx } from './types.ts';
  import Btn from './parts/Btn.svelte';
  import Dpad from './parts/Dpad.svelte';
  import Icon from './parts/Icon.svelte';
  import Rocker from './parts/Rocker.svelte';
  import Shortcut from './parts/Shortcut.svelte';

  let { ctx }: { ctx: RemoteCtx } = $props();
  const L = labels('roku');
  const names = { up: L('up'), down: L('down'), left: L('left'), right: L('right'), ok: L('ok') };
</script>

<div class="rk">
  <div class="top">
    <Btn cls="rb" label={L('assistant')} onTap={search}><Icon name="mic" size={22} /></Btn>
  </div>

  <div class="mid">
    <div class="main">
      <Dpad onok={ctx.ok} {names} />
      <div class="grid">
        <Btn cls="sq" kind="long" label={L('back')} onTap={key('back')} onLong={key('backlong')}><Icon name="backArrow" size={24} /></Btn>
        <Btn cls="sq" strong label={L('home')} onTap={home}><Icon name="home" size={24} /></Btn>
        <Btn cls="sq" label={L('replay')} onTap={media('prev')}><Icon name="replay" size={24} /></Btn>
        <Btn cls="sq" label={L('options')} onTap={menu}><Icon name="star" size={22} /></Btn>
      </div>
    </div>
    <Rocker onUp={media('volUp')} onDown={media('volDown')} upLabel={L('volUp')} downLabel={L('volDown')} mid={app.volume.known ? String(app.volume.level) : 'VOL'} />
  </div>

  <div class="trio">
    <Btn cls="sq" label={L('rew')} onTap={media('prev')}><Icon name="rew" size={22} /></Btn>
    <Btn cls="sq" label={L('play')} onTap={media('playPause')}><Icon name="playpause" size={22} /></Btn>
    <Btn cls="sq" label={L('ff')} onTap={media('next')}><Icon name="ff" size={22} /></Btn>
  </div>

  <div class="shorts">
    <Shortcut cls="sc" id="netflix" n={1} label={L('shortcut', 1)} />
    <Shortcut cls="sc" id="disney" n={2} label={L('shortcut', 2)} />
    <Shortcut cls="sc" id="prime" n={3} label={L('shortcut', 3)} />
    <Shortcut cls="sc" id="max" n={4} label={L('shortcut', 4)} />
  </div>
</div>

<style>
  .rk {
    --dp-bg: linear-gradient(160deg, #2a2330, #17131b);
    --dp-shadow: inset 0 1px 0 #ffffff1c, inset 0 -3px 8px #000a, 0 0 0 3px #6c1d9c;
    --dp-ico: #f2eaf8;
    --dp-ok: 38%;
    --dp-ok-bg: linear-gradient(160deg, #7a24b2, #561a7e);
    --dp-ok-fg: #fff;
    --dp-ok-shadow: 0 3px 8px #000a, inset 0 1px 0 #ffffff30;
    --dp-cut: #00000055;
    --dp-size: 100%;
    --rk-bg: linear-gradient(90deg, #2a2330, #1b1620);
    --rk-ink: #f2eaf8;
    --rk-w: 52px;
    --rk-h: 150px;
    --rk-r: 26px;
    --rk-shadow: inset 0 1px 0 #ffffff1a, inset 0 -2px 6px #000a;
    width: min(100%, 290px);
    margin: 0 auto;
    padding: 20px 20px 22px;
    border-radius: 40px;
    background: linear-gradient(175deg, #231d2a, #131016 60%, #0e0c11);
    box-shadow: 0 14px 34px #000b, inset 0 1px 0 #ffffff1c, 0 0 0 2px #3c1058;
    display: flex;
    flex-direction: column;
    gap: 14px;
    flex: none;
  }
  .top {
    display: flex;
    justify-content: center;
  }
  .rk :global(.rb) {
    width: 46px;
    height: 46px;
    border-radius: 50%;
    background: #2a2330;
    color: #f2eaf8;
    box-shadow: inset 0 1px 0 #ffffff1c, 0 2px 4px #000a;
  }
  .mid {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .rk :global(.sq) {
    height: 48px;
    width: 100%;
    border-radius: 14px;
    background: #2a2330;
    color: #f2eaf8;
    box-shadow: inset 0 1px 0 #ffffff1c, 0 2px 4px #000a;
  }
  .trio {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }
  .shorts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 9px;
  }
  .rk :global(.sc) {
    height: 48px;
    width: 100%;
    border-radius: 12px;
    background: linear-gradient(160deg, #5a1d82, #3d1259);
    color: #f2eaf8;
    box-shadow: inset 0 1px 0 #ffffff26, 0 2px 4px #000a;
  }
  .rk :global(.k:active) {
    background: color-mix(in srgb, var(--accent) 55%, #2a2330);
  }
</style>
