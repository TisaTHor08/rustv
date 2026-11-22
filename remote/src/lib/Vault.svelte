<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { t } from './i18n.ts';
  import { app, send } from './connection.svelte.ts';
  import { tick } from './haptics.ts';
  import type { VaultFill, VaultItem } from './protocol/types.ts';

  let { onclose }: { onclose: () => void } = $props();

  // Le mot de passe maître ne vit que dans `master`, vidé dès l'envoi.
  let email = $state('');
  let master = $state('');
  let code = $state('');
  let method = $state<0 | 1 | 3>(0);
  let q = $state('');
  let sel = $state<VaultItem | null>(null);
  let msg = $state('');
  let busy = $state(false);
  let timer: number | undefined;
  let lastSeq = app.vault.seq;
  let prev = '';

  const v = $derived(app.vault);
  const needCode = $derived(v.op === 'login' && !v.ok && v.code === 'need2fa');

  onMount(() => send({ t: 'vaultStatus' }));
  onDestroy(() => {
    master = '';
    clearTimeout(timer);
  });

  // Passage à « déverrouillé » : recherche initiale (suggestions d'après la fenêtre au premier plan).
  $effect(() => {
    const s = v.status;
    if (s === 'unlocked' && prev !== 'unlocked') search();
    if (s !== 'unlocked') sel = null;
    prev = s;
  });

  // Résultat d'une opération : message d'erreur, ou fermeture après une saisie réussie.
  $effect(() => {
    if (v.seq === lastSeq) return;
    lastSeq = v.seq;
    busy = false;
    if (v.ok) {
      msg = '';
      if (v.op === 'fill') onclose();
    } else {
      const k = `vault.err.${v.code}`;
      msg = t(k) === k ? t('vault.err.failed') : t(k);
    }
  });

  function search() {
    send({ t: 'vaultSearch', q: q.trim() });
  }
  function onQuery() {
    clearTimeout(timer);
    timer = window.setTimeout(search, 350);
  }
  function unlock() {
    if (!master) return;
    busy = true;
    msg = '';
    send({ t: 'vaultUnlock', password: master });
    master = '';
  }
  function login() {
    if (!master || !email.trim()) return;
    busy = true;
    msg = '';
    const c = code.trim();
    send({ t: 'vaultLogin', email: email.trim(), password: master, code: c || undefined, method: c ? method : undefined });
    master = '';
  }
  function fill(what: VaultFill, enter = false) {
    if (!sel) return;
    if (!app.textFocus) {
      msg = t('vault.needfocus');
      return;
    }
    tick(true);
    busy = true;
    msg = '';
    send({ t: 'vaultFill', id: sel.id, what, enter });
  }
  function lock() {
    send({ t: 'vaultLock' });
    sel = null;
  }
</script>

<div class="sheet">
  <header>
    <strong>🔑 {t('vault.title')}</strong>
    <button onclick={onclose}>✕ {t('vault.close')}</button>
  </header>

  {#if v.status === 'unknown'}
    <p class="muted">{t('vault.checking')}</p>
  {:else if v.status === 'notInstalled'}
    <p>{t('vault.nocli')}</p>
    <code>winget install Bitwarden.CLI</code>
    <button onclick={() => send({ t: 'vaultStatus' })}>{t('vault.recheck')}</button>
  {:else if v.status === 'loggedOut'}
    <p class="muted">{t('vault.loggedout')}</p>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        login();
      }}
      autocomplete="off"
    >
      <input type="email" bind:value={email} placeholder={t('vault.email')} autocomplete="off" autocapitalize="off" spellcheck="false" />
      <input type="password" bind:value={master} placeholder={t('vault.master')} autocomplete="off" autocapitalize="off" spellcheck="false" />
      {#if needCode || code}
        <div class="seg">
          {#each [0, 1, 3] as m}
            <button type="button" class:sel={method === m} onclick={() => (method = m as 0 | 1 | 3)}>{t(`vault.method.${m}`)}</button>
          {/each}
        </div>
      {/if}
      <input inputmode="numeric" bind:value={code} placeholder={t('vault.code')} autocomplete="off" />
      <button class="primary" type="submit" disabled={busy}>{t('vault.login')}</button>
    </form>
    <p class="muted small">{t('vault.masternote')}</p>
  {:else if v.status === 'locked'}
    <p class="muted">{t('vault.locked')} {v.email}</p>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        unlock();
      }}
      autocomplete="off"
    >
      <input type="password" bind:value={master} placeholder={t('vault.master')} autocomplete="off" autocapitalize="off" spellcheck="false" />
      <button class="primary" type="submit" disabled={busy}>{t('vault.unlock')}</button>
    </form>
    <p class="muted small">{t('vault.masternote')}</p>
  {:else}
    <div class="searchrow">
      <input bind:value={q} oninput={onQuery} placeholder={t('vault.search')} autocomplete="off" autocapitalize="off" spellcheck="false" />
      <button onclick={lock}>{t('vault.lock')}</button>
    </div>
    <div class="list">
      {#each v.items as it (it.id)}
        <button class="item" class:sel={sel?.id === it.id} onclick={() => (sel = it)}>
          <b>{it.name}</b>
          <small>{it.username}{it.host ? ` · ${it.host}` : ''}</small>
        </button>
      {:else}
        <p class="muted">{t('vault.none')}</p>
      {/each}
    </div>
    {#if sel}
      {#if !app.textFocus}<p class="warn">{t('vault.needfocus')}</p>{/if}
      <div class="row">
        <button class="primary" disabled={!app.textFocus || busy} onclick={() => fill('both')}>{t('vault.fill')}</button>
        <button class="primary" disabled={!app.textFocus || busy} onclick={() => fill('both', true)}>{t('vault.fillenter')}</button>
      </div>
      <div class="row">
        <button disabled={!app.textFocus || busy} onclick={() => fill('username')}>{t('vault.user')}</button>
        <button disabled={!app.textFocus || busy} onclick={() => fill('password')}>{t('vault.pass')}</button>
        {#if sel.hasTotp}<button disabled={!app.textFocus || busy} onclick={() => fill('totp')}>{t('vault.totp')}</button>{/if}
      </div>
    {/if}
  {/if}
  {#if msg}<p class="warn">{msg}</p>{/if}
</div>

<style>
  .sheet {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: max(12px, env(safe-area-inset-top)) 16px max(16px, env(safe-area-inset-bottom));
    background: var(--bg, #111);
    overflow-y: auto;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  header button {
    padding: 8px 14px;
    border-radius: 12px;
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  input {
    background: var(--panel-2);
    color: var(--text);
    border: 0;
    border-radius: 12px;
    padding: 14px;
    font: inherit;
    min-width: 0;
  }
  .searchrow {
    display: flex;
    gap: 8px;
  }
  .searchrow input {
    flex: 1;
  }
  .list {
    flex: 1;
    min-height: 120px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    text-align: left;
    padding: 10px 14px;
    border-radius: 12px;
    background: var(--panel);
  }
  .item.sel {
    background: var(--accent);
  }
  .item small {
    color: var(--muted);
  }
  .item.sel small {
    color: inherit;
    opacity: 0.85;
  }
  .row {
    display: flex;
    gap: 10px;
  }
  .row button {
    flex: 1;
    padding: 14px 4px;
  }
  button:disabled {
    opacity: 0.4;
  }
  .seg {
    display: flex;
    gap: 6px;
  }
  .seg button {
    flex: 1;
    padding: 8px 0;
    border-radius: 10px;
  }
  .seg .sel {
    background: var(--accent);
  }
  .muted {
    color: var(--muted);
  }
  .small {
    font-size: 0.8rem;
  }
  .warn {
    color: var(--warn);
  }
  code {
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--panel-2);
    user-select: all;
  }
</style>
