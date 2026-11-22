<script lang="ts">
  import { activate, app, deactivate, send, unpair } from './connection.svelte.ts';
  import { longpress, repeat, tick } from './haptics.ts';
  import Apps from './Apps.svelte';
  import Touchpad from './Touchpad.svelte';
  import TextInput from './TextInput.svelte';

  type Tab = 'remote' | 'apps' | 'pad';
  let tab = $state<Tab>('remote');
  let altOpen = $state(false);
  let altTimer: number | undefined;
  let manualKeyboard = $state(false);

  const key = (k: 'up' | 'down' | 'left' | 'right' | 'enter' | 'back' | 'backlong') => () => send({ t: 'key', k });

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
        {#if connected}{app.via}{:else if app.status === 'connecting'}Connexion…{app.error ? ` (${app.error})` : ''}{:else}Déconnecté{/if}
      </small>
    </div>
    <button class="small" onclick={() => confirm('Oublier ce PC ?') && unpair()}>⚙</button>
  </header>

  {#if !connected}
    <div class="center">
      {#if app.status === 'idle'}
        <button class="primary activate" onclick={activate}>Activer</button>
      {:else}
        <div class="spinner"></div>
        <button class="small" onclick={deactivate}>Annuler</button>
      {/if}
    </div>
  {:else}
    <nav>
      <button class:sel={tab === 'remote'} onclick={() => (tab = 'remote')}>Télécommande</button>
      <button class:sel={tab === 'apps'} onclick={() => (tab = 'apps')}>Apps</button>
      <button class:sel={tab === 'pad'} onclick={() => (tab = 'pad')}>Pavé</button>
    </nav>

    <section>
      {#if tab === 'remote'}
        <div class="dpad">
          <button class="up" use:repeat={key('up')}>▲</button>
          <button class="left" use:repeat={key('left')}>◀</button>
          <button class="ok primary" onpointerdown={(e) => { e.preventDefault(); tick(); ok(); }}>{altOpen ? 'Valider' : 'OK'}</button>
          <button class="right" use:repeat={key('right')}>▶</button>
          <button class="down" use:repeat={key('down')}>▼</button>
        </div>

        {#if altOpen}
          <div class="row">
            <button use:repeat={() => alt('left')}>◀ Onglet</button>
            <button use:repeat={() => alt('right')}>Onglet ▶</button>
          </div>
        {/if}

        <div class="row">
          <button use:longpress={{ onTap: key('back'), onLong: key('backlong') }}>↩ Retour</button>
          <button onpointerdown={(e) => { e.preventDefault(); tick(); tabSwitch(); }}>⇥ Onglets</button>
          <button class:active={app.pinned} onpointerdown={(e) => { e.preventDefault(); tick(); send({ t: 'pin' }); }}>📌 Épingler</button>
          <button class:active={manualKeyboard} onpointerdown={(e) => { e.preventDefault(); tick(); manualKeyboard = !manualKeyboard; }}>⌨ Clavier</button>
        </div>
      {:else if tab === 'apps'}
        <Apps />
      {:else}
        <Touchpad />
      {/if}
    </section>
  {/if}
</div>

{#if showKeyboard}<TextInput onclose={() => (manualKeyboard = false)} />{/if}

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
    gap: 16px;
  }
  .dpad {
    flex: 1;
    max-height: 420px;
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    grid-template-rows: 1fr 1fr 1fr;
    gap: 10px;
    margin-top: 8px;
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
    padding: 16px 4px;
    font-size: 0.85rem;
  }
  .active {
    background: var(--accent);
  }
</style>
