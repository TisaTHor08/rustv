<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { PairingScreenProps } from '../../screens';

  // Appairage style Roku : fond violet, QR sur carte blanche, étapes à droite, téléphones en lignes qui passent au blanc quand ciblées.
  let { svg, paired, bluetooth, devices, index, embedded, hover, activate, newCode, close }: PairingScreenProps = $props();
</script>

<div class="rp" transition:fade={{ duration: 160 }}>
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
        <b>{d.name}</b><span>{d.armed ? t('pair.revoke.confirm') : d.connected ? t('pair.connected') : t('pair.offline')}</span>
      </button>
    {:else}<p>{t('pair.none')}</p>{/each}
    <button class="dev" class:on={index === devices.length} onmouseenter={() => hover(devices.length)} onclick={newCode}><b>{t('pair.new')}</b></button>
    {#if embedded}<button class="dev" class:on={index === devices.length + 1} onmouseenter={() => hover(devices.length + 1)} onclick={close}><b>{t('common.close')}</b></button>{/if}
  </div>
</div>

<style>
  .rp { position: absolute; inset: 0; z-index: 12; padding: 4rem 6rem 2rem; box-sizing: border-box; color: #fff; font-family: var(--font); overflow: hidden; background: radial-gradient(70rem 50rem at 10% 0%, #662d91 0%, transparent 70%), linear-gradient(160deg, #3b1259 0%, #1b0a2c 55%, #0b0412 100%); }
  h1 { margin: 0 0 2rem; font-size: 3rem; font-weight: 600; }
  .main { display: flex; align-items: center; gap: 4rem; }
  .qr { position: relative; flex: none; width: 18rem; height: 18rem; padding: 1rem; border-radius: 0.6rem; background: #fff; display: grid; place-items: center; }
  .qr :global(svg) { width: 100%; height: 100%; display: block; }
  .qr.ok :global(svg) { opacity: 0.1; }
  .done { position: absolute; inset: 0; display: grid; place-content: center; text-align: center; color: #0a7d3a; font-size: 1.4rem; font-weight: 600; }
  .done b { font-size: 5rem; line-height: 1; }
  .spin { width: 3.4rem; height: 3.4rem; border: 0.35rem solid #ddd; border-top-color: #662d91; border-radius: 50%; animation: s 0.8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg); } }
  ol { margin: 0; padding-left: 2rem; font-size: 1.7rem; font-weight: 500; line-height: 1.7; }
  ol .sub { list-style: none; margin-left: -2rem; font-size: 1.2rem; opacity: 0.7; }
  h3 { margin: 2.2rem 0 0.8rem; font-size: 1.2rem; font-weight: 700; opacity: 0.75; }
  .list { display: flex; flex-direction: column; gap: 0.2rem; max-width: 52rem; }
  .list p { margin: 0; opacity: 0.7; font-size: 1.3rem; }
  .dev { display: flex; align-items: center; justify-content: space-between; padding: 0.8rem 1.4rem; border: 0; border-radius: 0.4rem; background: transparent; color: inherit; font: inherit; font-size: 1.5rem; text-align: left; cursor: pointer; }
  .dev span { font-size: 1.1rem; opacity: 0.8; }
  .dev.on { background: #fff; color: #2b0f45; }
  .dev.armed { background: #ffb4a9; color: #3c0a06; }
</style>
