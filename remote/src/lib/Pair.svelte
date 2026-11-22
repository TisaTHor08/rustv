<script lang="ts">
  import { BarcodeFormat, BarcodeScanner } from '@capacitor-mlkit/barcode-scanning';
  import { parsePairing, savePairing } from './connection.svelte.ts';
  import { isNative } from './native.ts';

  let error = $state('');
  let manual = $state('');

  function accept(text: string) {
    const p = parsePairing(text);
    if (p) savePairing(p);
    else error = "Ce QR code n'est pas un appairage RusTV.";
  }

  async function scan() {
    error = '';
    try {
      const perm = await BarcodeScanner.requestPermissions();
      if (perm.camera !== 'granted' && perm.camera !== 'limited') {
        error = 'Accès à la caméra refusé.';
        return;
      }
      // Le scanner ML Kit « Google » est un module séparé, installé à la demande.
      const { available } = await BarcodeScanner.isGoogleBarcodeScannerModuleAvailable();
      if (!available) await BarcodeScanner.installGoogleBarcodeScannerModule();
      const { barcodes } = await BarcodeScanner.scan({ formats: [BarcodeFormat.QrCode] });
      if (barcodes[0]?.rawValue) accept(barcodes[0].rawValue);
    } catch (e) {
      error = (e as Error).message || 'Scan annulé.';
    }
  }
</script>

<main>
  <h1>RusTV</h1>
  <p>Sur le PC : icône de la zone de notification → « Ajouter un téléphone », puis scannez le QR code.</p>
  {#if isNative}
    <button class="primary big" onclick={scan}>Scanner le QR code</button>
  {:else}
    <textarea placeholder="Coller le contenu du QR code (mode navigateur)" bind:value={manual} rows="5"></textarea>
    <button class="primary big" onclick={() => accept(manual)}>Valider</button>
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
