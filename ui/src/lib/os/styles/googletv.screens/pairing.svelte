<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { PairingScreenProps } from '../../screens';

  // Appairage style Google TV : page plein écran, grand QR sur carte blanche, étapes, liste des téléphones, pilules d'action.
  let { svg, paired, bluetooth, devices, index, embedded, hover, activate, newCode, close }: PairingScreenProps = $props();
</script>

<div class="gp" transition:fade={{ duration: 180 }}>
  <h1>{t('pair.title')}</h1>
  <div class="main">
    <div class="qr" class:ok={paired}>
      {#if svg}{@html svg}{:else}<span class="spin"></span>{/if}
      {#if paired}<div class="done"><b>✓</b><span>{t('pair.done')}</span></div>{/if}
    </div>
    <ol>
      <li>{t('pair.s1')}</li>
      <li>{t('pair.s2')}</li>
      <li>{t('pair.s3')}</li>
      {#if bluetooth}<li class="sub">{t('pair.bt')}</li>{/if}
    </ol>
  </div>
  <h3>{t('pair.paired')}</h3>
  <div class="list">
    {#each devices as d, i (d.id)}
      <button class="dev" class:on={index === i} class:armed={d.armed} onmouseenter={() => hover(i)} onclick={() => activate(i)}>
        <i class:live={d.connected}></i><b>{d.name}</b><span>{d.armed ? t('pair.revoke.confirm') : d.connected ? t('pair.connected') : t('pair.offline')}</span>
      </button>
    {:else}<p>{t('pair.none')}</p>{/each}
  </div>
  <div class="acts">
    <button class="pill" class:on={index === devices.length} onmouseenter={() => hover(devices.length)} onclick={newCode}>{t('pair.new')}</button>
    {#if embedded}<button class="pill" class:on={index === devices.length + 1} onmouseenter={() => hover(devices.length + 1)} onclick={close}>{t('common.close')}</button>{/if}
  </div>
</div>

<style>
  .gp { position: absolute; inset: 0; z-index: 12; padding: 4.2rem 6rem 2rem; box-sizing: border-box; background: radial-gradient(60rem 40rem at 100% 0%, rgba(80, 90, 170, 0.22), transparent 70%), var(--bg); color: var(--text); font-family: var(--font); overflow: hidden; }
  h1 { margin: 0 0 2rem; font-size: 3.2rem; font-weight: 400; }
  .main { display: flex; align-items: center; gap: 4rem; }
  .qr { position: relative; flex: none; width: 19rem; height: 19rem; padding: 1rem; border-radius: 2rem; background: #fff; display: grid; place-items: center; }
  .qr :global(svg) { width: 100%; height: 100%; display: block; }
  .qr.ok :global(svg) { opacity: 0.1; }
  .done { position: absolute; inset: 0; display: grid; place-content: center; text-align: center; color: #0a7d3a; font-size: 1.4rem; font-weight: 500; }
  .done b { font-size: 5rem; line-height: 1; }
  .spin { width: 3.4rem; height: 3.4rem; border: 0.35rem solid #ddd; border-top-color: #4a6cf7; border-radius: 50%; animation: s 0.8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg); } }
  ol { margin: 0; padding-left: 2rem; font-size: 1.7rem; line-height: 1.7; color: var(--text); }
  ol .sub { list-style: none; margin-left: -2rem; font-size: 1.2rem; color: var(--muted); }
  h3 { margin: 2.4rem 0 0.8rem; font-size: 1.15rem; font-weight: 500; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); }
  .list { display: flex; flex-direction: column; gap: 0.4rem; max-width: 60rem; }
  .list p { margin: 0; color: var(--muted); font-size: 1.3rem; }
  button { font-family: inherit; border: 0; color: inherit; cursor: pointer; }
  .dev { display: flex; align-items: center; gap: 1.1rem; padding: 0.9rem 1.6rem; border-radius: 99rem; background: var(--glass); font-size: 1.35rem; text-align: left; }
  .dev i { width: 0.8rem; height: 0.8rem; border-radius: 50%; background: #777; }
  .dev i.live { background: #81c995; }
  .dev span { margin-left: auto; font-size: 1.05rem; opacity: 0.7; }
  .dev.on, .pill.on { background: #e8eaed; color: #1f1f1f; }
  .dev.armed { background: #f28b82; color: #3c0a06; }
  .acts { display: flex; gap: 1rem; margin-top: 1.8rem; }
  .pill { padding: 0.9rem 2.2rem; border-radius: 99rem; background: var(--glass-strong); font-size: 1.4rem; font-weight: 500; }
</style>
