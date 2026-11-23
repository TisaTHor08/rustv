<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { api } from '../api';
  import { t } from '../i18n';
  import { play } from '../sounds';
  import Quick from './defaults/Quick.svelte';
  import type { QuickItemView } from './screens';
  import ScreenSlot from './ScreenSlot.svelte';
  import { VOLUME_STEP, clampVolume } from './system';
  import './strings';

  // Panneau de réglages rapides (logique partagée) : volume, sortie audio, écran, téléphones, tous les réglages, quitter le mode TV.
  // Le rendu vient du style actif (meta.screens.quick), sinon de defaults/Quick.svelte. Ouvert par model.openQuick().
  // Réseau (Wi-Fi/Ethernet) et mise en veille du PC : non proposés (pas de commande native sûre et simple, voir docs/OS-STYLES.md).
  interface Output {
    id: string;
    name: string;
    default: boolean;
    volume: number;
    muted: boolean;
  }
  let {
    time,
    date,
    phonesLabel,
    onclose,
    onsettings,
    onpairing,
  }: { time: string; date: string; phonesLabel: string; onclose: () => void; onsettings: (page?: string) => void; onpairing: () => void } = $props();

  let outputs = $state<Output[]>([]);
  let level = $state(0);
  let muted = $state(false);
  let idx = $state(0);

  const cur = $derived(outputs.find((o) => o.default) ?? outputs[0]);

  async function load() {
    try {
      outputs = (await invoke<Output[]>('audio_outputs')) ?? [];
    } catch {
      outputs = [];
    }
    if (cur) {
      level = cur.volume;
      muted = cur.muted;
    }
  }

  async function setLevel(v: number) {
    const o = cur;
    level = clampVolume(v);
    muted = false;
    if (o) invoke('audio_set_output_volume', { id: o.id, level }).catch(() => {});
  }

  async function cycleOutput(dir: -1 | 1) {
    if (outputs.length < 2) return;
    const i = outputs.findIndex((o) => o.default);
    const n = outputs[(i + dir + outputs.length) % outputs.length];
    try {
      outputs = (await invoke<Output[]>('audio_set_default', { id: n.id })) ?? outputs;
      const d = outputs.find((o) => o.default);
      if (d) ((level = d.volume), (muted = d.muted));
    } catch {}
  }

  const items = $derived<QuickItemView[]>([
    { kind: 'slider', id: 'volume', label: t('quick.volume'), level, muted, value: muted ? t('quick.muted') : `${level}` },
    { kind: 'choice', id: 'output', label: t('quick.output'), value: cur?.name ?? t('quick.nooutput') },
    { kind: 'action', id: 'display', label: t('quick.display') },
    { kind: 'action', id: 'phones', label: t('quick.phones'), value: phonesLabel },
    { kind: 'action', id: 'settings', label: t('quick.settings') },
    { kind: 'action', id: 'exit', label: t('quick.exit') },
  ]);

  function adjust(i: number, dir: -1 | 1) {
    const it = items[i];
    if (it.id === 'volume') void setLevel(level + dir * VOLUME_STEP);
    else if (it.id === 'output') void cycleOutput(dir);
  }

  function activate(i: number) {
    const it = items[i];
    play('select');
    if (it.id === 'display') (onclose(), onsettings('display'));
    else if (it.id === 'phones') (onclose(), onpairing());
    else if (it.id === 'settings') (onclose(), onsettings());
    else if (it.id === 'exit') (onclose(), void api.tvExit());
    else if (it.id === 'output') void cycleOutput(1);
  }

  function onKey(e: KeyboardEvent) {
    const before = idx;
    if (e.key === 'ArrowDown') idx = Math.min(items.length - 1, idx + 1);
    else if (e.key === 'ArrowUp') idx = Math.max(0, idx - 1);
    else if (e.key === 'ArrowLeft') adjust(idx, -1);
    else if (e.key === 'ArrowRight') adjust(idx, 1);
    else if (e.key === 'Enter') activate(idx);
    else if (e.key === 'Escape' || e.key === 'Backspace' || e.key === 'ContextMenu') (play('back'), onclose());
    else return;
    if (idx !== before) play('move');
    e.preventDefault();
    e.stopImmediatePropagation();
  }

  onMount(() => {
    void load();
    // Le volume peut aussi changer depuis le téléphone ou les touches Windows pendant que le panneau est ouvert.
    const un = listen<{ level: number; muted: boolean }>('volume-changed', (e) => ((level = e.payload.level), (muted = e.payload.muted)));
    return () => void un.then((f) => f());
  });

  const view = $derived({
    items,
    index: idx,
    time,
    date,
    phonesLabel,
    hint: t('quick.hint'),
    title: t('quick.title'),
    hover: (i: number) => (idx = i),
    activate,
    adjust,
    close: onclose,
  });
</script>

<svelte:window onkeydowncapture={onKey} />

<ScreenSlot name="quick" props={view}>
  {#snippet fallback()}<Quick {...view} />{/snippet}
</ScreenSlot>
