<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { fly, fade } from 'svelte/transition';
  import { api, type ShellState, type Tile } from './api';
  import Pairing from './Pairing.svelte';

  type Modal = null | { kind: 'settings' } | { kind: 'pairing' } | { kind: 'url'; tile: Tile };

  const CATEGORIES: [Tile['category'], string][] = [
    ['video', 'Films et séries'],
    ['tv', 'Télévision'],
    ['music', 'Musique'],
    ['library', 'Médiathèque'],
    ['games', 'Jeux'],
  ];
  const TILE_STEP = 18.6; // rem : largeur d'une tuile + espacement
  const ROW_H = 17; // rem

  let st = $state<ShellState | null>(null);
  let now = $state(new Date());
  let focus = $state({ row: 0, col: 0 });
  let tilt = $state('');
  let modal = $state<Modal>(null);
  let menuIndex = $state(0);
  let toast = $state('');
  let urlValue = $state('');
  let urlInput = $state<HTMLInputElement | null>(null);
  let recent = $state<string[]>(loadRecent());

  function loadRecent(): string[] {
    try {
      return JSON.parse(localStorage.getItem('rustv.recent') ?? '[]');
    } catch {
      return [];
    }
  }

  const rows = $derived.by(() => {
    if (!st) return [];
    const out: { title: string; tiles: Tile[] }[] = [];
    const byId = new Map(st.catalog.map((t) => [t.id, t]));
    const rec = recent.map((id) => byId.get(id)).filter((t): t is Tile => !!t);
    if (rec.length) out.push({ title: 'Récemment utilisées', tiles: rec });
    for (const [cat, title] of CATEGORIES) {
      const tiles = st.catalog.filter((t) => t.category === cat);
      if (tiles.length) out.push({ title, tiles });
    }
    return out;
  });

  const focused = $derived(focus.row >= 0 ? rows[focus.row]?.tiles[focus.col] : undefined);
  const phones = $derived(st ? st.devices.filter((d) => d.connected).length : 0);
  const menu = $derived([
    { id: 'pairing', label: 'Téléphones et appairage' },
    { id: 'blackout', label: `Éteindre les autres écrans : ${st?.settings.blackout ? 'oui' : 'non'}` },
    { id: 'autostart', label: `Démarrer avec Windows : ${{ off: 'non', tray: 'zone de notification', tv: 'directement en mode TV' }[st?.autostart ?? 'off']}` },
    { id: 'screen', label: "Changer d'écran" },
    { id: 'exit', label: 'Quitter le mode TV' },
    { id: 'quit', label: 'Fermer RusTV' },
  ]);

  async function refresh() {
    st = await api.state();
  }

  function note(msg: string) {
    toast = msg;
    setTimeout(() => (toast = ''), 3500);
  }

  // ---------- navigation à la télécommande ----------

  function move(dr: number, dc: number, dirName: string) {
    tilt = dirName;
    let { row, col } = focus;
    if (dr !== 0) {
      const nr = Math.max(-1, Math.min(rows.length - 1, row + dr));
      if (nr === row) return;
      row = nr;
      if (row >= 0) col = Math.min(col, rows[row].tiles.length - 1);
      else col = 0;
    } else if (row >= 0) {
      col = Math.max(0, Math.min(rows[row].tiles.length - 1, col + dc));
    }
    focus = { row, col };
  }

  async function activate() {
    if (focus.row === -1) {
      modal = { kind: 'settings' };
      menuIndex = 0;
      return;
    }
    if (focused) await openTile(focused);
  }

  async function openTile(t: Tile) {
    if (t.needs_url && !st?.settings.urls[t.id]) return askUrl(t);
    try {
      const r = await api.launch(t.id);
      if (r === 'needs_url') return askUrl(t);
      recent = [t.id, ...recent.filter((x) => x !== t.id)].slice(0, 8);
      try {
        localStorage.setItem('rustv.recent', JSON.stringify(recent));
      } catch {}
    } catch (e) {
      note(String(e));
    }
  }

  async function askUrl(t: Tile) {
    urlValue = st?.settings.urls[t.id] ?? '';
    modal = { kind: 'url', tile: t };
    await tick();
    urlInput?.focus();
  }

  async function submitUrl() {
    if (modal?.kind !== 'url') return;
    const t = modal.tile;
    const v = urlValue.trim();
    if (!v) return;
    modal = null;
    try {
      await api.launch(t.id, v);
      await refresh();
    } catch (e) {
      note(String(e));
    }
  }

  async function menuAction(id: string) {
    if (id === 'pairing') modal = { kind: 'pairing' };
    else if (id === 'blackout') {
      await api.setBlackout(!st!.settings.blackout);
      await refresh();
    } else if (id === 'autostart') {
      const next = { off: 'tray', tray: 'tv', tv: 'off' }[st!.autostart];
      await api.setAutostart(next);
      await refresh();
    } else if (id === 'screen') {
      modal = null;
      await api.tvExit();
      await api.tvStart();
    } else if (id === 'exit') await api.tvExit();
    else if (id === 'quit') await api.quit();
  }

  function onKey(e: KeyboardEvent) {
    if (modal?.kind === 'pairing') return; // le composant gère ses touches
    if (modal?.kind === 'url') {
      if (e.key === 'Enter') submitUrl();
      else if (e.key === 'Escape') modal = null;
      else return;
      e.preventDefault();
      return;
    }
    if (modal?.kind === 'settings') {
      if (e.key === 'ArrowDown') menuIndex = Math.min(menu.length - 1, menuIndex + 1);
      else if (e.key === 'ArrowUp') menuIndex = Math.max(0, menuIndex - 1);
      else if (e.key === 'Enter') menuAction(menu[menuIndex].id);
      else if (e.key === 'Escape' || e.key === 'Backspace') modal = null;
      else return;
      e.preventDefault();
      return;
    }
    switch (e.key) {
      case 'ArrowDown': move(1, 0, 'down'); break;
      case 'ArrowUp': move(-1, 0, 'up'); break;
      case 'ArrowLeft': move(0, -1, 'left'); break;
      case 'ArrowRight': move(0, 1, 'right'); break;
      case 'Enter': activate(); break;
      default: return;
    }
    e.preventDefault();
  }

  // ---------- cycle de vie ----------

  let idleTimer: number | undefined;
  function wake() {
    document.documentElement.classList.remove('idle');
    clearTimeout(idleTimer);
    idleTimer = window.setTimeout(() => document.documentElement.classList.add('idle'), 3000);
  }

  onMount(() => {
    refresh();
    const clock = setInterval(() => (now = new Date()), 1000 * 15);
    const un = api.onDevicesChanged(() => refresh());
    wake();
    return () => {
      clearInterval(clock);
      un.then((f) => f());
    };
  });

  const time = $derived(now.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' }));
  const date = $derived(now.toLocaleDateString('fr-FR', { weekday: 'long', day: 'numeric', month: 'long' }));
  const fg = (t?: Tile) => (t?.dark_text ? '#16130a' : '#ffffff');
</script>

<svelte:window onkeydown={onKey} onmousemove={wake} />

<div class="root" style:--c1={focused?.color ?? '#3a2a1a'} style:--c2={focused?.color2 ?? '#1a1a2e'}>
  <div class="bg"></div>
  <div class="veil"></div>

  <!-- Barre du haut -->
  <header>
    <div class="brand">
      <svg viewBox="0 0 64 64" aria-hidden="true">
        <defs>
          <linearGradient id="lg" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stop-color="#ffb347" /><stop offset="0.6" stop-color="#f74c00" /><stop offset="1" stop-color="#b83200" />
          </linearGradient>
        </defs>
        <g fill="url(#lg)" transform="translate(32 32)">
          <circle r="20" fill="none" stroke="url(#lg)" stroke-width="4.4" />
          {#each Array(12) as _, i}<rect x="-2.3" y="-29" width="4.6" height="7.4" rx="1" transform="rotate({i * 30})" />{/each}
          <path d="M-5.6 -8 L-5.6 8 Q-5.6 9.4 -4.4 8.6 L9 1.2 Q10 0 9 -1.2 L-4.4 -8.6 Q-5.6 -9.4 -5.6 -8 Z" />
        </g>
      </svg>
      <span>RusTV</span>
    </div>
    <div class="right">
      <div class="pill" class:on={phones > 0}>
        <span class="dot"></span>{phones > 0 ? `${phones} téléphone${phones > 1 ? 's' : ''}` : 'Aucun téléphone'}
      </div>
      <div class="clock"><strong>{time}</strong><small>{date}</small></div>
      <button class="icon" class:focus={focus.row === -1} onclick={() => ((modal = { kind: 'settings' }), (menuIndex = 0))} onmouseenter={() => (focus = { row: -1, col: 0 })} aria-label="Réglages">
        <svg viewBox="0 0 24 24"><path d="M19.4 13a7.7 7.7 0 0 0 0-2l2-1.6a.5.5 0 0 0 .1-.6l-1.9-3.3a.5.5 0 0 0-.6-.2l-2.4 1a7.2 7.2 0 0 0-1.7-1l-.4-2.5a.5.5 0 0 0-.5-.4h-3.8a.5.5 0 0 0-.5.4L9.3 5.3a7.2 7.2 0 0 0-1.7 1l-2.4-1a.5.5 0 0 0-.6.2L2.7 8.8a.5.5 0 0 0 .1.6l2 1.6a7.7 7.7 0 0 0 0 2l-2 1.6a.5.5 0 0 0-.1.6l1.9 3.3a.5.5 0 0 0 .6.2l2.4-1c.5.4 1.1.7 1.7 1l.4 2.5c0 .2.2.4.5.4h3.8c.3 0 .5-.2.5-.4l.4-2.5c.6-.3 1.2-.6 1.7-1l2.4 1c.2.1.5 0 .6-.2l1.9-3.3a.5.5 0 0 0-.1-.6ZM12 15.5A3.5 3.5 0 1 1 12 8.5a3.5 3.5 0 0 1 0 7Z" /></svg>
      </button>
    </div>
  </header>

  <!-- Héro -->
  <section class="hero">
    {#key focused?.id}
      <div class="hero-in" in:fly={{ y: 24, duration: 520, easing: cubicOut }}>
        {#if focused}
          <span class="eyebrow">{CATEGORIES.find((c) => c[0] === focused.category)?.[1]}</span>
          <h1>{focused.name}</h1>
          <p>{focused.desc}</p>
          <div class="cta"><span class="play">▶</span> Ouvrir <kbd>OK</kbd></div>
        {:else}
          <span class="eyebrow">RusTV</span>
          <h1>Réglages</h1>
          <p>Téléphones, écrans et préférences</p>
        {/if}
      </div>
    {/key}
  </section>

  <!-- Rangées -->
  <main style:--shift="{-focus.row * ROW_H}rem">
    <div class="rows">
      {#each rows as r, ri (r.title)}
        <div class="row" class:active={ri === focus.row}>
          <h2>{r.title}</h2>
          <div class="track" style:--tx="{-Math.max(0, (ri === focus.row ? focus.col : 0) - 2) * TILE_STEP}rem">
            {#each r.tiles as t, ci (t.id)}
              <button
                class="tile"
                class:focus={ri === focus.row && ci === focus.col}
                data-tilt={ri === focus.row && ci === focus.col ? tilt : ''}
                style:--a={t.color}
                style:--b={t.color2}
                style:--fg={fg(t)}
                onmouseenter={() => (focus = { row: ri, col: ci })}
                onclick={() => openTile(t)}
              >
                <span class="mark">{t.name[0]}</span>
                <span class="shine"></span>
                <span class="label">{t.name}</span>
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </main>

  {#if toast}<div class="toast" transition:fade={{ duration: 200 }}>{toast}</div>{/if}

  <!-- Modales -->
  {#if modal}
    <div class="scrim" transition:fade={{ duration: 180 }}>
      {#if modal.kind === 'settings'}
        <div class="sheet" in:fly={{ y: 30, duration: 360, easing: cubicOut }}>
          <h3>Réglages</h3>
          {#each menu as m, i}
            <button class="item" class:sel={i === menuIndex} onmouseenter={() => (menuIndex = i)} onclick={() => menuAction(m.id)}>{m.label}</button>
          {/each}
          <p class="hint">▲▼ choisir · OK valider · Retour fermer — RusTV {st?.version}</p>
        </div>
      {:else if modal.kind === 'pairing'}
        <div class="sheet wide" in:fly={{ y: 30, duration: 360, easing: cubicOut }}>
          <Pairing embedded onclose={() => (modal = { kind: 'settings' })} />
        </div>
      {:else if modal.kind === 'url'}
        <div class="sheet" in:fly={{ y: 30, duration: 360, easing: cubicOut }}>
          <h3>Adresse de {modal.tile.name}</h3>
          <input bind:this={urlInput} bind:value={urlValue} placeholder={modal.tile.url_hint ?? 'http://…'} autocomplete="off" spellcheck="false" />
          <p class="hint">Saisissez avec le clavier du téléphone · OK valider · Retour annuler</p>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  @property --c1 { syntax: '<color>'; inherits: true; initial-value: #3a2a1a; }
  @property --c2 { syntax: '<color>'; inherits: true; initial-value: #1a1a2e; }

  .root {
    position: fixed;
    inset: 0;
    transition: --c1 0.9s ease, --c2 0.9s ease;
  }
  .bg {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(70rem 46rem at 14% 4%, color-mix(in srgb, var(--c1) 70%, transparent), transparent 62%),
      radial-gradient(64rem 42rem at 96% -6%, color-mix(in srgb, var(--c2) 85%, transparent), transparent 66%),
      var(--bg);
  }
  .veil {
    position: absolute;
    inset: 0;
    background: linear-gradient(to bottom, transparent 18%, rgba(6, 8, 12, 0.78) 52%, rgba(6, 8, 12, 0.94));
  }

  /* ---- barre du haut ---- */
  header {
    position: absolute;
    inset: 2.4rem 5rem auto 5rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    z-index: 5;
  }
  .brand { display: flex; align-items: center; gap: 0.8rem; font-weight: 700; font-size: 1.5rem; letter-spacing: 0.02em; }
  .brand svg { width: 2.6rem; height: 2.6rem; }
  .right { display: flex; align-items: center; gap: 1.6rem; }
  .pill { display: flex; align-items: center; gap: 0.6rem; padding: 0.5rem 1.1rem; border-radius: 99rem; background: var(--glass); font-size: 1.05rem; color: var(--muted); backdrop-filter: blur(1.2rem); }
  .pill .dot { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: #666; }
  .pill.on { color: var(--text); }
  .pill.on .dot { background: #3ddc84; box-shadow: 0 0 0.8rem #3ddc84; }
  .clock { text-align: right; line-height: 1.15; }
  .clock strong { display: block; font-size: 1.7rem; font-weight: 600; }
  .clock small { display: block; font-size: 0.95rem; color: var(--muted); }
  .clock small::first-letter { text-transform: uppercase; }
  .icon { width: 3.4rem; height: 3.4rem; border-radius: 50%; border: 0; background: var(--glass); display: grid; place-items: center; transition: transform 0.3s var(--ease), background 0.3s; }
  .icon svg { width: 1.7rem; height: 1.7rem; fill: var(--text); }
  .icon.focus { background: #fff; transform: scale(1.18); box-shadow: 0 0.6rem 2rem rgba(0, 0, 0, 0.5); }
  .icon.focus svg { fill: #111; }

  /* ---- héro ---- */
  .hero { position: absolute; left: 5rem; top: 10.5rem; width: 56rem; height: 15rem; z-index: 3; }
  .hero-in { position: absolute; inset: 0; }
  .eyebrow { font-size: 1.1rem; letter-spacing: 0.18em; text-transform: uppercase; color: var(--muted); font-weight: 600; }
  h1 { margin: 0.5rem 0 0.4rem; font-size: 4.6rem; line-height: 1; font-weight: 800; letter-spacing: -0.02em; text-shadow: 0 0.3rem 2rem rgba(0, 0, 0, 0.45); }
  .hero p { margin: 0 0 1.4rem; font-size: 1.5rem; color: var(--muted); }
  .cta { display: inline-flex; align-items: center; gap: 0.8rem; padding: 0.7rem 1.6rem 0.7rem 1.3rem; border-radius: 99rem; background: #fff; color: #111; font-weight: 700; font-size: 1.25rem; }
  .cta .play { font-size: 0.95rem; }
  kbd { font: inherit; font-size: 0.8rem; padding: 0.15rem 0.5rem; border-radius: 0.4rem; background: rgba(0, 0, 0, 0.12); }

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
  .tile .label { position: absolute; left: 1.1rem; bottom: 0.9rem; font-size: 1.55rem; font-weight: 700; letter-spacing: -0.01em; text-shadow: 0 0.1rem 0.6rem rgba(0, 0, 0, 0.35); }
  .tile .shine { position: absolute; inset: 0; background: linear-gradient(115deg, transparent 28%, rgba(255, 255, 255, 0.32) 48%, transparent 66%); transform: translateX(-130%); }
  .tile.focus {
    transform: perspective(60rem) translateY(-0.3rem) scale(1.16);
    box-shadow:
      0 0 0 0.22rem rgba(255, 255, 255, 0.95),
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

  /* ---- toast ---- */
  .toast { position: absolute; left: 50%; bottom: 3rem; transform: translateX(-50%); padding: 0.9rem 1.8rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.95); color: #111; font-size: 1.2rem; font-weight: 600; z-index: 20; }

  /* ---- modales ---- */
  .scrim { position: absolute; inset: 0; background: rgba(4, 6, 10, 0.72); backdrop-filter: blur(1.6rem); display: grid; place-items: center; z-index: 10; }
  .sheet { width: 36rem; padding: 2.2rem 2.4rem; border-radius: 1.8rem; background: rgba(34, 38, 48, 0.92); box-shadow: 0 3rem 6rem rgba(0, 0, 0, 0.6), inset 0 0 0 0.08rem rgba(255, 255, 255, 0.08); }
  .sheet.wide { width: 56rem; }
  .sheet h3 { margin: 0 0 1.2rem; font-size: 1.9rem; }
  .item { display: block; width: 100%; text-align: left; margin: 0.3rem 0; padding: 0.95rem 1.2rem; border: 0; border-radius: 0.9rem; background: transparent; color: var(--text); font-family: inherit; font-size: 1.35rem; transition: background 0.2s, transform 0.25s var(--ease); }
  .item.sel { background: #fff; color: #111; transform: scale(1.025); font-weight: 600; }
  .hint { margin: 1.2rem 0 0; font-size: 0.95rem; color: var(--muted); }
  .sheet input { width: 100%; padding: 1rem 1.2rem; border-radius: 0.9rem; border: 0.12rem solid rgba(255, 255, 255, 0.25); background: rgba(0, 0, 0, 0.35); color: var(--text); font-size: 1.4rem; outline: none; }
  .sheet input:focus { border-color: #fff; }
</style>
