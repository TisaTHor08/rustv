<script lang="ts">
  import { t } from './i18n.ts';
  import { activate, app, deactivate, send } from './connection.svelte.ts';
  import { longpress, repeat, tick } from './haptics.ts';
  import type { KeyName, MediaAction } from './protocol/types.ts';
  import Apps from './Apps.svelte';
  import Surface from './Surface.svelte';
  import Touchpad from './Touchpad.svelte';
  import TextInput from './TextInput.svelte';
  import Vault from './Vault.svelte';
  import Ring from './Ring.svelte';
  import Volume from './Volume.svelte';
  import ModPanels from './ModPanels.svelte';
  import Settings from './Settings.svelte';
  import { applyTheme, findTheme, loadThemeChoice, panelsOf } from './mods.ts';

  type Tab = 'remote' | 'apps' | 'pad' | 'mods';
  let tab = $state<Tab>('remote');
  let altOpen = $state(false);
  let altTimer: number | undefined;
  let manualKeyboard = $state(false);
  let vaultOpen = $state(false);
  let settingsOpen = $state(false);
  let themeChoice = $state(loadThemeChoice());
  const hasPanels = $derived(panelsOf(app.remoteMods).length > 0);

  // Thème choisi : appliqué tant que son mod est actif, sinon retour silencieux au défaut.
  $effect(() => {
    applyTheme(findTheme(app.remoteMods, themeChoice));
  });
  $effect(() => {
    if (tab === 'mods' && !hasPanels) tab = 'remote';
  });
  let mode = $state<'dpad' | 'surface'>(loadMode());

  function loadMode(): 'dpad' | 'surface' {
    try {
      return localStorage.getItem('rustv.mode') === 'surface' ? 'surface' : 'dpad';
    } catch {
      return 'dpad';
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
  // Pendant une reconnexion on garde l'interface (onglet, champ de saisie) au lieu de la remplacer par un spinner.

  // Boutons de volume du téléphone = volume du PC (l'appli Android intercepte les touches quand elle est connectée).
  $effect(() => {
    if (!connected) return;
    const native = (window as any).RusTVNative;
    native?.setVolumeCapture?.(true);
    const onkey = (e: Event) => {
      send({ t: 'media', a: (e as CustomEvent).detail === 'up' ? 'volUp' : 'volDown' });
      tick();
    };
    window.addEventListener('rustv-volkey', onkey);
    return () => {
      window.removeEventListener('rustv-volkey', onkey);
      native?.setVolumeCapture?.(false);
    };
  });
  const showKeyboard = $derived((connected || (app.wasConnected && app.status === 'connecting')) && (app.textFocus || manualKeyboard));
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
    <button class="small" onclick={() => (settingsOpen = true)} aria-label={t('settings.title')}>⚙</button>
  </header>

  {#if !connected && !(app.wasConnected && app.status === 'connecting')}
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
      {#if hasPanels}<button class:sel={tab === 'mods'} onclick={() => (tab = 'mods')}>{t('tab.mods')}</button>{/if}
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
          <Ring onok={ok} {altOpen} />
        {/if}

        {#if altOpen}
          <div class="row">
            <button use:repeat={() => alt('left')}>{t('tab.prev')}</button>
            <button use:repeat={() => alt('right')}>{t('tab.next')}</button>
            <button class="primary" onpointerdown={(e) => { e.preventDefault(); tick(true); ok(); }}>{t('validate')}</button>
          </div>
        {/if}

        <div class="round">
          <button use:longpress={{ onTap: key('back'), onLong: key('backlong') }} aria-label={t('back')}>
            <svg viewBox="0 0 24 24"><path d="M15 6l-6 6 6 6" /></svg><small>{t('back')}</small>
          </button>
          <button onpointerdown={(e) => { e.preventDefault(); tick(true); send({ t: 'home' }); }} aria-label={t('home')}>
            <svg viewBox="0 0 24 24"><path d="M4 11l8-7 8 7v8a1 1 0 01-1 1h-4v-6H9v6H5a1 1 0 01-1-1z" /></svg><small>{t('home')}</small>
          </button>
          <button onpointerdown={(e) => { e.preventDefault(); tick(); send({ t: 'key', k: 'search' }); }} aria-label="Recherche">
            <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="6" /><path d="M16 16l4 4" /></svg><small>{t('search')}</small>
          </button>
          <button onpointerdown={(e) => { e.preventDefault(); tick(); tabSwitch(); }} aria-label={t('tabs')}>
            <svg viewBox="0 0 24 24"><rect x="4" y="6" width="12" height="10" rx="2" /><path d="M8 19h12V9" /></svg><small>{t('tabs')}</small>
          </button>
        </div>

        <Volume />

        <div class="row media">
          <button onpointerdown={media('prev')} aria-label="Précédent">⏮</button>
          <button onpointerdown={media('playPause')} aria-label="Lecture / pause">⏯</button>
          <button onpointerdown={media('next')} aria-label="Suivant">⏭</button>
          <button class:active={manualKeyboard} onpointerdown={(e) => { e.preventDefault(); tick(); manualKeyboard = !manualKeyboard; }} aria-label="Clavier">⌨</button>
          <button onpointerdown={(e) => { e.preventDefault(); tick(); vaultOpen = true; }} aria-label="Bitwarden">🔑</button>
        </div>

        {#if app.tvMode}
          <button class="quit" onpointerdown={(e) => { e.preventDefault(); tick(true); send({ t: 'tv', on: false }); }}>{t('tv.quit')}</button>
        {/if}
      {:else if tab === 'apps'}
        <Apps />
      {:else if tab === 'mods'}
        <ModPanels />
      {:else}
        <Touchpad />
      {/if}
    </section>
  {/if}
</div>

{#if settingsOpen}<Settings choice={themeChoice} onpick={(k) => (themeChoice = k)} onclose={() => (settingsOpen = false)} />{/if}
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
  .round {
    display: flex;
    justify-content: space-around;
    gap: 8px;
  }
  .round button {
    flex: 1;
    max-width: 84px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 10px 0 8px;
    border-radius: 22px;
    background: var(--panel);
  }
  .round svg {
    width: 26px;
    height: 26px;
    fill: none;
    stroke: var(--text);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .round small {
    font-size: 0.7rem;
    color: var(--muted);
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
