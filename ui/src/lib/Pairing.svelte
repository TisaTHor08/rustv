<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api, type Device, type PairingStart } from './api';
  import { t } from './i18n';
  import ScreenSlot from './os/ScreenSlot.svelte';

  let { embedded = false, onclose }: { embedded?: boolean; onclose?: () => void } = $props();

  let pairing = $state<PairingStart | null>(null);
  let devices = $state<Device[]>([]);
  let sel = $state(0);
  let armed = $state<string | null>(null); // appareil en attente de confirmation de révocation

  const paired = $derived(pairing ? devices.find((d) => d.id === pairing!.device)?.used === true : false);
  const others = $derived(devices.filter((d) => d.id !== pairing?.device && d.used));
  // éléments sélectionnables au clavier : appareils, nouveau QR, fermer
  const count = $derived(others.length + 1 + (embedded ? 1 : 0));

  async function refresh() {
    devices = (await api.state()).devices;
  }

  async function newCode() {
    if (pairing && !paired) await api.pairingCancel(pairing.device);
    pairing = await api.pairingStart();
    await refresh();
  }

  function activate() {
    if (sel < others.length) {
      const d = others[sel];
      if (armed === d.id) {
        armed = null;
        api.deviceRevoke(d.id);
      } else armed = d.id;
    } else if (sel === others.length) newCode();
    else onclose?.();
  }

  // Props de l'écran de style (voir os/screens.ts).
  const screenProps = $derived({
    svg: pairing?.svg ?? '',
    paired,
    bluetooth: !!pairing?.bluetooth,
    devices: others.map((d) => ({ id: d.id, name: d.name, connected: d.connected, armed: armed === d.id })),
    index: sel,
    embedded,
    hover: (i: number) => (sel = i),
    activate: (i: number) => ((sel = i), activate()),
    newCode: () => void newCode(),
    close: () => onclose?.(),
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') sel = Math.min(count - 1, sel + 1);
    else if (e.key === 'ArrowUp') sel = Math.max(0, sel - 1);
    else if (e.key === 'Enter') activate();
    else if (e.key === 'Escape' || e.key === 'Backspace') {
      if (embedded) onclose?.();
    } else return;
    armed = e.key === 'Enter' ? armed : null;
    e.preventDefault();
  }

  let un: Promise<() => void> | undefined;
  onMount(() => {
    newCode();
    un = api.onDevicesChanged(refresh);
  });
  onDestroy(() => {
    un?.then((f) => f());
    // Un QR non scanné ne doit pas rester valide.
    if (pairing && !paired) api.pairingCancel(pairing.device);
  });
</script>

<svelte:window onkeydown={onKey} />

{#snippet base()}
<div class="wrap" class:embedded>
  <h2>{t('pair.title')}</h2>
  <div class="grid">
    <div class="qr" class:ok={paired}>
      {#if pairing}
        {@html pairing.svg}
        {#if paired}<div class="done"><span>✓</span><b>{t('pair.done')}</b></div>{/if}
      {:else}
        <div class="spin"></div>
      {/if}
    </div>
    <ol>
      <li>{t('pair.s1')}</li>
      <li>{t('pair.s2')}</li>
      <li>{t('pair.s3')}</li>
      {#if pairing?.bluetooth}<li class="sub">{t('pair.bt')}</li>{/if}
    </ol>
  </div>

  <h3>{t('pair.paired')}</h3>
  <div class="list">
    {#each others as d, i (d.id)}
      <div class="dev" class:sel={sel === i} class:armed={armed === d.id}>
        <span class="dot" class:on={d.connected}></span>
        <span class="n">{d.name}</span>
        <span class="s">{armed === d.id ? t('pair.revoke.confirm') : d.connected ? t('pair.connected') : t('pair.offline')}</span>
        <button onclick={() => { sel = i; armed = d.id; activate(); }}>{t('pair.revoke')}</button>
      </div>
    {:else}
      <p class="empty">{t('pair.none')}</p>
    {/each}
  </div>

  <div class="actions">
    <button class="act" class:sel={sel === others.length} onclick={newCode}>{t('pair.new')}</button>
    {#if embedded}<button class="act" class:sel={sel === others.length + 1} onclick={() => onclose?.()}>{t('common.close')}</button>{/if}
  </div>
</div>
{/snippet}

<!-- Fenêtre d'appairage du bureau : toujours l'écran RusTV. En mode TV (intégré aux Réglages) : écran du style s'il existe. -->
{#if embedded}
  <ScreenSlot name="pairing" props={screenProps}>
    {#snippet fallback()}{@render base()}{/snippet}
  </ScreenSlot>
{:else}
  {@render base()}
{/if}

<style>
  .wrap { padding: 1.4rem 1.6rem; color: var(--text); background: var(--bg); min-height: 100%; }
  .wrap.embedded { padding: 0; background: transparent; min-height: 0; }
  h2 { margin: 0 0 1rem; font-size: 1.7rem; }
  h3 { margin: 1.4rem 0 0.6rem; font-size: 1.1rem; color: var(--muted); font-weight: 600; text-transform: uppercase; letter-spacing: 0.1em; }
  .grid { display: flex; gap: 1.6rem; align-items: center; }
  .qr { position: relative; flex: none; width: 15rem; height: 15rem; background: #fff; border-radius: 1rem; padding: 0.5rem; display: grid; place-items: center; }
  .qr :global(svg) { width: 100%; height: 100%; display: block; }
  .qr.ok :global(svg) { opacity: 0.12; }
  .done { position: absolute; inset: 0; display: grid; place-content: center; text-align: center; color: #0a7d3a; }
  .done span { font-size: 4rem; line-height: 1; }
  .spin { width: 3rem; height: 3rem; border: 0.3rem solid #ddd; border-top-color: #f74c00; border-radius: 50%; animation: s 0.8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg); } }
  ol { margin: 0; padding-left: 1.3rem; line-height: 1.6; font-size: 1.1rem; }
  ol .sub { list-style: none; margin-left: -1.3rem; color: var(--muted); font-size: 0.95rem; margin-top: 0.4rem; }
  .list { display: flex; flex-direction: column; gap: 0.4rem; }
  .dev { display: flex; align-items: center; gap: 0.8rem; padding: 0.6rem 0.9rem; border-radius: 0.8rem; background: rgba(255, 255, 255, 0.06); font-size: 1.05rem; }
  .dev.sel { background: #fff; color: #111; }
  .dev.armed { background: #ffd9c9; color: #7a2000; }
  .dev .n { font-weight: 600; }
  .dev .s { flex: 1; color: inherit; opacity: 0.65; font-size: 0.9rem; }
  .dot { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: #777; }
  .dot.on { background: #3ddc84; }
  .empty { color: var(--muted); margin: 0; font-size: 1rem; }
  button { font: inherit; border: 0; border-radius: 0.6rem; padding: 0.4rem 0.9rem; background: rgba(255, 255, 255, 0.12); color: inherit; cursor: pointer; }
  .actions { display: flex; gap: 0.8rem; margin-top: 1.2rem; }
  .act { padding: 0.7rem 1.4rem; font-size: 1.1rem; font-weight: 600; background: rgba(255, 255, 255, 0.12); }
  .act.sel { background: #fff; color: #111; }
</style>
