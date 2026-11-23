<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import type { PairingScreenProps } from '../../screens';

  // Appairage style tvOS (écran « Configurer ») : tout centré, QR sur carte blanche aux grands angles, étapes en dessous, téléphones en lignes
  // translucides dont la ciblée devient blanche.
  let { svg, paired, bluetooth, devices, index, embedded, hover, activate, newCode, close }: PairingScreenProps = $props();
</script>

<div class="tp" transition:fade={{ duration: 220 }}>
  <h1>{t('pair.title')}</h1>
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
  <div class="list">
    {#each devices as d, i (d.id)}
      <button class="dev" class:on={index === i} class:armed={d.armed} onmouseenter={() => hover(i)} onclick={() => activate(i)}>
        <b>{d.name}</b><span>{d.armed ? t('pair.revoke.confirm') : d.connected ? t('pair.connected') : t('pair.offline')}</span>
      </button>
    {/each}
    <button class="dev" class:on={index === devices.length} onmouseenter={() => hover(devices.length)} onclick={newCode}><b>{t('pair.new')}</b></button>
    {#if embedded}<button class="dev" class:on={index === devices.length + 1} onmouseenter={() => hover(devices.length + 1)} onclick={close}><b>{t('common.close')}</b></button>{/if}
  </div>
</div>

<style>
  .tp { position: absolute; inset: 0; z-index: 12; display: flex; flex-direction: column; align-items: center; padding: 3.4rem 0 1.6rem; box-sizing: border-box; color: #fff; font-family: var(--font); overflow: hidden; background: radial-gradient(90rem 50rem at 50% 0%, #2b2d3f, transparent 70%), #08080a; }
  h1 { margin: 0 0 1.8rem; font-size: 3.2rem; font-weight: 600; letter-spacing: -0.01em; }
  .qr { position: relative; width: 17rem; height: 17rem; padding: 1.2rem; box-sizing: border-box; border-radius: 2.2rem; background: #fff; display: grid; place-items: center; box-shadow: 0 1.4rem 3.4rem rgba(0, 0, 0, 0.5); }
  .qr :global(svg) { width: 100%; height: 100%; display: block; }
  .qr.ok :global(svg) { opacity: 0.1; }
  .done { position: absolute; inset: 0; display: grid; place-content: center; text-align: center; color: #0a7d3a; font-size: 1.4rem; font-weight: 600; }
  .done b { font-size: 5rem; line-height: 1; }
  .spin { width: 3.4rem; height: 3.4rem; border: 0.35rem solid #ddd; border-top-color: #555; border-radius: 50%; animation: s 0.8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg); } }
  ol { margin: 1.8rem 0 1.4rem; padding: 0; list-style: none; text-align: center; font-size: 1.55rem; line-height: 1.6; color: var(--muted); }
  ol .sub { font-size: 1.2rem; }
  .list { width: 56rem; display: flex; flex-direction: column; gap: 0.4rem; padding: 1rem; margin: -1rem; }
  .dev { display: flex; align-items: center; justify-content: space-between; min-height: 4.8rem; padding: 0 2.2rem; border: 0; border-radius: var(--r); background: rgba(118, 118, 128, 0.24); color: inherit; font: inherit; font-size: 1.55rem; text-align: left; cursor: pointer; transition: transform 0.2s var(--ease), background 0.2s; }
  .dev b { font-weight: 500; }
  .dev span { font-size: 1.15rem; color: var(--muted); }
  .dev.on { background: rgba(255, 255, 255, 0.95); color: #111; transform: scale(1.03); }
  .dev.on span { color: #555; }
  .dev.armed { background: #ff453a; color: #fff; }
</style>
