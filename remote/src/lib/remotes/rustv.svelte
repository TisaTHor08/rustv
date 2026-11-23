<script lang="ts">
  import { app, send } from '../connection.svelte.ts';
  import { longpress, repeat, tick } from '../haptics.ts';
  import { t } from '../i18n.ts';
  import type { KeyName, MediaAction } from '../protocol/types.ts';
  import Surface from '../Surface.svelte';
  import Ring from '../Ring.svelte';
  import Volume from '../Volume.svelte';
  import type { RemoteCtx } from './types.ts';

  // Modèle par défaut : l'ancienne télécommande RusTV (anneau façon Google TV ou surface tactile, boutons ronds, volume).
  let { ctx }: { ctx: RemoteCtx } = $props();

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
</script>

<div class="seg">
  <button class:sel={mode === 'surface'} onclick={() => setMode('surface')}>{t('seg.surface')}</button>
  <button class:sel={mode === 'dpad'} onclick={() => setMode('dpad')}>{t('seg.dpad')}</button>
</div>

{#if mode === 'surface'}
  <Surface />
{:else}
  <Ring onok={ctx.ok} altOpen={ctx.altOpen} />
{/if}

{#if ctx.altOpen}
  <div class="row">
    <button use:repeat={() => ctx.alt('left')}>{t('tab.prev')}</button>
    <button use:repeat={() => ctx.alt('right')}>{t('tab.next')}</button>
    <button class="primary" onpointerdown={(e) => { e.preventDefault(); tick(true); ctx.ok(); }}>{t('validate')}</button>
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
  <button onpointerdown={(e) => { e.preventDefault(); tick(); ctx.tabSwitch(); }} aria-label={t('tabs')}>
    <svg viewBox="0 0 24 24"><rect x="4" y="6" width="12" height="10" rx="2" /><path d="M8 19h12V9" /></svg><small>{t('tabs')}</small>
  </button>
</div>

<Volume />

<div class="row media">
  <button onpointerdown={media('prev')} aria-label="Précédent">⏮</button>
  <button onpointerdown={media('playPause')} aria-label="Lecture / pause">⏯</button>
  <button onpointerdown={media('next')} aria-label="Suivant">⏭</button>
  <button class:active={ctx.keyboardOn} onpointerdown={(e) => { e.preventDefault(); tick(); ctx.toggleKeyboard(); }} aria-label="Clavier">⌨</button>
  <button onpointerdown={(e) => { e.preventDefault(); tick(); ctx.openVault(); }} aria-label="Bitwarden">🔑</button>
</div>


<style>
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
</style>
