<script lang="ts">
  import { t } from './i18n.ts';
  import { activate, app, deactivate, send, unpair } from './connection.svelte.ts';
  import { longpress, repeat, tick } from './haptics.ts';
  import type { KeyName, MediaAction } from './protocol/types.ts';
  import Apps from './Apps.svelte';
  import Surface from './Surface.svelte';
  import Touchpad from './Touchpad.svelte';
  import TextInput from './TextInput.svelte';
  import Vault from './Vault.svelte';

  type Tab = 'remote' | 'apps' | 'pad';
  let tab = $state<Tab>('remote');
  let altOpen = $state(false);
  let altTimer: number | undefined;
  let manualKeyboard = $state(false);
  let vaultOpen = $state(false);
  let mode = $state<'dpad' | 'surface'>(loadMode());

  function loadMode(): 'dpad' | 'surface' {
    try {
      return localStorage.getItem('rustv.mode') === 'dpad' ? 'dpad' : 'surface';
    } catch {
      return 'surface';
    }
  }
  function setMode(m: 'dpad' | 'surface') {
    mode = m;
    try {
      localStorage.setItem('rustv.mode', m);
    } catch {}
  }

  const key = (k: KeyName) => () => send({ t: 'key', k });
  const media = (a: MediaAction) => (e: PointerEvent) => {
    e.preventDefault();
    tick();
    send({ t: 'media', a });
  };

  // Le PC relâche Alt tout seul après 5 s sans activité : on aligne l'affichage.
  function touchAlt() {
    clearTimeout(altTimer);
    altTimer = window.setTimeout(() => (altOpen = false), 5000);
  }
  function tabSwitch() {
    send({ t: 'altTab', a: 'open' });
    altOpen = true;
    touchAlt();
  }
  function alt(a: 'left' | 'right') {
    send({ t: 'altTab', a });
    touchAlt();
  }
  function ok() {
    if (altOpen) {
      send({ t: 'altTab', a: 'confirm' });
      altOpen = false;
      clearTimeout(altTimer);
    } else send({ t: 'key', k: 'enter' });
  }

  const connected = $derived(app.status === 'connected');
  const showKeyboard = $derived(connected && (app.textFocus || manualKeyboard));
</script>

<div class="screen">
  <header>
    <span class="dot" class:on={connected} class:wait={app.status === 'connecting'}></span>
    <div class="title">
      <strong>{app.pairing?.name}</strong>
      <small>
        {#if connected}{app.via}{app.tvMode ? t('mode.tv') : ''}{:else if app.status === 'connecting'}{t('connecting')}{app.error ? ` (${app.error})` : ''}{:else}{t('disconnected')}{/if}
      </small>
    </div>
    <button class="small" onclick={() => confirm(t('forget')) && unpair()}>⚙</button>
  </header>

  {#if !connected}
    <div class="center">
      {#if app.status === 'idle'}
        <button class="primary activate" onclick={activate}>{t('activate')}</button>
      {:else}
        <div class="spinner"></div>
        <button class="small" onclick={deactivate}>{t('cancel')}</button>
      {/if}
    </div>
  {:else}
    <nav>
      <button class:sel={tab === 'remote'} onclick={() => (tab = 'remote')}>{t('tab.remote')}</button>
      <button class:sel={tab === 'apps'} onclick={() => (tab = 'apps')}>{t('tab.apps')}</button>
      <button class:sel={tab === 'pad'} onclick={() => (tab = 'pad')}>{t('tab.mouse')}</button>
    </nav>

    <section>
      {#if tab === 'remote'}
        {#if app.now.active}
          <div class="now">
            <div class="np">
              <b>{app.now.title}</b>
              <small>{app.now.artist || app.now.source.replace(/\.exe$/i, '')}</small>
              {#if app.now.duration > 0}<i class="pb"><u style:width="{Math.min(100, (app.now.position / app.now.duration) * 100)}%"></u></i>{/if}
            </div>
            <button onpointerdown={media('prev')} aria-label="Précédent">⏮</button>
            <button class="primary" onpointerdown={media('playPause')} aria-label="Lecture / pause">{app.now.playing ? '⏸' : '▶'}</button>
            <button onpointerdown={media('next')} aria-label="Suivant">⏭</button>
          </div>
        {/if}

        {#if !app.tvMode}
          <button class="primary tvstart" onpointerdown={(e) => { e.preventDefault(); tick(true); send({ t: 'tv', on: true }); }}>{t('tv.start')}</button>
        {/if}

        <div class="seg">
          <button class:sel={mode === 'surface'} onclick={() => setMode('surface')}>{t('seg.surface')}</button>
          <button class:sel={mode === 'dpad'} onclick={() => setMode('dpad')}>{t('seg.dpad')}</button>
        </div>

        {#if mode === 'surface'}
          <Surface />
        {:else}
          <div class="dpad">
            <button class="up" use:repeat={key('up')}>▲</button>
            <button class="left" use:repeat={key('left')}>◀</button>
            <button class="ok primary" use:longpress={{ onTap: ok, onLong: key('menu') }}>{altOpen ? t('validate') : 'OK'}</button>
            <button class="right" use:repeat={key('right')}>▶</button>
            <button class="down" use:repeat={key('down')}>▼</button>
          </div>
        {/if}

        {#if altOpen}
          <div class="row">
            <button use:repeat={() => alt('left')}>{t('tab.prev')}</button>
            <button use:repeat={() => alt('right')}>{t('tab.next')}</button>
            <button class="primary" onpointerdown={(e) => { e.preventDefault(); tick(true); ok(); }}>{t('validate')}</button>
          </div>
        {/if}

        <div class="row">
          <button use:longpress={{ onTap: key('back'), onLong: key('backlong') }}>{t('back')}</button>
          <button onpointerdown={(e) => { e.preventDefault(); tick(true); send({ t: 'home' }); }}>{t('home')}</button>
          <button onpointerdown={(e) => { e.preventDefault(); tick(); tabSwitch(); }}>{t('tabs')}</button>
          <button onpointerdown={(e) => { e.preventDefault(); tick(); send({ t: 'key', k: 'search' }); }} aria-label="Recherche">🔍</button>
          <button class:active={manualKeyboard} onpointerdown={(e) => { e.preventDefault(); tick(); manualKeyboard = !manualKeyboard; }}>⌨</button>
          <button class:active={app.textFocus} onpointerdown={(e) => { e.preventDefault(); tick(); vaultOpen = true; }} aria-label="Bitwarden">🔑</button>
        </div>

        <div class="row media">
          <button onpointerdown={media('prev')} aria-label="Précédent">⏮</button>
          <button onpointerdown={media('playPause')} aria-label="Lecture / pause">⏯</button>
          <button onpointerdown={media('next')} aria-label="Suivant">⏭</button>
          <button use:repeat={() => send({ t: 'media', a: 'volDown' })} aria-label="Volume moins">🔉</button>
          <button use:repeat={() => send({ t: 'media', a: 'volUp' })} aria-label="Volume plus">🔊</button>
          <button onpointerdown={media('mute')} aria-label="Muet">🔇</button>
        </div>

        {#if app.tvMode}
          <button class="quit" onpointerdown={(e) => { e.preventDefault(); tick(true); send({ t: 'tv', on: false }); }}>{t('tv.quit')}</button>
        {/if}
      {:else if tab === 'apps'}
        <Apps />
      {:else}
        <Touchpad />
      {/if}
    </section>
  {/if}
</div>

{#if vaultOpen}<Vault onclose={() => (vaultOpen = false)} />{/if}
{#if showKeyboard && !vaultOpen}<TextInput onclose={() => (manualKeyboard = false)} />{/if}

<style>
  .screen {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: max(12px, env(safe-area-inset-top)) 16px max(16px, env(safe-area-inset-bottom));
    gap: 12px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .title {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  small {
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #555;
  }
  .dot.on {
    background: var(--ok);
  }
  .dot.wait {
    background: var(--warn);
  }
  .small {
    padding: 8px 14px;
    border-radius: 12px;
  }
  .center {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 24px;
  }
  .activate {
    width: 220px;
    height: 220px;
    border-radius: 50%;
    font-size: 1.6rem;
  }
  .spinner {
    width: 56px;
    height: 56px;
    border: 5px solid var(--panel-2);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  nav {
    display: flex;
    gap: 8px;
  }
  nav button {
    flex: 1;
    padding: 10px 0;
    border-radius: 12px;
    font-size: 0.9rem;
  }
  nav .sel {
    background: var(--accent);
  }
  section {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .tvstart {
    padding: 16px;
    font-size: 1.05rem;
    border-radius: 18px;
  }
  .seg {
    align-self: center;
    display: flex;
    background: var(--panel);
    border-radius: 12px;
    padding: 3px;
  }
  .seg button {
    padding: 6px 18px;
    border-radius: 10px;
    font-size: 0.85rem;
    background: transparent;
  }
  .seg .sel {
    background: var(--panel-2);
  }
  .dpad {
    flex: 1;
    min-height: 220px;
    max-height: 380px;
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    grid-template-rows: 1fr 1fr 1fr;
    gap: 10px;
  }
  .dpad button {
    font-size: 1.6rem;
  }
  .up {
    grid-area: 1 / 2;
  }
  .left {
    grid-area: 2 / 1;
  }
  .ok {
    grid-area: 2 / 2;
    border-radius: 50%;
    font-size: 1.2rem !important;
  }
  .right {
    grid-area: 2 / 3;
  }
  .down {
    grid-area: 3 / 2;
  }
  .row {
    display: flex;
    gap: 10px;
  }
  .row button {
    flex: 1;
    padding: 14px 4px;
    font-size: 0.85rem;
  }
  .media button {
    font-size: 1.15rem;
    padding: 12px 0;
  }
  .active {
    background: var(--accent);
  }
  .now {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 10px 10px 14px;
    border-radius: 18px;
    background: var(--panel);
  }
  .now .np {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .now b {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 0.95rem;
  }
  .now button {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    padding: 0;
    font-size: 1rem;
    flex: none;
  }
  .pb {
    display: block;
    height: 3px;
    margin-top: 4px;
    border-radius: 99px;
    background: #ffffff22;
    overflow: hidden;
  }
  .pb u {
    display: block;
    height: 100%;
    background: var(--accent);
    text-decoration: none;
  }
  .quit {
    padding: 12px;
    font-size: 0.85rem;
    color: var(--muted);
    background: transparent;
  }
</style>
