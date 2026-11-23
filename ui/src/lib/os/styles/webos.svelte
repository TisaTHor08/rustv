<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../i18n';
  import ModWidgets from '../../ModWidgets.svelte';
  import { iconSrc } from '../../tiles.svelte';
  import Weather from '../../Weather.svelte';
  import type { HomeModel } from '../model.svelte';
  import { reducedMotion, tileFg } from '../util';
  import { promosOf, SYS, TOP } from './webos.meta';

  // Style webOS (inspiré, non officiel) : bannière rotative, icônes d'état, lanceur de cartes larges. Voir docs/OS-STYLES.md.
  let { model }: { model: HomeModel } = $props();

  const zone = $derived(model.zone);
  const tiles = $derived(model.rows[0]?.tiles ?? []);
  const promos = $derived(promosOf(model));
  const bi = $derived(promos.length ? Math.min(model.ui.bannerIdx ?? 0, promos.length - 1) : 0);
  const promo = $derived(promos[bi]);
  const sysFocus = $derived<number>(model.ui.sysFocus ?? 0);
  const topFocus = $derived<number>(model.ui.topFocus ?? 1);
  const fTile = $derived(model.focused);

  const ICONS: Record<string, string> = {
    live: 'M21 6h-7.6l3.1-3.1L15.6 2 12 5.6 8.4 2l-.9.9L10.6 6H3a1 1 0 0 0-1 1v13a1 1 0 0 0 1 1h18a1 1 0 0 0 1-1V7a1 1 0 0 0-1-1Zm-1 13H4V8h16Z',
    library: 'M4 6H2v14a2 2 0 0 0 2 2h14v-2H4Zm16-4H8a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2Zm-8 12.5v-9l6 4.5Z',
    search: 'M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z',
    settings: 'M19.4 13a7.7 7.7 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1L15 3.3h-4l-.4 2.6a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L6.6 11a7.7 7.7 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1a7.4 7.4 0 0 0 1.7 1l.4 2.6h4l.4-2.6a7.4 7.4 0 0 0 1.7-1l2.5 1 2-3.5ZM13 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z',
    profile: 'M12 12a4 4 0 1 0-4-4 4 4 0 0 0 4 4Zm0 2c-2.7 0-8 1.3-8 4v2h16v-2c0-2.7-5.3-4-8-4Z',
  };

  // Rotation automatique de la bannière (8 s), arrêtée quand elle a le focus ou si l'utilisateur limite les mouvements.
  $effect(() => {
    const n = promos.length;
    if (n < 2 || zone === 'banner' || reducedMotion()) return;
    const id = setInterval(() => (model.ui.bannerIdx = ((model.ui.bannerIdx ?? 0) + 1) % n), 8000);
    return () => clearInterval(id);
  });

  // La carte ciblée reste ancrée : défilement centré dans le lanceur.
  let track: HTMLElement | undefined = $state();
  $effect(() => {
    void model.focus.col;
    void sysFocus;
    if (!track || (zone !== 'content' && zone !== 'sys')) return;
    anchorTo(track, track.querySelector('.card.on'), 21);
  });


  // Ancrage horizontal : la carte ciblée reste à une position fixe (sauf en début de liste), défilement doux.
  function anchorTo(box: HTMLElement, el: Element | null, rems: number) {
    if (!el) return;
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
    const left = (el as HTMLElement).offsetLeft - rems * rem;
    box.scrollTo({ left: Math.max(0, left), behavior: reducedMotion() ? 'auto' : 'smooth' });
  }

  const hover = (z: string, key: string, i: number) => () => {
    model.ui[key] = i;
    model.setZone(z);
  };
</script>

<div class="wo">
  <div class="wall"></div>

  <!-- Bannière -->
  <section class="banner" class:act={zone === 'banner'}>
    {#key promo?.id}
      <div class="art" transition:fade={{ duration: 500 }} style:--a={promo?.color ?? '#2a3f8f'} style:--b={promo?.color2 ?? '#10183a'}>
        {#if promo?.poster}<img class="poster" src={iconSrc(promo.poster)} alt="" />{:else if promo?.tile?.icon}<img class="ico" src={iconSrc(promo.tile.icon)} alt="" />{:else if promo}<span class="big">{promo.title[0]}</span>{/if}
      </div>
      {#if promo}
        <div class="info" transition:fade={{ duration: 500 }}>
          <span class="kicker">{promo.resume ? t('os.webos.resume') : t('os.webos.featured')}</span>
          <h1>{promo.title}</h1>
          {#if promo.desc}<p>{promo.desc}</p>{/if}
          {#if promo.pct !== undefined}<span class="prog"><i style:width="{promo.pct}%"></i></span>{/if}
          <button class="cta" class:focus={zone === 'banner'} onmouseenter={() => model.setZone('banner')} onclick={() => (promo.resume ? model.resume(promo.resume) : promo.tile && model.open(promo.tile))}>
            <svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg>{promo.resume ? t('os.webos.watch') : t('os.webos.open')}
          </button>
        </div>
      {/if}
    {/key}
    {#if promos.length > 1}
      <div class="dots">{#each promos as p, i (p.id)}<i class:on={i === bi}></i>{/each}</div>
    {/if}
  </section>

  <!-- Icônes d'état en haut à droite -->
  <header>
    {#if model.widgetsNative}
      <ModWidgets toast={model.note} at="status" provided={['status']} />
      <Weather />
      <span class="chip" class:on={model.phones > 0} title={model.phonesLabel}><i></i>{model.phones > 0 ? model.phones : ''}</span>
    {/if}
    {#each TOP as id, i}
      <button class="ic" class:focus={zone === 'top' && topFocus === i} aria-label={t(`os.webos.${id}`)} onmouseenter={hover('top', 'topFocus', i)} onclick={() => ((model.ui.topFocus = i), model.setZone('top'), model.activate())}>
        <svg viewBox="0 0 24 24"><path d={ICONS[id]} /></svg>
      </button>
    {/each}
    {#if model.widgetsNative}<span class="clock">{model.time}</span>{/if}
  </header>

  <!-- Lanceur -->
  <nav class="launcher">
    <div class="title">{t('os.webos.home')}</div>
    <div class="track" bind:this={track}>
      {#each SYS as id, i}
        {@const on = zone === 'sys' && sysFocus === i}
        <button class="card sysc" class:on onmouseenter={hover('sys', 'sysFocus', i)} onclick={() => ((model.ui.sysFocus = i), model.setZone('sys'), model.activate())}>
          <span class="face" data-k={id}>
            <svg viewBox="0 0 24 24"><path d={ICONS[id]} /></svg>
          </span>
          <span class="name">{t(`os.webos.${id}`)}</span>
        </button>
      {/each}
      <span class="gap"></span>
      {#each tiles as tile, i (tile.id)}
        {@const on = zone === 'content' && i === model.focus.col}
        <button class="card" class:on class:sel={zone !== 'content' && fTile?.id === tile.id} onmouseenter={() => (model.setZone('content'), model.focusAt(0, i, true))} onclick={() => model.open(tile)} oncontextmenu={(e) => (e.preventDefault(), model.contextMenu(tile))}>
          <span class="face" style:--a={tile.color} style:--b={tile.color2} style:--fg={tileFg(tile.dark_text)}>
            {#if tile.icon}<img src={iconSrc(tile.icon)} alt="" />{:else}<b>{tile.name[0]}</b>{/if}
          </span>
          <span class="name">{tile.name}</span>
        </button>
      {/each}
    </div>
  </nav>
</div>

<style>
  .wo { position: absolute; inset: 0; overflow: hidden; font-family: var(--font); color: #f4f7fb; background: #0a0f1d; }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; padding: 0; }
  svg { fill: currentColor; }

  /* fond flou façon fond d'écran : taches de couleur adoucies */
  .wall { position: absolute; inset: -4rem; filter: blur(3rem); background: radial-gradient(48rem 34rem at 18% 22%, rgba(88, 110, 220, 0.5), transparent 70%), radial-gradient(52rem 38rem at 84% 12%, rgba(60, 150, 210, 0.38), transparent 70%), radial-gradient(60rem 40rem at 60% 90%, rgba(120, 70, 190, 0.34), transparent 70%), linear-gradient(160deg, #141b38, #0a0f1d 70%); }

  /* bannière */
  .banner { position: absolute; z-index: 1; left: 0; right: 0; top: 0; bottom: 18rem; }
  .art { position: absolute; inset: 0; background: radial-gradient(52rem 30rem at 76% 34%, color-mix(in srgb, var(--a) 70%, transparent), transparent 72%), linear-gradient(135deg, color-mix(in srgb, var(--b) 60%, transparent), transparent 72%); }
  .art::after { content: ''; position: absolute; inset: 0; background: linear-gradient(90deg, rgba(10, 15, 29, 0.78) 8%, transparent 62%), linear-gradient(0deg, rgba(10, 15, 29, 0.9) 0%, transparent 40%); }
  .art .big { position: absolute; right: 12%; top: 6rem; font-size: 24rem; font-weight: 700; line-height: 1; opacity: 0.15; }
  .art .ico { position: absolute; right: 18%; top: 9rem; width: 15rem; height: 15rem; object-fit: contain; opacity: 0.92; filter: drop-shadow(0 1rem 2rem rgba(0, 0, 0, 0.45)); }
  .art .poster { position: absolute; right: 8%; top: 5rem; height: 24rem; width: 17rem; object-fit: cover; border-radius: var(--r); box-shadow: 0 1.4rem 3rem rgba(0, 0, 0, 0.5); }
  .info { position: absolute; z-index: 2; left: 5rem; bottom: 2.6rem; width: min(56rem, 58%); }
  .kicker { font-size: 1.2rem; font-weight: 500; letter-spacing: 0.06em; color: rgba(255, 255, 255, 0.7); }
  h1 { margin: 0.4rem 0 0.4rem; font-size: 4.4rem; font-weight: 600; line-height: 1.08; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .info p { margin: 0 0 1.2rem; font-size: 1.4rem; color: rgba(255, 255, 255, 0.78); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .prog { display: block; width: 24rem; height: 0.3rem; margin: 1.2rem 0 1.4rem; border-radius: 1rem; background: rgba(255, 255, 255, 0.25); overflow: hidden; }
  .prog i { display: block; height: 100%; background: #fff; }
  .cta { display: inline-flex; align-items: center; gap: 0.7rem; height: 4rem; padding: 0 2.2rem 0 1.6rem; border-radius: 99rem; font-size: 1.4rem; font-weight: 500; background: rgba(255, 255, 255, 0.18); transition: background 0.2s, color 0.2s, transform 0.25s var(--ease); }
  .cta svg { width: 1.8rem; height: 1.8rem; }
  .cta.focus { background: #fff; color: #11172b; transform: scale(1.06); box-shadow: 0 0.6rem 1.8rem rgba(0, 0, 0, 0.45); }
  .dots { position: absolute; z-index: 2; right: 5rem; bottom: 3rem; display: flex; gap: 0.6rem; }
  .dots i { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: rgba(255, 255, 255, 0.35); }
  .dots i.on { width: 2rem; border-radius: 1rem; background: #fff; }

  header { position: absolute; z-index: 5; top: 1.8rem; right: 4rem; display: flex; align-items: center; gap: 1.1rem; }
  .ic { width: 3.6rem; height: 3.6rem; border-radius: 50%; display: grid; place-items: center; color: rgba(255, 255, 255, 0.88); transition: background 0.2s, color 0.2s, transform 0.25s var(--ease); }
  .ic svg { width: 2rem; height: 2rem; }
  .ic.focus { background: #fff; color: #11172b; transform: scale(1.12); }
  .clock { margin-left: 0.8rem; font-size: 1.7rem; font-weight: 500; }
  .chip { display: inline-flex; align-items: center; gap: 0.4rem; font-size: 1.1rem; color: rgba(255, 255, 255, 0.65); }
  .chip i { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: #8590aa; }
  .chip.on i { background: #3ddc84; }

  /* lanceur : tiers inférieur */
  .launcher { position: absolute; z-index: 3; left: 0; right: 0; bottom: 0; height: 18rem; padding-top: 0.4rem; background: linear-gradient(0deg, rgba(8, 12, 24, 0.78), rgba(8, 12, 24, 0.4) 70%, transparent); backdrop-filter: blur(1.2rem); }
  .title { padding-left: 5rem; font-size: 1.5rem; font-weight: 500; color: rgba(255, 255, 255, 0.85); height: 2.4rem; }
  .track { position: relative; display: flex; align-items: flex-start; gap: 1.4rem; padding: 1.2rem 5rem 2rem 5rem; overflow-x: hidden; scrollbar-width: none; }
  .gap { flex: none; width: 0.1rem; height: 8rem; background: rgba(255, 255, 255, 0.16); margin: 0 0.6rem; }
  .card { flex: none; width: 16rem; text-align: left; transition: transform 0.28s var(--ease); }
  .face { position: relative; display: grid; place-items: center; width: 100%; aspect-ratio: 2 / 1; border-radius: var(--r); overflow: hidden; background: linear-gradient(135deg, var(--a), var(--b)); color: var(--fg); box-shadow: 0 0.4rem 1rem rgba(0, 0, 0, 0.35); }
  .face b { font-size: 3.4rem; font-weight: 700; }
  .face img { width: 3.8rem; height: 3.8rem; object-fit: contain; }
  .face svg { width: 3.4rem; height: 3.4rem; }
  .sysc .face { background: rgba(255, 255, 255, 0.16); backdrop-filter: blur(0.6rem); color: #fff; box-shadow: inset 0 0 0 0.06rem rgba(255, 255, 255, 0.2); }
  .name { display: block; margin-top: 0.7rem; font-size: 1.2rem; font-weight: 400; color: rgba(255, 255, 255, 0.78); text-align: center; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card.on { transform: scale(var(--focus-scale)); position: relative; z-index: 3; }
  .card.on .face { box-shadow: 0 0 0 var(--focus-ring) #fff, 0 1.2rem 2.4rem rgba(0, 0, 0, 0.55); }
  .card.on .name { color: #fff; font-weight: 600; }
  .card.sel .face { box-shadow: 0 0 0 0.12rem rgba(255, 255, 255, 0.4); }
</style>
