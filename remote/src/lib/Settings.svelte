<script lang="ts">
  import { t } from './i18n.ts';
  import { app, unpair } from './connection.svelte.ts';
  import { lang } from './i18n.ts';
  import { MODELS } from './remotes/registry.ts';
  import { modelPref, saveModelChoice } from './remotes/select.svelte.ts';
  import { label, modName, saveThemeChoice, themeKey } from './mods.ts';

  let { choice, onpick, onclose }: { choice: string; onpick: (key: string) => void; onclose: () => void } = $props();

  const items = $derived(app.remoteMods.flatMap((m) => (m.themes ?? []).map((th) => ({ key: themeKey(m, th), text: `${label(th.name)} (${modName(m)})` }))));

  function pick(k: string) {
    saveThemeChoice(k);
    onpick(k);
  }
</script>

<div class="back" role="presentation" onclick={onclose}>
  <div class="sheet" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
    <h3>{t('settings.title')}</h3>
    <small class="cap">{t('settings.model')}</small>
    <div class="list">
      <button class:sel={modelPref.choice === 'auto'} onclick={() => saveModelChoice('auto')}>{t('settings.auto')}</button>
      {#each MODELS as m (m.id)}
        <button class:sel={modelPref.choice === m.id} onclick={() => saveModelChoice(m.id)}>{m.name[lang]}</button>
      {/each}
    </div>
    <small class="cap">{t('settings.theme')}</small>
    <div class="list">
      <button class:sel={choice === ''} onclick={() => pick('')}>{t('settings.default')}</button>
      {#each items as it (it.key)}
        <button class:sel={choice === it.key} onclick={() => pick(it.key)}>{it.text}</button>
      {/each}
    </div>
    <button class="forget" onclick={() => confirm(t('forget')) && (onclose(), unpair())}>{t('settings.forget')}</button>
    <button class="close" onclick={onclose}>{t('settings.close')}</button>
  </div>
</div>

<style>
  .back {
    position: fixed;
    inset: 0;
    background: #000a;
    display: flex;
    align-items: flex-end;
    z-index: 50;
  }
  .sheet {
    width: 100%;
    max-height: 80%;
    overflow-y: auto;
    background: var(--bg);
    border-radius: 22px 22px 0 0;
    padding: 16px 16px max(16px, env(safe-area-inset-bottom));
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h3 {
    margin: 0;
  }
  .cap {
    color: var(--muted);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .list button,
  .forget,
  .close {
    padding: 12px;
    border-radius: 12px;
    text-align: left;
    background: var(--panel);
  }
  .list .sel {
    background: var(--accent);
  }
  .forget {
    color: var(--warn);
  }
  .close {
    text-align: center;
    background: var(--panel-2);
  }
</style>
