<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import { t } from '../../i18n';
  import ContinueRow from '../../ContinueRow.svelte';
  import ModWidgets from '../../ModWidgets.svelte';
  import { iconSrc } from '../../tiles.svelte';
  import Wallpaper from '../../Wallpaper.svelte';
  import Weather from '../../Weather.svelte';
  import type { DTile } from '../../tiles.svelte';
  import type { HomeModel } from '../model.svelte';

  // Style RusTV : l'accueil d'origine (héros + rangées), déplacé tel quel depuis Home.svelte.
  let { model }: { model: HomeModel } = $props();

  const TILE_STEP = 18.6; // rem : largeur d'une tuile + espacement
  const ROW_H = 17; // rem
  const fg = (x?: DTile) => (x?.dark_text ? '#16130a' : '#ffffff');
  const tinted = $derived(model.tinted);
</script>

<div class="lay" style:--c1={tinted ? model.focused!.color : 'transparent'} style:--c2={tinted ? model.focused!.color2 : 'transparent'}>
  <Wallpaper />
  <div class="tint"></div>
  <div class="veil"></div>

  <!-- Barre du haut -->
  <header>
    <div class="brand">
      <svg viewBox="0 0 106 106" aria-hidden="true">
        <path fill="#FF5A3C" d="M 52.896484,0.50195312 A 1.50015,1.50015 0 0 0 51.712891,1.2285156 l -2.660157,4.4335938 c -0.474118,0.039238 -0.945895,0.085535 -1.416015,0.1386718 h -0.002 l -3.472657,-3.828125 a 1.50015,1.50015 0 0 0 -2.52539,0.5019532 L 39.892578,7.3398438 C 39.436217,7.471171 38.981915,7.6112907 38.53125,7.7558594 l -4.154297,-3.078125 a 1.50015,1.50015 0 0 0 -2.378906,0.984375 L 31.236328,10.791016 C 30.820966,11.006176 30.408138,11.228124 30,11.455078 H 29.998 L 25.306641,9.2382812 a 1.50015,1.50015 0 0 0 -2.138672,1.4296878 l 0.251953,5.199219 c -0.360134,0.287525 -0.714891,0.579304 -1.066406,0.876953 l -5.050782,-1.261719 a 1.50015,1.50015 0 0 0 -1.820312,1.820312 l 1.261719,5.050782 c -0.297649,0.351515 -0.589428,0.706272 -0.876953,1.066406 L 10.667969,23.167969 A 1.50015,1.50015 0 0 0 9.2382812,25.306641 L 11.455078,30 c -0.226954,0.408138 -0.448902,0.820966 -0.664062,1.236328 l -5.1289066,0.761719 a 1.50015,1.50015 0 0 0 -0.984375,2.378906 l 3.078125,4.154297 c -0.1445689,0.450666 -0.2846882,0.904966 -0.4160156,1.361328 l -4.8652344,1.744141 a 1.50015,1.50015 0 0 0 -0.5019532,2.52539 l 3.828125,3.472657 c -0.053229,0.470758 -0.09938,0.943201 -0.1386718,1.417968 l -4.4335938,2.660157 a 1.50015,1.50015 0 0 0 0,2.574218 l 4.4335938,2.660157 c 0.039238,0.474118 0.085535,0.947848 0.1386718,1.417968 l -3.828125,3.472657 a 1.50015,1.50015 0 0 0 0.5019532,2.52539 l 4.8652344,1.744141 c 0.1313273,0.456361 0.2714467,0.910662 0.4160156,1.361328 l -3.078125,4.154297 a 1.50015,1.50015 0 0 0 0.984375,2.378906 l 5.1289066,0.761719 c 0.21516,0.415362 0.437108,0.830143 0.664062,1.238281 l -2.2167968,4.691406 a 1.50015,1.50015 0 0 0 1.4296878,2.138672 l 5.199219,-0.251953 c 0.287525,0.360134 0.579304,0.714891 0.876953,1.066406 l -1.261719,5.050782 a 1.50015,1.50015 0 0 0 1.820312,1.820312 l 5.050782,-1.261719 c 0.351515,0.297649 0.706272,0.589428 1.066406,0.876953 l -0.251953,5.199219 a 1.50015,1.50015 0 0 0 2.138672,1.429688 L 30,94.544922 c 0.408138,0.226954 0.820966,0.448902 1.236328,0.664062 l 0.761719,5.128906 a 1.50015,1.50015 0 0 0 2.378906,0.98438 l 4.154297,-3.078129 c 0.450665,0.144568 0.904967,0.284688 1.361328,0.416015 l 1.744141,4.865234 a 1.50015,1.50015 0 0 0 2.52539,0.50195 l 3.472657,-3.82812 c 0.470758,0.0532 0.943201,0.0994 1.417968,0.13867 l 2.660157,4.43359 a 1.50015,1.50015 0 0 0 2.574218,0 l 2.660157,-4.43359 c 0.474119,-0.0392 0.945894,-0.0855 1.416015,-0.13867 h 0.002 l 3.472657,3.82812 a 1.50015,1.50015 0 0 0 2.52539,-0.50195 l 1.744141,-4.865234 c 0.456361,-0.131327 0.910662,-0.271447 1.361328,-0.416015 l 4.154297,3.078129 A 1.50015,1.50015 0 0 0 74.002,100.33789 l 0.761719,-5.128906 C 75.179034,94.993824 75.591862,94.771876 76,94.544922 h 0.002 l 4.691406,2.216797 a 1.50015,1.50015 0 0 0 2.138672,-1.429688 l -0.251953,-5.199219 c 0.360134,-0.287525 0.714891,-0.579304 1.066406,-0.876953 l 5.050782,1.261719 a 1.50015,1.50015 0 0 0 1.820312,-1.820312 l -1.261719,-5.050782 c 0.297649,-0.351515 0.589428,-0.706272 0.876953,-1.066406 l 5.199219,0.251953 a 1.50015,1.50015 0 0 0 1.429688,-2.138672 l -2.216797,-4.691406 c 0.227248,-0.408632 0.448637,-0.822407 0.664062,-1.238281 l 5.128909,-0.761719 a 1.50015,1.50015 0 0 0 0.98438,-2.378906 L 98.244141,67.46875 c 0.144568,-0.450666 0.284688,-0.904967 0.416015,-1.361328 l 4.865234,-1.744141 a 1.50015,1.50015 0 0 0 0.50195,-2.52539 l -3.82812,-3.47461 c 0.0531,-0.47012 0.0994,-0.941897 0.13867,-1.416015 l 4.43359,-2.660157 a 1.50015,1.50015 0 0 0 0,-2.574218 l -4.43359,-2.660157 c -0.0392,-0.474119 -0.0855,-0.947847 -0.13867,-1.417968 l 3.82812,-3.472657 a 1.50015,1.50015 0 0 0 -0.50195,-2.52539 L 98.660156,39.892578 C 98.528829,39.436216 98.38871,38.981916 98.244141,38.53125 l 3.078129,-4.154297 a 1.50015,1.50015 0 0 0 -0.98438,-2.378906 l -5.128906,-0.761719 c -0.21516,-0.415361 -0.437109,-0.830144 -0.664062,-1.238281 l 2.216797,-4.691406 a 1.50015,1.50015 0 0 0 -1.429688,-2.138672 l -5.199219,0.251953 c -0.287525,-0.360134 -0.579304,-0.714891 -0.876953,-1.066406 l 1.261719,-5.050782 a 1.50015,1.50015 0 0 0 -1.820312,-1.820312 l -5.050782,1.261719 c -0.351515,-0.297649 -0.706272,-0.589428 -1.066406,-0.876953 l 0.251953,-5.199219 A 1.50015,1.50015 0 0 0 80.693359,9.2382812 l -4.691406,2.2167968 c -0.40863,-0.227246 -0.82241,-0.448638 -1.238281,-0.664062 L 74.001953,5.6621094 A 1.50015,1.50015 0 0 0 71.623047,4.6777344 L 67.46875,7.7558594 C 67.018084,7.6112905 66.563783,7.4711712 66.107422,7.3398438 L 64.363281,2.4746094 a 1.50015,1.50015 0 0 0 -2.52539,-0.5019532 l -3.47461,3.828125 C 57.893161,5.7476439 57.421384,5.7013471 56.947266,5.6621094 L 54.287109,1.2285156 A 1.50015,1.50015 0 0 0 52.896484,0.50195312 Z M 53,10 a 3,3 0 0 1 3,3 3,3 0 0 1 -3,3 3,3 0 0 1 -3,-3 3,3 0 0 1 3,-3 z m -7.09375,5.148438 4.972656,4.972656 a 3.0003,3.0003 0 0 0 4.242188,0 l 4.972656,-4.972656 c 11.560278,2.139797 21.282716,9.41638 26.722656,19.388671 l -3.203125,6.285157 a 3.0003,3.0003 0 0 0 1.3125,4.035156 L 91.1875,48.048828 C 91.394076,49.669155 91.5,51.322097 91.5,53 c 0,10.26189 -3.983419,19.566752 -10.492188,26.457031 l -6.966796,-1.103515 a 3.0003,3.0003 0 0 0 -3.431641,2.492187 L 69.505859,87.8125 C 64.507396,90.178425 58.913018,91.5 53,91.5 47.086982,91.5 41.492604,90.178425 36.494141,87.8125 l -1.103516,-6.966797 a 3.0003,3.0003 0 0 0 -3.431641,-2.492187 l -6.966796,1.103515 C 18.483419,72.566752 14.5,63.26189 14.5,53 c 0,-1.677903 0.105924,-3.332798 0.3125,-4.953125 l 6.261719,-3.189453 a 3.0003,3.0003 0 0 0 1.3125,-4.035156 l -3.203125,-6.285157 c 5.43994,-9.97229 15.162378,-17.248874 26.722656,-19.388671 z m -30.791016,22.49414 a 3,3 0 0 1 0.769532,0.144531 3,3 0 0 1 1.925781,3.779297 3,3 0 0 1 -3.779297,1.925782 3,3 0 0 1 -1.925781,-3.779297 3,3 0 0 1 3.009765,-2.070313 z m 75.769532,0 a 3,3 0 0 1 3.009765,2.070313 3,3 0 0 1 -1.925781,3.779297 3,3 0 0 1 -3.779297,-1.925782 3,3 0 0 1 1.925781,-3.779297 3,3 0 0 1 0.769532,-0.144531 z m -61.47461,44.71875 a 3,3 0 0 1 1.841797,0.572266 3,3 0 0 1 0.664063,4.191406 3,3 0 0 1 -4.191407,0.662109 3,3 0 0 1 -0.662109,-4.189453 3,3 0 0 1 2.347656,-1.236328 z m 47.179688,0 a 3,3 0 0 1 2.347656,1.236328 3,3 0 0 1 -0.662109,4.189453 3,3 0 0 1 -4.191407,-0.662109 3,3 0 0 1 0.664063,-4.191406 3,3 0 0 1 1.841797,-0.572266 z"/>
  <path d="M 38 29 L 38 77 L 78 53 Z" fill="#FF5A3C" stroke="#FF5A3C" stroke-width="6" stroke-linejoin="round"/>
      </svg>
      <span>RusTV</span>
    </div>
    <div class="right">
      {#if model.widgetsNative}<ModWidgets toast={model.note} at="status" provided={['status']} />{/if}
      {#if model.widgetsNative}<Weather />{/if}
      {#if model.widgetsNative}
      <div class="pill" class:on={model.phones > 0}>
        <span class="dot"></span>{model.phonesLabel}
      </div>
      <div class="clock"><strong>{model.time}</strong><small>{model.date}</small></div>
      {/if}
      <button class="icon" aria-label={t('search.title')} onclick={() => model.openSearch()}>
        <svg viewBox="0 0 24 24"><path d="M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z" /></svg>
      </button>
      <button class="icon" class:focus={model.focus.row === -1} onclick={() => model.openSettings()} onmouseenter={() => model.focusAt(-1, 0, true)} aria-label={t('settings.title')}>
        <svg viewBox="0 0 24 24"><path d="M19.4 13a7.7 7.7 0 0 0 0-2l2-1.6a.5.5 0 0 0 .1-.6l-1.9-3.3a.5.5 0 0 0-.6-.2l-2.4 1a7.2 7.2 0 0 0-1.7-1l-.4-2.5a.5.5 0 0 0-.5-.4h-3.8a.5.5 0 0 0-.5.4L9.3 5.3a7.2 7.2 0 0 0-1.7 1l-2.4-1a.5.5 0 0 0-.6.2L2.7 8.8a.5.5 0 0 0 .1.6l2 1.6a7.7 7.7 0 0 0 0 2l-2 1.6a.5.5 0 0 0-.1.6l1.9 3.3a.5.5 0 0 0 .6.2l2.4-1c.5.4 1.1.7 1.7 1l.4 2.5c0 .2.2.4.5.4h3.8c.3 0 .5-.2.5-.4l.4-2.5c.6-.3 1.2-.6 1.7-1l2.4 1c.2.1.5 0 .6-.2l1.9-3.3a.5.5 0 0 0-.1-.6ZM12 15.5A3.5 3.5 0 1 1 12 8.5a3.5 3.5 0 0 1 0 7Z" /></svg>
      </button>
    </div>
  </header>

  <!-- Héro -->
  <section class="hero">
    {#key model.focused?.id}
      <div class="hero-in" in:fly={{ y: 24, duration: 520, easing: cubicOut }}>
        {#if model.focused}
          <span class="eyebrow">{t(`cat.${model.focused.category}`)}</span>
          <h1>{model.focused.name}</h1>
          <p>{model.focused.desc}</p>
          <div class="cta"><span class="play">▶</span> {t('hero.open')} <kbd>OK</kbd></div>
        {:else}
          <span class="eyebrow">RusTV</span>
          <h1>{t('settings.title')}</h1>
          <p>{t('settings.appearance')} · {t('settings.wallpaper')} · {t('settings.tiles')}</p>
        {/if}
      </div>
    {/key}
  </section>

  <!-- Rangées -->
  <main style:--shift="{-model.focus.row * ROW_H}rem">
    <div class="rows">
      {#each model.rows as r, ri (r.key)}
        <div class="row" class:active={ri === model.focus.row}>
          <h2>{r.title}</h2>
          <div class="track" style:--tx="{-Math.max(0, (ri === model.focus.row ? model.focus.col : 0) - 2) * TILE_STEP}rem">
            {#if r.key === 'continue'}
              <ContinueRow items={model.continueEntries} focus={ri === model.focus.row ? model.focus.col : -1} tilt={model.tilt} onopen={(i) => model.resume(model.continueEntries[i].url)} onfocus={(i) => model.focusAt(ri, i, true)} />
            {:else}
            {#each r.tiles as tile, ci (tile.id)}
              <button
                class="tile"
                class:focus={ri === model.focus.row && ci === model.focus.col}
                data-tilt={ri === model.focus.row && ci === model.focus.col ? model.tilt : ''}
                style:--a={tile.color}
                style:--b={tile.color2}
                style:--fg={fg(tile)}
                onmouseenter={() => model.focusAt(ri, ci, true)}
                onclick={() => model.open(tile)}
                oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}
              >
                {#if tile.icon}
                  <img class="ico" src={iconSrc(tile.icon)} alt="" />
                {:else}
                  <span class="mark">{tile.name[0]}</span>
                {/if}
                <span class="shine"></span>
                <span class="label">{tile.name}</span>
              </button>
            {/each}
            {/if}
          </div>
        </div>
      {/each}
    </div>
  </main>
</div>
<style>
  @property --c1 { syntax: '<color>'; inherits: true; initial-value: transparent; }
  @property --c2 { syntax: '<color>'; inherits: true; initial-value: transparent; }

  .lay { position: absolute; inset: 0; transition: --c1 0.9s ease, --c2 0.9s ease; }
  .tint {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(70rem 46rem at 14% 4%, color-mix(in srgb, var(--c1) 68%, transparent), transparent 62%),
      radial-gradient(64rem 42rem at 96% -6%, color-mix(in srgb, var(--c2) 85%, transparent), transparent 66%);
  }
  .veil { position: absolute; inset: 0; background: linear-gradient(to bottom, transparent 18%, rgba(var(--bg-rgb), 0.78) 52%, rgba(var(--bg-rgb), 0.94)); }

  /* ---- barre du haut ---- */
  header { position: absolute; inset: 2.4rem 5rem auto 5rem; display: flex; justify-content: space-between; align-items: center; z-index: 5; }
  .brand { display: flex; align-items: center; gap: 0.8rem; font-weight: 700; font-size: 1.5rem; letter-spacing: 0.02em; }
  .brand svg { width: 2.6rem; height: 2.6rem; }
  .right { display: flex; align-items: center; gap: 1.4rem; }
  .pill { display: flex; align-items: center; gap: 0.6rem; padding: 0.5rem 1.1rem; border-radius: 99rem; background: var(--glass); font-size: 1.05rem; color: var(--muted); backdrop-filter: blur(1.2rem); }
  .pill .dot { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: #888; }
  .pill.on { color: var(--text); }
  .pill.on .dot { background: #3ddc84; box-shadow: 0 0 0.8rem #3ddc84; }
  .clock { text-align: right; line-height: 1.15; }
  .clock strong { display: block; font-size: 1.7rem; font-weight: 600; }
  .clock small { display: block; font-size: 0.95rem; color: var(--muted); }
  .clock small::first-letter { text-transform: uppercase; }
  .icon { width: 3.4rem; height: 3.4rem; border-radius: 50%; border: 0; background: var(--glass); display: grid; place-items: center; transition: transform 0.3s var(--ease), background 0.3s; }
  .icon svg { width: 1.7rem; height: 1.7rem; fill: var(--text); }
  .icon.focus { background: var(--ring); transform: scale(1.18); box-shadow: 0 0.6rem 2rem rgba(0, 0, 0, 0.5); }
  .icon.focus svg { fill: var(--bg); }

  /* ---- héro ---- */
  .hero { position: absolute; left: 5rem; top: 10.5rem; width: 56rem; height: 15rem; z-index: 3; }
  .hero-in { position: absolute; inset: 0; }
  .eyebrow { font-size: 1.1rem; letter-spacing: 0.18em; text-transform: uppercase; color: var(--muted); font-weight: 600; }
  h1 { margin: 0.5rem 0 0.4rem; font-size: 4.6rem; line-height: 1; font-weight: 800; letter-spacing: -0.02em; text-shadow: 0 0.3rem 2rem rgba(0, 0, 0, 0.35); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .hero p { margin: 0 0 1.4rem; font-size: 1.5rem; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .cta { display: inline-flex; align-items: center; gap: 0.8rem; padding: 0.7rem 1.6rem 0.7rem 1.3rem; border-radius: 99rem; background: var(--accent); color: #fff; font-weight: 700; font-size: 1.25rem; box-shadow: 0 0.6rem 1.6rem color-mix(in srgb, var(--accent) 45%, transparent); }
  .cta .play { font-size: 0.95rem; }
  kbd { font: inherit; font-size: 0.8rem; padding: 0.15rem 0.5rem; border-radius: 0.4rem; background: rgba(0, 0, 0, 0.2); }

  /* ---- rangées ---- */
  main { position: absolute; left: 0; right: 0; top: 27rem; bottom: 0; overflow: hidden; z-index: 4; }
  .rows { transform: translateY(var(--shift)); transition: transform 0.55s var(--ease); }
  .row { height: 17rem; padding-left: 5rem; opacity: 0.55; transition: opacity 0.4s; }
  .row.active { opacity: 1; }
  .row h2 { margin: 0 0 1.9rem; font-size: 1.35rem; font-weight: 600; color: var(--muted); letter-spacing: 0.02em; }
  .track { display: flex; gap: 1.6rem; transform: translateX(var(--tx)); transition: transform 0.55s var(--ease); width: max-content; }

  .tile {
    position: relative;
    flex: none;
    width: 17rem;
    height: 9.6rem;
    border: 0;
    padding: 0;
    border-radius: 1.2rem;
    overflow: hidden;
    color: var(--fg);
    background: linear-gradient(135deg, var(--a), var(--b));
    box-shadow: 0 0.6rem 1.4rem -0.6rem rgba(0, 0, 0, 0.6);
    transition: transform 0.4s var(--ease), box-shadow 0.4s var(--ease);
    font-family: inherit;
    cursor: pointer;
    transform-style: preserve-3d;
  }
  .tile .mark { position: absolute; right: -0.6rem; bottom: -2.6rem; font-size: 10rem; font-weight: 800; opacity: 0.14; line-height: 1; }
  .tile .ico { position: absolute; left: 1.1rem; top: 1rem; width: 3.6rem; height: 3.6rem; object-fit: contain; filter: drop-shadow(0 0.2rem 0.5rem rgba(0, 0, 0, 0.35)); }
  .tile .label { position: absolute; left: 1.1rem; right: 1rem; bottom: 0.9rem; font-size: 1.55rem; font-weight: 700; letter-spacing: -0.01em; text-shadow: 0 0.1rem 0.6rem rgba(0, 0, 0, 0.35); text-align: left; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tile .shine { position: absolute; inset: 0; background: linear-gradient(115deg, transparent 28%, rgba(255, 255, 255, 0.32) 48%, transparent 66%); transform: translateX(-130%); }
  .tile.focus {
    transform: perspective(60rem) translateY(-0.3rem) scale(1.16);
    box-shadow:
      0 0 0 0.22rem var(--ring),
      0 2.2rem 3.6rem -0.8rem color-mix(in srgb, var(--a) 80%, transparent),
      0 1rem 2rem rgba(0, 0, 0, 0.5);
    z-index: 2;
    animation: settle 0.5s var(--ease);
  }
  .tile.focus .shine { animation: sweep 1.3s 0.1s var(--ease) forwards; }
  .tile.focus[data-tilt='right'] { --ry: 5deg; --rx: 0deg; }
  .tile.focus[data-tilt='left'] { --ry: -5deg; --rx: 0deg; }
  .tile.focus[data-tilt='down'] { --rx: -5deg; --ry: 0deg; }
  .tile.focus[data-tilt='up'] { --rx: 5deg; --ry: 0deg; }
  @keyframes settle {
    from { transform: perspective(60rem) translateY(0) scale(1.04) rotateX(var(--rx, 0deg)) rotateY(var(--ry, 0deg)); }
    to { transform: perspective(60rem) translateY(-0.3rem) scale(1.16) rotateX(0) rotateY(0); }
  }
  @keyframes sweep { to { transform: translateX(130%); } }
</style>
