<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { PairingScreenProps } from '../../screens';

  // Appairage style VIDAA : fond bleu nuit, carte centrale (QR à gauche, étapes à droite), liste des téléphones en dessous.
  let { svg, paired, bluetooth, devices, index, embedded, hover, activate, newCode, close }: PairingScreenProps = $props();
</script>

<div class="vp" transition:fade={{ duration: 160 }}>
  <div class="card">
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
</div>

<style>
  .vp { position: absolute; inset: 0; z-index: 12; display: grid; place-items: center; color: #fff; font-family: var(--font); background: #10121a; overflow: hidden; }
  .card { width: 70rem; max-height: 92%; box-sizing: border-box; padding: 2.6rem 3rem; border-radius: 0.6rem; background: #1b1e2b; overflow: hidden auto; scrollbar-width: none; }
  h1 { margin: 0 0 1.6rem; font-size: 2.4rem; font-weight: 300; letter-spacing: 0.03em; text-align: center; }
  .main { display: flex; align-items: center; gap: 3rem; }
  .qr { position: relative; flex: none; width: 16rem; height: 16rem; padding: 0.8rem; border-radius: 0.4rem; background: #fff; display: grid; place-items: center; }
  .qr :global(svg) { width: 100%; height: 100%; display: block; }
  .qr.ok :global(svg) { opacity: 0.1; }
  .done { position: absolute; inset: 0; display: grid; place-content: center; text-align: center; color: #0a7d3a; font-size: 1.3rem; font-weight: 500; }
  .done b { font-size: 4.4rem; line-height: 1; }
  .spin { width: 3rem; height: 3rem; border: 0.3rem solid #ddd; border-top-color: #2fb5c8; border-radius: 50%; animation: s 0.8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg); } }
  ol { margin: 0; padding-left: 1.8rem; font-size: 1.5rem; line-height: 1.7; }
  ol .sub { list-style: none; margin-left: -1.8rem; font-size: 1.1rem; opacity: 0.65; }
  h3 { margin: 1.8rem 0 0.6rem; font-size: 1.1rem; font-weight: 500; opacity: 0.65; }
  .list { display: flex; flex-direction: column; gap: 0.2rem; }
  .list p { margin: 0; opacity: 0.65; font-size: 1.2rem; }
  .dev { display: flex; align-items: center; justify-content: space-between; padding: 0.75rem 1.2rem; border: 0.2rem solid transparent; border-radius: 0.3rem; background: transparent; color: inherit; font: inherit; font-size: 1.4rem; text-align: left; cursor: pointer; }
  .dev span { font-size: 1.05rem; color: #7fe3f0; }
  .dev.on { border-color: #fff; background: rgba(255, 255, 255, 0.1); }
  .dev.armed { background: rgba(255, 120, 100, 0.25); }
</style>
