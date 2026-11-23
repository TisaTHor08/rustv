<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { t } from './i18n.ts';
  import { BarcodeFormat, BarcodeScanner } from '@capacitor-mlkit/barcode-scanning';
  import { pairingFromText, savePairing } from './connection.svelte.ts';
  import { isNative } from './native.ts';

  let error = $state('');
  let manual = $state('');

  let scanning = $state(false);
  let video = $state<HTMLVideoElement | null>(null);
  let stream: MediaStream | null = null;
  let raf = 0;
  // La caméra (getUserMedia) n'existe qu'en contexte sécurisé : HTTPS ou localhost.
  const canCamera = !isNative && typeof navigator !== 'undefined' && !!navigator.mediaDevices?.getUserMedia;

  function accept(text: string) {
    const p = pairingFromText(text);
    if (p) {
      stopCamera();
      savePairing(p);
    } else error = t('pair.invalid');
  }

  function stopCamera() {
    cancelAnimationFrame(raf);
    stream?.getTracks().forEach((tr) => tr.stop());
    stream = null;
    scanning = false;
  }
  onDestroy(stopCamera);

  /** Navigateur : lit le QR du PC avec la caméra (décodage JS, chargé à la demande). */
  async function scanWeb() {
    error = '';
    try {
      const { default: jsQR } = await import('jsqr');
      stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' }, audio: false });
      scanning = true;
      await tick();
      if (!video) return stopCamera();
      video.srcObject = stream;
      await video.play();
      const canvas = document.createElement('canvas');
      const g = canvas.getContext('2d', { willReadFrequently: true })!;
      const loop = () => {
        if (!scanning || !video) return;
        if (video.readyState >= 2 && video.videoWidth) {
          canvas.width = video.videoWidth;
          canvas.height = video.videoHeight;
          g.drawImage(video, 0, 0);
          const img = g.getImageData(0, 0, canvas.width, canvas.height);
          const hit = jsQR(img.data, img.width, img.height);
          if (hit?.data) {
            accept(hit.data);
            if (!scanning) return;
          }
        }
        raf = requestAnimationFrame(loop);
      };
      loop();
    } catch {
      stopCamera();
      error = t('pair.cam');
    }
  }

  async function scan() {
    error = '';
    try {
      const perm = await BarcodeScanner.requestPermissions();
      if (perm.camera !== 'granted' && perm.camera !== 'limited') {
        error = t('pair.cam');
        return;
      }
      // Le scanner ML Kit « Google » est un module séparé, installé à la demande.
      const { available } = await BarcodeScanner.isGoogleBarcodeScannerModuleAvailable();
      if (!available) await BarcodeScanner.installGoogleBarcodeScannerModule();
      const { barcodes } = await BarcodeScanner.scan({ formats: [BarcodeFormat.QrCode] });
      if (barcodes[0]?.rawValue) accept(barcodes[0].rawValue);
    } catch (e) {
      error = (e as Error).message || t('pair.cancelled');
    }
  }
</script>

<main>
  <h1>RusTV</h1>
  <p>{isNative ? t('pair.help') : t('pair.helpweb')}</p>
  {#if isNative}
    <button class="primary big" onclick={scan}>{t('pair.scan')}</button>
  {:else}
    {#if scanning}
      <!-- svelte-ignore a11y_media_has_caption -->
      <video bind:this={video} playsinline muted></video>
      <button onclick={stopCamera}>{t('cancel')}</button>
    {:else}
      {#if canCamera}
        <button class="primary big" onclick={scanWeb}>{t('pair.scan')}</button>
      {:else}
        <p class="note">{t('pair.nocam')}</p>
      {/if}
      <textarea placeholder={t('pair.paste')} bind:value={manual} rows="4"></textarea>
      <button class="primary big" onclick={() => accept(manual)}>{t('pair.ok')}</button>
    {/if}
  {/if}
  {#if error}<p class="err">{error}</p>{/if}
</main>

<style>
  main {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 20px;
    padding: 24px 16px;
    text-align: center;
  }
  h1 {
    margin: 0;
    font-size: 2rem;
  }
  p {
    color: var(--muted);
    margin: 0;
    line-height: 1.5;
  }
  .big {
    padding: 20px;
    font-size: 1.15rem;
  }
  textarea {
    background: var(--panel);
    color: var(--text);
    border: 0;
    border-radius: 12px;
    padding: 12px;
  }
  video {
    width: 100%;
    max-height: 50vh;
    border-radius: 12px;
    background: #000;
    object-fit: cover;
  }
  .note {
    font-size: 0.85rem;
  }
  .err {
    color: var(--warn);
  }
</style>
