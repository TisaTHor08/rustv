<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { PairingScreenProps } from '../../screens';

  // Appairage style Titan : fond indigo sombre, QR sur carte blanche arrondie à gauche, étapes à droite, téléphones sur barre bleue.
  let { svg, paired, bluetooth, devices, index, embedded, hover, activate, newCode, close }: PairingScreenProps = $props();
</script>

<div class="tp" transition:fade={{ duration: 160 }}>
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
  .tp { position: absolute; inset: 0; z-index: 12; padding: 4rem 6rem 2rem; box-sizing: border-box; color: #fff; font-family: var(--font); overflow: hidden; background: radial-gradient(60rem 40rem at 85% 10%, rgba(70, 80, 190, 0.4), transparent 70%), #080818; }
  h1 { margin: 0 0 2rem; font-size: 2.8rem; font-weight: 500; }
  .main { display: flex; align-items: center; gap: 4rem; }
  .qr { position: relative; flex: none; width: 17rem; height: 17rem; padding: 1rem; border-radius: 1.2rem; background: #fff; display: grid; place-items: center; }
  .qr :global(svg) { width: 100%; height: 100%; display: block; }
  .qr.ok :global(svg) { opacity: 0.1; }
  .done { position: absolute; inset: 0; display: grid; place-content: center; text-align: center; color: #0a7d3a; font-size: 1.3rem; font-weight: 600; }
  .done b { font-size: 4.6rem; line-height: 1; }
  .spin { width: 3.2rem; height: 3.2rem; border: 0.35rem solid #ddd; border-top-color: #1d7be5; border-radius: 50%; animation: s 0.8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg); } }
  ol { margin: 0; padding-left: 2rem; font-size: 1.6rem; line-height: 1.7; }
  ol .sub { list-style: none; margin-left: -2rem; font-size: 1.15rem; opacity: 0.65; }
  h3 { margin: 2.2rem 0 0.8rem; font-size: 1.15rem; font-weight: 600; opacity: 0.7; }
  .list { display: flex; flex-direction: column; gap: 0.2rem; max-width: 52rem; }
  .list p { margin: 0; opacity: 0.65; font-size: 1.25rem; }
  .dev { display: flex; align-items: center; justify-content: space-between; padding: 0.8rem 1.4rem; border: 0; border-radius: 0.3rem; background: transparent; color: inherit; font: inherit; font-size: 1.45rem; text-align: left; cursor: pointer; }
  .dev span { font-size: 1.1rem; opacity: 0.85; }
  .dev.on { background: #1d7be5; }
  .dev.armed { background: #b3261e; }
</style>
