<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { t } from '../i18n';
  import { prefs } from '../prefs.svelte';
  import { TILE_COLORS } from '../prefs.svelte';
  import Ambient from './defaults/Ambient.svelte';
  import Osd from './defaults/Osd.svelte';
  import Toast from './defaults/Toast.svelte';
  import { ambientDue, paletteOf } from './system';
  import type { HomeModel } from './model.svelte';
  import ScreenSlot from './ScreenSlot.svelte';
  import './strings';

  // Couche « système » de l'accueil : affichage du volume (OSD), veille ambiante, toast. Le rendu vient du style actif
  // (meta.screens.osd / ambient / toast), sinon des écrans par défaut. Voir docs/OS-STYLES.md.
  //
  // Limite : cette couche vit dans la fenêtre Accueil. Elle s'affiche donc par-dessus l'accueil et ses fenêtres internes, mais
  // pas devant une app web ou le lecteur (autres fenêtres).
  let { model, toast, blocked }: { model: HomeModel; toast: string; blocked: boolean } = $props();

  // ---------- volume ----------
  let vol = $state<{ level: number; muted: boolean } | null>(null);
  let volTimer: number | undefined;
  export function showVolume(level: number, muted: boolean) {
    if (!prefs.volumeOsd) return;
    vol = { level: Math.max(0, Math.min(100, Math.round(level))), muted };
    clearTimeout(volTimer);
    volTimer = window.setTimeout(() => (vol = null), 2000);
  }

  // ---------- veille ambiante ----------
  let ambient = $state(false);
  let last = Date.now();
  let now = $state(Date.now());
  const palette = $derived(paletteOf(model.allTiles.map((x) => x.color), TILE_COLORS.map((c) => c[0])));
  const status = $derived(model.phones > 0 ? model.phonesLabel : t('ambient.phones.none'));

  export function startAmbient() {
    ambient = true;
  }
  const touch = () => (last = Date.now());

  function onInputCapture(e: Event) {
    if (!ambient) return touch();
    // La première touche réveille seulement : elle n'est pas transmise à l'interface.
    if (e.type === 'keydown') {
      e.preventDefault();
      e.stopImmediatePropagation();
    }
    ambient = false;
    touch();
  }

  onMount(() => {
    const evs = ['keydown', 'mousedown', 'mousemove', 'wheel', 'touchstart', 'pointerdown'] as const;
    for (const n of evs) window.addEventListener(n, onInputCapture, { capture: true, passive: n !== 'keydown' });
    const unVol = listen<{ level: number; muted: boolean }>('volume-changed', (e) => {
      showVolume(e.payload.level, e.payload.muted);
      touch(); // un changement de volume (téléphone, touches Windows) compte comme une action
    });
    const id = window.setInterval(() => {
      now = Date.now();
      const focused = document.hasFocus() && document.visibilityState === 'visible';
      if (!focused) touch(); // une app web ou le lecteur est au premier plan : le décompte repart à zéro au retour
      if (ambientDue({ minutes: prefs.ambientMin, idleMs: Date.now() - last, active: ambient, blocked: blocked || !focused })) ambient = true;
    }, 2000);
    return () => {
      for (const n of evs) window.removeEventListener(n, onInputCapture, { capture: true } as EventListenerOptions);
      unVol.then((f) => f());
      clearInterval(id);
      clearTimeout(volTimer);
    };
  });

  // Une fenêtre qui s'ouvre (réglages, lecteur, lancement) coupe la veille.
  $effect(() => {
    if (blocked && ambient) ambient = false;
  });
  void now;
</script>

{#if vol}
  <ScreenSlot name="osd" props={{ level: vol.level, muted: vol.muted, label: vol.muted ? t('osd.muted') : t('osd.volume') }}>
    {#snippet fallback()}<Osd level={vol!.level} muted={vol!.muted} label={vol!.muted ? t('osd.muted') : t('osd.volume')} />{/snippet}
  </ScreenSlot>
{/if}

{#if toast}
  <ScreenSlot name="toast" props={{ message: toast }}>
    {#snippet fallback()}<Toast message={toast} />{/snippet}
  </ScreenSlot>
{/if}

{#if ambient}
  <ScreenSlot name="ambient" props={{ time: model.time, date: model.date, palette, weatherEnabled: model.weatherEnabled, phones: model.phones, status }}>
    {#snippet fallback()}<Ambient time={model.time} date={model.date} {palette} weatherEnabled={model.weatherEnabled} phones={model.phones} {status} />{/snippet}
  </ScreenSlot>
{/if}
