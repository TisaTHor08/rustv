<script lang="ts">
  // Télécommande Titan OS (Philips) : corps anthracite brossé, croix + OK, Retour / Accueil / Options, gros bouton Ambilight,
  // volume et chaîne, touches couleur, raccourcis streaming.
  // Ambilight : sans effet sur le PC (l'ambilight Hyperion s'active côté PC), le bouton affiche seulement une bulle d'explication.
  import { app } from '../connection.svelte.ts';
  import { home, key, media, menu } from './actions.ts';
  import { AMBILIGHT_TOAST, labels, toastText } from './labels.ts';
  import type { RemoteCtx } from './types.ts';
  import Btn from './parts/Btn.svelte';
  import Dpad from './parts/Dpad.svelte';
  import Icon from './parts/Icon.svelte';
  import Rocker from './parts/Rocker.svelte';
  import Shortcut from './parts/Shortcut.svelte';

  let { ctx }: { ctx: RemoteCtx } = $props();
  const L = labels('titan');
  const names = { up: L('up'), down: L('down'), left: L('left'), right: L('right'), ok: L('ok') };
  // Touches couleur : rouge = stop, vert = lecture/pause, jaune = précédent, bleu = suivant.
  const COLORS = [
    { c: '#e53935', k: 'red', a: media('stop') },
    { c: '#43a047', k: 'green', a: media('playPause') },
    { c: '#fdd835', k: 'yellow', a: media('prev') },
    { c: '#1e88e5', k: 'blue', a: media('next') },
  ];
</script>

<div class="tt">
  <Dpad onok={ctx.ok} {names} />

  <div class="row3">
    <Btn cls="rb" kind="long" label={L('back')} onTap={key('back')} onLong={key('backlong')}><Icon name="back" size={26} sw={2.4} /></Btn>
    <Btn cls="rb" strong label={L('home')} onTap={home}><Icon name="home" size={24} /></Btn>
    <Btn cls="rb" label={L('options')} onTap={menu}><Icon name="menu" size={24} /></Btn>
  </div>

  <Btn cls="amb" label={L('ambilight')} onTap={() => ctx.toast(toastText(AMBILIGHT_TOAST))}>
    <span class="ambi"><Icon name="ambilight" size={22} /><b>Ambilight</b></span>
  </Btn>

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
  </div>
</div>

<style>
  .tt {
    --dp-bg: linear-gradient(160deg, #3a3d42, #26282b);
    --dp-r: 40%;
    --dp-shadow: inset 0 1px 0 #ffffff2a, inset 0 -3px 8px #0008, 0 0 0 2px #55595f;
    --dp-ico: #eef0f3;
    --dp-ok: 36%;
    --dp-ok-bg: linear-gradient(160deg, #4a4e55, #33363b);
    --dp-ok-shadow: 0 3px 8px #000a, inset 0 1px 0 #ffffff2a;
    --dp-cut: #0006;
    --dp-size: min(100%, 230px);
    --rk-bg: linear-gradient(90deg, #3a3d42, #2b2d31);
    --rk-ink: #eef0f3;
    --rk-w: 62px;
    --rk-h: 112px;
    --rk-r: 30px;
    --rk-shadow: inset 0 1px 0 #ffffff22, inset 0 -2px 6px #0008;
    width: min(100%, 280px);
    margin: 0 auto;
    padding: 24px 22px 20px;
    border-radius: 34px;
    background:
      repeating-linear-gradient(90deg, #ffffff08 0 1px, transparent 1px 3px),
      linear-gradient(175deg, #34373c, #212326 60%, #1a1b1e);
    box-shadow: 0 14px 34px #000b, inset 0 1px 0 #ffffff26;
    display: flex;
    flex-direction: column;
    gap: 16px;
    flex: none;
  }
  .row3,
  .rk2 {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .tt :global(.rb) {
    width: 54px;
    height: 54px;
    border-radius: 50%;
    background: linear-gradient(160deg, #42454b, #2d3034);
    color: #eef0f3;
    box-shadow: inset 0 1px 0 #ffffff22, 0 2px 5px #0008;
  }
  .tt :global(.amb) {
    width: 100%;
    height: 52px;
    border-radius: 26px;
    color: #fff;
    background: linear-gradient(#26282c, #26282c) padding-box, linear-gradient(90deg, #ff2d95, #ffb400, #25e0a0, #2d7dff, #ff2d95) border-box;
    border: 2px solid transparent;
    box-shadow: 0 0 14px #ff2d9544, 0 0 14px #2d7dff44;
  }
  .ambi {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.95rem;
    letter-spacing: 0.03em;
  }
  .dots {
    display: flex;
    justify-content: space-between;
    padding: 0 6px;
  }
  .tt :global(.dot) {
    background: transparent;
    width: 56px;
  }
  .dots i {
    display: block;
    width: 34px;
    height: 10px;
    border-radius: 99px;
    box-shadow: 0 0 6px #0008, inset 0 1px 0 #ffffff55;
  }
  .shorts {
    display: flex;
    gap: 8px;
  }
  .tt :global(.sc) {
    flex: 1;
    height: 46px;
    border-radius: 14px;
    background: #2c2e32;
    color: #d4d8de;
    box-shadow: inset 0 1px 0 #ffffff1a, 0 2px 4px #0008;
  }
  .tt :global(.k:active) {
    background: color-mix(in srgb, var(--accent) 55%, #2c2e32);
  }
</style>
