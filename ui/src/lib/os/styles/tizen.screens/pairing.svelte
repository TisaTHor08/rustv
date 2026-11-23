<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { PairingScreenProps } from '../../screens';

  // Appairage style Tizen : page plein écran bleu nuit, QR sur carte blanche arrondie à gauche, étapes et téléphones dans une carte à droite.
  let { svg, paired, bluetooth, devices, index, embedded, hover, activate, newCode, close }: PairingScreenProps = $props();
</script>

<div class="tp" transition:fade={{ duration: 180 }}>
  <h1>{t('pair.title')}</h1>
  <div class="cols">
    <div class="qr" class:ok={paired}>
      {#if svg}{@html svg}{:else}<span class="spin"></span>{/if}
      {#if paired}<div class="done"><b>&#10003;</b><span>{t('pair.done')}</span></div>{/if}
    </div>
    <section class="card">
      <ol>
        <li>{t('pair.s1')}</li>
        <li>{t('pair.s2')}</li>
        <li>{t('pair.s3')}</li>
        {#if bluetooth}<li class="sub">{t('pair.bt')}</li>{/if}
      </ol>
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
    </section>
  </div>
</div>

<style>
  .tp { position: absolute; inset: 0; z-index: 12; padding: 4rem 7rem 2rem; box-sizing: border-box; color: #f4f5fa; font-family: var(--font); background: radial-gradient(80rem 50rem at 80% 0%, #26285a 0%, transparent 70%), linear-gradient(160deg, #14162f 0%, #0a0b17 70%); overflow: hidden; }
  h1 { margin: 0 0 2.4rem; font-size: 3rem; font-weight: 600; }
  .cols { display: flex; gap: 4rem; align-items: flex-start; }
  .qr { position: relative; flex: none; width: 22rem; height: 22rem; padding: 1.2rem; box-sizing: border-box; border-radius: 2.4rem; background: #fff; display: grid; place-items: center; box-shadow: 0 1.6rem 4rem rgba(0, 0, 0, 0.5); }
  .qr :global(svg) { width: 100%; height: 100%; display: block; }
  .qr.ok :global(svg) { opacity: 0.1; }
  .done { position: absolute; inset: 0; display: grid; place-content: center; text-align: center; color: #0a7d3a; font-size: 1.4rem; font-weight: 500; }
  .done b { font-size: 5rem; line-height: 1; }
  .spin { width: 3.4rem; height: 3.4rem; border: 0.35rem solid #ddd; border-top-color: #3d7bff; border-radius: 50%; animation: s 0.8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg); } }
  .card { flex: 1; min-width: 0; padding: 2rem 2.4rem; border-radius: 2.4rem; background: linear-gradient(160deg, rgba(60, 62, 110, 0.55), rgba(24, 25, 46, 0.85)); box-shadow: inset 0 0 0 0.1rem rgba(255, 255, 255, 0.28); }
  ol { margin: 0; padding-left: 2.2rem; font-size: 1.6rem; line-height: 1.7; }
  ol .sub { list-style: none; margin-left: -2.2rem; font-size: 1.2rem; color: rgba(255, 255, 255, 0.6); }
  h3 { margin: 1.8rem 0 0.6rem; font-size: 1.1rem; font-weight: 500; letter-spacing: 0.08em; text-transform: uppercase; color: rgba(255, 255, 255, 0.6); }
  .list { display: flex; flex-direction: column; gap: 0.4rem; }
  .list p { margin: 0; color: rgba(255, 255, 255, 0.6); font-size: 1.3rem; }
  button { font-family: inherit; border: 0; color: inherit; cursor: pointer; }
  .dev { display: flex; align-items: center; gap: 1.1rem; padding: 0.85rem 1.6rem; border-radius: 1.4rem; background: rgba(255, 255, 255, 0.08); font-size: 1.35rem; text-align: left; }
  .dev i { width: 0.8rem; height: 0.8rem; border-radius: 50%; background: #777; }
  .dev i.live { background: #3ddc84; }
  .dev span { margin-left: auto; font-size: 1.05rem; opacity: 0.7; }
  .dev.on, .pill.on { background: #fff; color: #10131c; }
  .dev.armed { background: #ff8a80; color: #3c0a06; }
  .acts { display: flex; gap: 1rem; margin-top: 1.6rem; }
  .pill { padding: 0.9rem 2.4rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.16); font-size: 1.35rem; font-weight: 500; }
</style>
