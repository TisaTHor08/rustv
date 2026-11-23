<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { PairingScreenProps } from '../../screens';

  // Appairage style Fire TV : fond gris très sombre, QR sur carte blanche à gauche, étapes numérotées dans des pastilles rondes, téléphones en lignes.
  let { svg, paired, bluetooth, devices, index, embedded, hover, activate, newCode, close }: PairingScreenProps = $props();
</script>

<div class="fp" transition:fade={{ duration: 160 }}>
  <h1>{t('pair.title')}</h1>
  <div class="main">
    <div class="qr" class:ok={paired}>
      {#if svg}{@html svg}{:else}<span class="spin"></span>{/if}
      {#if paired}<div class="done"><b>✓</b><span>{t('pair.done')}</span></div>{/if}
    </div>
    <ol>
      <li><i>1</i>{t('pair.s1')}</li>
      <li><i>2</i>{t('pair.s2')}</li>
      <li><i>3</i>{t('pair.s3')}</li>
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
  .fp { position: absolute; inset: 0; z-index: 12; padding: 4.4rem 5.2rem 2rem; box-sizing: border-box; color: #fff; font-family: var(--font); overflow: hidden; background: #171717; }
  h1 { margin: 0 0 2.4rem; font-size: 3rem; font-weight: 500; }
  .main { display: flex; align-items: center; gap: 4.5rem; }
  .qr { position: relative; flex: none; width: 18rem; height: 18rem; padding: 1rem; border-radius: var(--r); background: #fff; display: grid; place-items: center; }
  .qr :global(svg) { width: 100%; height: 100%; display: block; }
  .qr.ok :global(svg) { opacity: 0.1; }
  .done { position: absolute; inset: 0; display: grid; place-content: center; text-align: center; color: #0a7d3a; font-size: 1.4rem; font-weight: 600; }
  .done b { font-size: 5rem; line-height: 1; }
  .spin { width: 3.4rem; height: 3.4rem; border: 0.35rem solid #ddd; border-top-color: #555; border-radius: 50%; animation: s 0.8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg); } }
  ol { margin: 0; padding: 0; list-style: none; font-size: 1.7rem; display: grid; gap: 1.2rem; }
  ol li { display: flex; align-items: center; gap: 1.2rem; }
  ol li i { display: grid; place-items: center; flex: none; width: 3rem; height: 3rem; border-radius: 50%; background: #3a3a3a; font-style: normal; font-size: 1.4rem; font-weight: 500; }
  ol .sub { font-size: 1.2rem; color: rgba(255, 255, 255, 0.6); }
  h3 { margin: 2.6rem 0 0.8rem; font-size: 1.2rem; font-weight: 700; letter-spacing: 0.22em; text-transform: uppercase; color: rgba(255, 255, 255, 0.85); }
  .list { display: flex; flex-direction: column; gap: 0.3rem; max-width: 56rem; padding: 0.4rem; margin: -0.4rem; }
  .list p { margin: 0; color: rgba(255, 255, 255, 0.6); font-size: 1.3rem; }
  .dev { display: flex; align-items: center; justify-content: space-between; min-height: 4.2rem; padding: 0 1.6rem; border: 0; border-radius: var(--r); background: #242424; color: inherit; font: inherit; font-size: 1.5rem; text-align: left; cursor: pointer; }
  .dev b { font-weight: 400; }
  .dev span { font-size: 1.1rem; color: rgba(255, 255, 255, 0.7); }
  .dev.on { background: #333; box-shadow: 0 0 0 0.2rem #fff; }
  .dev.armed { background: #5a2a24; }
</style>
