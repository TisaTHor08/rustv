<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from './i18n.ts';
  import { activate, app, send, stopByUser } from './connection.svelte.ts';
  import { tick } from './haptics.ts';
  import type { MediaAction } from './protocol/types.ts';
  import Apps from './Apps.svelte';
  import Touchpad from './Touchpad.svelte';
  import TextInput from './TextInput.svelte';
  import Vault from './Vault.svelte';
  import ModPanels from './ModPanels.svelte';
  import Settings from './Settings.svelte';
  import Utility from './remotes/parts/Utility.svelte';
  import { activeModel } from './remotes/select.svelte.ts';
  import type { RemoteCtx } from './remotes/types.ts';
  import { keyboardVisible, nextDismissed } from './protocol/appstate.ts';
  import { applyTheme, findTheme, loadThemeChoice, panelsOf } from './mods.ts';

  type Tab = 'remote' | 'apps' | 'pad' | 'mods';
  const TABS: Tab[] = ['remote', 'apps', 'pad', 'mods'];
  let savedTab: Tab = 'remote';
  try {
    const v = sessionStorage.getItem('rustv.tab') as Tab | null;
    if (v && TABS.includes(v)) savedTab = v;
  } catch {}
  let tab = $state<Tab>(savedTab);
  $effect(() => {
    try {
      sessionStorage.setItem('rustv.tab', tab);
    } catch {}
  });
  // Reconnexion automatique à l'ouverture (ou si Android a rechargé l'appli en arrière-plan), sauf arrêt volontaire.
  onMount(() => {
    let off = false;
    try {
      off = localStorage.getItem('rustv.manualOff') === '1';
    } catch {}
    if (!off && app.status === 'idle') activate();
  });
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

  // Modèle de télécommande affiché : `auto` suit le style d'accueil TV du PC (voir remotes/select.svelte.ts).
  const model = $derived(activeModel());
  let toastMsg = $state('');
  let toastTimer: number | undefined;
  const ctx: RemoteCtx = {
    ok,
    get altOpen() {
      return altOpen;
    },
    toast(msg) {
      toastMsg = msg;
      clearTimeout(toastTimer);
      toastTimer = window.setTimeout(() => (toastMsg = ''), 2600);
    },
    get keyboardOn() {
      return manualKeyboard;
    },
    toggleKeyboard: () => (manualKeyboard = !manualKeyboard),
    openVault: () => (vaultOpen = true),
    tabSwitch,
    alt,
  };

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
  // ✕ ferme la barre pour le champ actuel ; elle revient au prochain champ (après un état « sans champ »).
  let kbDismissed = $state(false);
  $effect(() => {
    kbDismissed = nextDismissed(kbDismissed, app.textFocus);
  });
  // Sans liaison, plus de saisie manuelle non plus.
  $effect(() => {
    if (app.status === 'idle') manualKeyboard = false;
  });
  const showKeyboard = $derived(
    keyboardVisible({ live: connected || (app.wasConnected && app.status === 'connecting'), textFocus: app.textFocus, manual: manualKeyboard, dismissed: kbDismissed }),
  );
</script>

<div class="screen">
  <header>
    <span class="dot" class:on={connected} class:wait={app.status === 'connecting'}></span>
    <div class="title">
      <strong>{app.pairing?.name}</strong>
      <small>
        {#if connected}{app.via}{app.tvMode ? t('mode.tv') : ''}{:else if app.status === 'connecting'}{t('connecting')}{app.error ? ` (${app.error})` : ''}{:else}{t('disconnected')}{app.error ? ` (${app.error})` : ''}{/if}
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
        <button class="small" onclick={stopByUser}>{t('cancel')}</button>
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

        <div class="model">
          {#key model.id}
            <model.component {ctx} />
          {/key}
          {#if !model.ownsUtilities}<Utility {ctx} />{/if}
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

{#if toastMsg}<div class="toast" role="status">{toastMsg}</div>{/if}
{#if settingsOpen}<Settings choice={themeChoice} onpick={(k) => (themeChoice = k)} onclose={() => (settingsOpen = false)} />{/if}
{#if vaultOpen}<Vault onclose={() => (vaultOpen = false)} />{/if}
{#if showKeyboard && !vaultOpen}<TextInput onclose={() => { manualKeyboard = false; kbDismissed = true; }} />{/if}

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
  .model {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    scrollbar-width: none;
  }
  .toast {
    position: fixed;
    left: 50%;
    bottom: max(24px, env(safe-area-inset-bottom));
    transform: translateX(-50%);
    max-width: calc(100% - 32px);
    padding: 10px 16px;
    border-radius: 14px;
    background: var(--panel-2);
    color: var(--text);
    font-size: 0.85rem;
    text-align: center;
    box-shadow: 0 6px 24px #0008;
    z-index: 60;
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
