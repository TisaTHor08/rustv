<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { PairingScreenProps } from '../../screens';

  // Appairage style webOS : fond quasi noir, QR sur carte blanche à coins peu arrondis, étapes numérotées, téléphones en lignes
  // rectangulaires avec contour blanc épais sur la ligne ciblée.
  let { svg, paired, bluetooth, devices, index, embedded, hover, activate, newCode, close }: PairingScreenProps = $props();
</script>

<div class="wp" transition:fade={{ duration: 180 }}>
  <h1>{t('pair.title')}</h1>
  <div class="cols">
    <div class="qr" class:ok={paired}>
      {#if svg}{@html svg}{:else}<span class="spin"></span>{/if}
      {#if paired}<div class="done"><b>&#10003;</b><span>{t('pair.done')}</span></div>{/if}
    </div>
    <section>
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
        <button class="btn" class:on={index === devices.length} onmouseenter={() => hover(devices.length)} onclick={newCode}>{t('pair.new')}</button>
        {#if embedded}<button class="btn" class:on={index === devices.length + 1} onmouseenter={() => hover(devices.length + 1)} onclick={close}>{t('common.close')}</button>{/if}
      </div>
    </section>
  </div>
</div>

<style>
  .wp { position: absolute; inset: 0; z-index: 12; padding: 4rem 8rem 2rem; box-sizing: border-box; color: #f4f7fb; font-family: var(--font); background: radial-gradient(60rem 40rem at 90% 100%, #1b2347 0%, transparent 70%), #08090d; overflow: hidden; }
  h1 { margin: 0 0 2.4rem; font-size: 2.8rem; font-weight: 500; }
  .cols { display: flex; gap: 4rem; align-items: flex-start; }
  .qr { position: relative; flex: none; width: 21rem; height: 21rem; padding: 1.2rem; box-sizing: border-box; border-radius: 0.8rem; background: #fff; display: grid; place-items: center; }
  .qr :global(svg) { width: 100%; height: 100%; display: block; }
  .qr.ok :global(svg) { opacity: 0.1; }
  .done { position: absolute; inset: 0; display: grid; place-content: center; text-align: center; color: #0a7d3a; font-size: 1.4rem; font-weight: 500; }
  .done b { font-size: 5rem; line-height: 1; }
  .spin { width: 3.4rem; height: 3.4rem; border: 0.35rem solid #ddd; border-top-color: #3a8cf0; border-radius: 50%; animation: s 0.8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg); } }
  section { flex: 1; min-width: 0; }
  ol { margin: 0; padding-left: 2.2rem; font-size: 1.6rem; line-height: 1.7; }
  ol .sub { list-style: none; margin-left: -2.2rem; font-size: 1.2rem; color: rgba(255, 255, 255, 0.6); }
  h3 { margin: 1.8rem 0 0.6rem; font-size: 1.1rem; font-weight: 500; letter-spacing: 0.08em; text-transform: uppercase; color: rgba(255, 255, 255, 0.55); }
  .list { display: flex; flex-direction: column; gap: 0.5rem; padding: 0.3rem; }
  .list p { margin: 0; color: rgba(255, 255, 255, 0.6); font-size: 1.3rem; }
  button { font-family: inherit; border: 0; color: inherit; cursor: pointer; }
  .dev { display: flex; align-items: center; gap: 1.1rem; padding: 0.9rem 1.6rem; border-radius: 0.8rem; background: #1d2027; font-size: 1.35rem; text-align: left; transition: box-shadow 0.15s; }
  .dev i { width: 0.8rem; height: 0.8rem; border-radius: 50%; background: #777; }
  .dev i.live { background: #3ddc84; }
  .dev span { margin-left: auto; font-size: 1.05rem; opacity: 0.7; }
  .dev.on, .btn.on { box-shadow: 0 0 0 0.26rem #fff; background: #2a2e38; }
  .dev.armed { background: #7a2a28; }
  .acts { display: flex; gap: 1rem; margin-top: 1.6rem; padding: 0.3rem; }
  .btn { padding: 0.9rem 2.4rem; border-radius: 0.7rem; background: #262a35; font-size: 1.35rem; }
</style>
