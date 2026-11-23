<script lang="ts">
  // Télécommande vocale Fire TV (et Vega OS) : corps sombre fin, anneau + OK, Retour | Accueil puis Menu | micro,
  // rangée rembobiner / lecture / avance, volume et muet, raccourcis streaming.
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
  const L = labels('fire');
  const names = { up: L('up'), down: L('down'), left: L('left'), right: L('right'), ok: L('ok') };
</script>

<div class="fr">
  <Dpad onok={ctx.ok} ok="" {names} />

  <div class="grid">
    <Btn cls="rb" kind="long" label={L('back')} onTap={key('back')} onLong={key('backlong')}><Icon name="backArrow" size={24} /></Btn>
    <Btn cls="rb" strong label={L('home')} onTap={home}><Icon name="home" size={24} /></Btn>
    <Btn cls="rb" label={L('menu')} onTap={menu}><Icon name="menu" size={24} /></Btn>
    <Btn cls="rb" label={L('assistant')} onTap={search}><Icon name="mic" size={24} /></Btn>
  </div>

  <div class="trio">
    <Btn cls="tr" label={L('rew')} onTap={media('prev')}><Icon name="rew" size={22} /></Btn>
    <Btn cls="tr" label={L('play')} onTap={media('playPause')}><Icon name="playpause" size={22} /></Btn>
    <Btn cls="tr" label={L('ff')} onTap={media('next')}><Icon name="ff" size={22} /></Btn>
  </div>

  <div class="vol">
    <Rocker horizontal onUp={media('volUp')} onDown={media('volDown')} upLabel={L('volUp')} downLabel={L('volDown')} mid={app.volume.known ? String(app.volume.level) : 'VOL'} />
    <Btn cls="rb" label={L('mute')} onTap={media('mute')}><Icon name={app.volume.muted ? 'mute' : 'vol'} size={22} /></Btn>
  </div>

  <div class="shorts">
    <Shortcut cls="sc" id="prime" n={1} label={L('shortcut', 1)} />
    <Shortcut cls="sc" id="netflix" n={2} label={L('shortcut', 2)} />
    <Shortcut cls="sc" id="disney" n={3} label={L('shortcut', 3)} />
    <Shortcut cls="sc" id="spotify" n={4} label={L('shortcut', 4)} />
  </div>
</div>

<style>
  .fr {
    --dp-bg: linear-gradient(165deg, #34383e, #25282c);
    --dp-shadow: inset 0 1px 0 #ffffff1c, inset 0 -3px 8px #0008;
    --dp-ico: #eceef1;
    --dp-ok: 38%;
    --dp-ok-bg: linear-gradient(165deg, #2c2f34, #1f2124);
    --dp-ok-shadow: inset 0 2px 6px #000b, 0 1px 0 #ffffff1c;
    --dp-cut: transparent;
    --rk-bg: #2d3035;
    --rk-ink: #eceef1;
    --rk-w: 150px;
    --rk-h: 50px;
    --rk-shadow: inset 0 1px 0 #ffffff1a, 0 2px 4px #0008;
    width: min(100%, 270px);
    margin: 0 auto;
    padding: 22px 22px 22px;
    border-radius: 36px;
    background: linear-gradient(175deg, #2a2d32, #1b1d20 60%, #151618);
    box-shadow: 0 14px 34px #000b, inset 0 1px 0 #ffffff1c;
    display: flex;
    flex-direction: column;
    gap: 14px;
    flex: none;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    justify-items: center;
    gap: 12px;
  }
  .fr :global(.rb) {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: #2d3035;
    color: #eceef1;
    box-shadow: inset 0 1px 0 #ffffff1a, 0 2px 4px #0008;
  }
  .trio {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .fr :global(.tr) {
    height: 46px;
    border-radius: 23px;
    background: #2d3035;
    color: #eceef1;
    box-shadow: inset 0 1px 0 #ffffff1a, 0 2px 4px #0008;
  }
  .vol {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
  }
  .vol :global(.rk) {
    flex: 1;
    width: auto;
    min-height: 50px;
  }
  .vol :global(.rb) {
    width: 50px;
    height: 50px;
    flex: none;
  }
  .shorts {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 7px;
  }
  .fr :global(.sc) {
    height: 46px;
    border-radius: 12px;
    background: #2d3035;
    color: #cfd3da;
    box-shadow: inset 0 1px 0 #ffffff14, 0 2px 4px #0008;
    min-width: 0;
  }
  .fr :global(.k:active) {
    background: color-mix(in srgb, var(--accent) 55%, #2d3035);
  }
</style>
