<script lang="ts">
  import { t } from './i18n.ts';
  import { BarcodeFormat, BarcodeScanner } from '@capacitor-mlkit/barcode-scanning';
  import { parsePairing, savePairing } from './connection.svelte.ts';
  import { isNative } from './native.ts';

  let error = $state('');
  let manual = $state('');

  function accept(text: string) {
    const p = parsePairing(text);
    if (p) savePairing(p);
    else error = t('pair.invalid');
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
  <p>{t('pair.help')}</p>
  {#if isNative}
    <button class="primary big" onclick={scan}>{t('pair.scan')}</button>
  {:else}
    <textarea placeholder={t('pair.paste')} bind:value={manual} rows="5"></textarea>
    <button class="primary big" onclick={() => accept(manual)}>{t('pair.ok')}</button>
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
  .err {
    color: var(--warn);
  }
</style>
