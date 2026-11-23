<script lang="ts">
  import { t } from '../../i18n.ts';
  import { labels } from '../labels.ts';
  import { repeat, tick } from '../../haptics.ts';
  import type { RemoteCtx } from '../types.ts';
  import Icon from './Icon.svelte';

  // Bande d'outils sous les modèles autres que `rustv` : clavier, coffre Bitwarden, onglets (Alt+Tab).
  // Ces fonctions n'existent pas sur les vraies télécommandes mais restent indispensables sur le téléphone.
  let { ctx }: { ctx: RemoteCtx } = $props();
  const L = labels('');
</script>

{#if ctx.altOpen}
  <div class="alt">
    <button use:repeat={() => ctx.alt('left')}>{t('tab.prev')}</button>
    <button use:repeat={() => ctx.alt('right')}>{t('tab.next')}</button>
    <button class="primary" onpointerdown={(e) => { e.preventDefault(); tick(true); ctx.ok(); }}>{t('validate')}</button>
  </div>
{/if}
<div class="util">
  <button class:on={ctx.keyboardOn} aria-label={L('keyboard')} onpointerdown={(e) => { e.preventDefault(); tick(); ctx.toggleKeyboard(); }}><Icon name="kbd" /></button>
  <button aria-label={L('vault')} onpointerdown={(e) => { e.preventDefault(); tick(); ctx.openVault(); }}><Icon name="key" /></button>
  <button aria-label={L('tabs')} onpointerdown={(e) => { e.preventDefault(); tick(); ctx.tabSwitch(); }}><Icon name="tabs" /></button>
</div>

<style>
  .util,
  .alt {
    display: flex;
    gap: 10px;
    justify-content: center;
    flex: none;
  }
  .util button {
    width: 56px;
    height: 44px;
    border-radius: 14px;
    padding: 0;
    display: grid;
    place-items: center;
    background: var(--panel);
    color: var(--muted);
  }
  .util .on {
    background: var(--accent);
    color: #fff;
  }
  .alt button {
    flex: 1;
    padding: 12px 4px;
    font-size: 0.85rem;
  }
</style>
