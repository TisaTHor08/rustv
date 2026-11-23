<script lang="ts">
  // Télécommande VIDAA (Hisense) : corps sombre, grande croix à coins arrondis, rangée Retour / Accueil / Menu / Réglages,
  // volume et chaîne, quatre touches couleur, raccourcis streaming.
  // Touches couleur (pas d'équivalent sur le PC) : rouge = stop, vert = lecture/pause, jaune = précédent, bleu = suivant.
  import { app } from '../connection.svelte.ts';
  import { home, key, media, menu } from './actions.ts';
  import { labels } from './labels.ts';
  import type { RemoteCtx } from './types.ts';
  import Btn from './parts/Btn.svelte';
  import Dpad from './parts/Dpad.svelte';
  import Icon from './parts/Icon.svelte';
  import Rocker from './parts/Rocker.svelte';
  import Shortcut from './parts/Shortcut.svelte';

  let { ctx }: { ctx: RemoteCtx } = $props();
  const L = labels('vidaa');
  const names = { up: L('up'), down: L('down'), left: L('left'), right: L('right'), ok: L('ok') };
  const COLORS = [
    { c: '#e53935', k: 'red', a: media('stop') },
    { c: '#43a047', k: 'green', a: media('playPause') },
    { c: '#fdd835', k: 'yellow', a: media('prev') },
    { c: '#1e88e5', k: 'blue', a: media('next') },
  ];
</script>

<div class="vd">
  <Dpad onok={ctx.ok} {names} />

  <div class="row4">
    <Btn cls="rb" kind="long" label={L('back')} onTap={key('back')} onLong={key('backlong')}><Icon name="back" size={26} sw={2.4} /></Btn>
    <Btn cls="rb" strong label={L('home')} onTap={home}><Icon name="home" size={24} /></Btn>
    <Btn cls="rb" label={L('menu')} onTap={menu}><Icon name="menu" size={24} /></Btn>
    <Btn cls="rb" label={L('settings')} onTap={menu}><Icon name="gear" size={24} /></Btn>
  </div>

  <div class="rk2">
    <Rocker onUp={media('volUp')} onDown={media('volDown')} upLabel={L('volUp')} downLabel={L('volDown')} mid={app.volume.known ? String(app.volume.level) : 'VOL'} />
    <Btn cls="rb" label={L('mute')} onTap={media('mute')}><Icon name={app.volume.muted ? 'mute' : 'vol'} size={22} /></Btn>
    <Rocker onUp={media('next')} onDown={media('prev')} upLabel={L('chUp')} downLabel={L('chDown')} mid="CH" />
  </div>

  <div class="dots">
    {#each COLORS as d}
      <Btn cls="dot" label={L(d.k)} onTap={d.a}><i style:background={d.c}></i></Btn>
    {/each}
  </div>

  <div class="shorts">
    <Shortcut cls="sc" id="netflix" n={1} label={L('shortcut', 1)} />
    <Shortcut cls="sc" id="youtube" n={2} label={L('shortcut', 2)} />
    <Shortcut cls="sc" id="prime" n={3} label={L('shortcut', 3)} />
    <Shortcut cls="sc" id="disney" n={4} label={L('shortcut', 4)} />
  </div>
</div>

<style>
  .vd {
    --dp-bg: linear-gradient(160deg, #2e3035, #1f2124);
    --dp-r: 24%;
    --dp-shadow: inset 0 1px 0 #ffffff1c, inset 0 -3px 8px #0008, 0 0 0 5px #15161a;
    --dp-ico: #eef0f3;
    --dp-ok: 36%;
    --dp-ok-bg: linear-gradient(160deg, #3d4046, #2a2c31);
    --dp-ok-shadow: 0 3px 8px #000a, inset 0 1px 0 #ffffff22;
    --dp-cut: #00000066;
    --dp-size: min(100%, 220px);
    --rk-bg: linear-gradient(90deg, #30333a, #24262b);
    --rk-ink: #eef0f3;
    --rk-w: 62px;
    --rk-h: 112px;
    --rk-r: 14px;
    --rk-shadow: inset 0 1px 0 #ffffff1c, inset 0 -2px 6px #0008;
    width: min(100%, 290px);
    margin: 0 auto;
    padding: 24px 22px 20px;
    border-radius: 30px;
    background: linear-gradient(175deg, #232529, #17181b 60%, #121316);
    box-shadow: 0 14px 34px #000b, inset 0 1px 0 #ffffff1c;
    display: flex;
    flex-direction: column;
    gap: 16px;
    flex: none;
  }
  .row4,
  .rk2 {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .vd :global(.rb) {
    width: 50px;
    height: 50px;
    border-radius: 50%;
    background: linear-gradient(160deg, #35383e, #26282c);
    color: #eef0f3;
    box-shadow: inset 0 1px 0 #ffffff1c, 0 2px 5px #0008;
  }
  .dots {
    display: flex;
    justify-content: space-between;
    padding: 0 6px;
  }
  .vd :global(.dot) {
    background: transparent;
    width: 56px;
  }
  .dots i {
    display: block;
    width: 38px;
    height: 12px;
    border-radius: 99px;
    box-shadow: 0 0 6px #0008, inset 0 1px 0 #ffffff55;
  }
  .shorts {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 7px;
  }
  .vd :global(.sc) {
    height: 46px;
    border-radius: 12px;
    background: #2a2c31;
    color: #cfd3da;
    box-shadow: inset 0 1px 0 #ffffff14, 0 2px 4px #0008;
    min-width: 0;
  }
  .vd :global(.k:active) {
    background: color-mix(in srgb, var(--accent) 55%, #2a2c31);
  }
</style>
