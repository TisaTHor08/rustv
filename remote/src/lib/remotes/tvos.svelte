<script lang="ts">
  // Siri Remote d'Apple TV : cadre aluminium, grand pavé rond (glisser = flèches, tap = OK, clic sur le bord = flèche),
  // Retour et TV (accueil), muet et lecture / pause, bascule de volume horizontale dessous, bouton Siri (micro) en haut à droite du pavé
  // (sur l'appareil il est sur le bord, ici une petite pastille).
  import { app } from '../connection.svelte.ts';
  import { home, key, media, search } from './actions.ts';
  import { labels } from './labels.ts';
  import type { RemoteCtx } from './types.ts';
  import Surface from '../Surface.svelte';
  import Btn from './parts/Btn.svelte';
  import Icon from './parts/Icon.svelte';
  import Rocker from './parts/Rocker.svelte';

  // Aucun besoin du contexte : le pavé tactile gère lui-même le OK.
  let {}: { ctx: RemoteCtx } = $props();
  const L = labels('tvos');
</script>

<div class="tv">
  <div class="face">
    <Surface variant="clickpad" />

    <div class="top">
      <Btn cls="rb sm" label={L('assistant')} onTap={search}><Icon name="mic" size={18} /></Btn>
    </div>
    <div class="grid">
      <Btn cls="rb" kind="long" label={L('back')} onTap={key('back')} onLong={key('backlong')}><Icon name="back" size={26} sw={2.4} /></Btn>
      <Btn cls="rb" strong label={L('home')} onTap={home}><Icon name="tv" size={24} /></Btn>
      <Btn cls="rb" label={L('mute')} onTap={media('mute')}><Icon name={app.volume.muted ? 'mute' : 'vol'} size={22} /></Btn>
      <Btn cls="rb" label={L('play')} onTap={media('playPause')}><Icon name="playpause" size={22} /></Btn>
    </div>
    <Rocker horizontal onUp={media('volUp')} onDown={media('volDown')} upLabel={L('volUp')} downLabel={L('volDown')} mid={app.volume.known ? String(app.volume.level) : 'VOL'} />
  </div>
</div>

<style>
  .tv {
    --cp-size: 100%;
    --cp-bg: radial-gradient(circle at 50% 28%, #2b2b2f, #141416 72%);
    --cp-shadow: inset 0 0 0 1px #ffffff12, inset 0 3px 10px #0009, 0 0 0 5px #34353a, 0 0 0 6px #0b0b0c;
    --cp-ink: #55555c;
    --rk-bg: linear-gradient(90deg, #2c2c30, #1b1b1e);
    --rk-ink: #f1f1f3;
    --rk-w: 100%;
    --rk-h: 54px;
    --rk-r: 27px;
    --rk-shadow: inset 0 1px 0 #ffffff1a, 0 2px 4px #000a;
    width: min(100%, 270px);
    margin: 0 auto;
    padding: 5px;
    border-radius: 40px;
    background: linear-gradient(120deg, #e9ebef, #b6bac2 35%, #dfe2e7 55%, #a7abb3);
    box-shadow: 0 14px 34px #000b, inset 0 1px 0 #fff;
    flex: none;
  }
  .face {
    border-radius: 36px;
    padding: 20px 18px 20px;
    background: linear-gradient(175deg, #1f1f22, #121214);
    box-shadow: inset 0 1px 0 #ffffff14;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .face :global(.surface) {
    --cp-size: 100%;
  }
  .top {
    display: flex;
    justify-content: flex-end;
    margin-bottom: -6px;
  }
  .tv :global(.rb.sm) {
    width: 34px;
    height: 34px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    justify-items: center;
  }
  .tv :global(.rb) {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: linear-gradient(160deg, #2e2e32, #1d1d20);
    color: #f1f1f3;
    box-shadow: inset 0 1px 0 #ffffff1a, 0 2px 4px #000a;
  }
  .tv :global(.k:active) {
    background: color-mix(in srgb, var(--accent) 50%, #1d1d20);
  }
</style>
