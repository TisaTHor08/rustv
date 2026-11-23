// Page de réglages « Écran et son » : sortie audio, volume, résolution / fréquence de l'écran de la TV, HDR.
import { invoke } from '@tauri-apps/api/core';
import { t } from '../i18n';
import './display.strings';

type SettingsRow = {
  label: string;
  value?: string;
  swatch?: string;
  left?: () => void | Promise<void>;
  right?: () => void | Promise<void>;
  enter?: () => void | Promise<void>;
  arrows?: boolean;
};

export interface Output {
  id: string;
  name: string;
  default: boolean;
  volume: number;
  muted: boolean;
}
export interface Mode {
  width: number;
  height: number;
  refresh: number;
}
export interface Monitor {
  index: number;
  name: string;
  width: number;
  height: number;
  refresh: number;
  hdr: { supported: boolean; enabled: boolean };
  modes: Mode[];
}
interface Info {
  monitors: Monitor[];
  tvMonitor: number;
  pendingRevertSeconds: number | null;
}

/** Tailles distinctes (déjà triées de la plus grande à la plus petite). */
export function sizesOf(modes: Mode[]): { width: number; height: number }[] {
  const out: { width: number; height: number }[] = [];
  for (const m of modes) if (!out.some((s) => s.width === m.width && s.height === m.height)) out.push({ width: m.width, height: m.height });
  return out;
}
/** Fréquences d'une taille (de la plus haute à la plus basse). */
export function ratesOf(modes: Mode[], w: number, h: number): number[] {
  return modes.filter((m) => m.width === w && m.height === h).map((m) => m.refresh);
}
/** Fréquence la plus proche de `want` parmi `rates` (la plus haute en cas d'égalité). */
export function nearest(rates: number[], want: number): number {
  return rates.reduce((best, r) => (Math.abs(r - want) < Math.abs(best - want) ? r : best), rates[0]);
}
export const modeLabel = (m: Mode) => `${m.width} x ${m.height} @ ${m.refresh} Hz`;
const wrap = (i: number, d: number, n: number) => (i + d + n) % n;

export function createPage(ctx: { note: (m: string) => void }) {
  let outputs = $state<Output[]>([]);
  let sel = $state(0); // sortie choisie (pas encore forcément appliquée)
  let info = $state<Info | null>(null);
  let want = $state<Mode | null>(null);
  let countdown = $state<number | null>(null);
  let timer: ReturnType<typeof setInterval> | null = null;

  const mon = () => info?.monitors[info.tvMonitor] ?? info?.monitors[0] ?? null;

  function stopTimer() {
    if (timer) clearInterval(timer);
    timer = null;
  }
  function startTimer(secs: number) {
    stopTimer();
    countdown = secs;
    timer = setInterval(() => {
      if (countdown === null) return stopTimer();
      countdown -= 1;
      if (countdown <= 0) {
        // Rust a annulé tout seul : on relit l'état.
        stopTimer();
        countdown = null;
        setTimeout(() => load().catch(() => {}), 500);
        ctx.note(t('display.reverted'));
      }
    }, 1000);
  }

  async function safe(fn: () => Promise<void>) {
    try {
      await fn();
    } catch {
      ctx.note(t('display.err'));
    }
  }

  async function load() {
    try {
      outputs = await invoke<Output[]>('audio_outputs');
      const d = outputs.findIndex((o) => o.default);
      sel = d >= 0 ? d : 0;
    } catch {
      outputs = [];
    }
    try {
      info = await invoke<Info>('display_info');
      const m = mon();
      want = m ? { width: m.width, height: m.height, refresh: m.refresh } : null;
      if (info.pendingRevertSeconds) startTimer(info.pendingRevertSeconds);
      else {
        stopTimer();
        countdown = null;
      }
    } catch {
      info = null;
    }
  }

  const out = () => outputs[sel];

  async function applyAudio() {
    const o = out();
    if (!o || o.default) return;
    await safe(async () => {
      outputs = await invoke<Output[]>('audio_set_default', { id: o.id });
      sel = Math.max(0, outputs.findIndex((x) => x.id === o.id));
      ctx.note(t('display.applied'));
    });
  }

  async function volume(d: number) {
    const o = out();
    if (!o) return;
    const level = Math.max(0, Math.min(100, o.volume + d));
    await safe(async () => {
      await invoke('audio_set_output_volume', { id: o.id, level });
      o.volume = level;
      o.muted = false;
    });
  }

  function pickSize(d: number) {
    const m = mon();
    if (!m || !want) return;
    const sizes = sizesOf(m.modes);
    if (!sizes.length) return;
    const i = sizes.findIndex((s) => s.width === want!.width && s.height === want!.height);
    const s = sizes[wrap(i < 0 ? 0 : i, d, sizes.length)];
    want = { ...s, refresh: nearest(ratesOf(m.modes, s.width, s.height), want.refresh) };
  }
  function pickRate(d: number) {
    const m = mon();
    if (!m || !want) return;
    const rates = ratesOf(m.modes, want.width, want.height);
    if (!rates.length) return;
    const i = rates.indexOf(want.refresh);
    want = { ...want, refresh: rates[wrap(i < 0 ? 0 : i, d, rates.length)] };
  }

  const changed = () => {
    const m = mon();
    return !!(m && want && (want.width !== m.width || want.height !== m.height || want.refresh !== m.refresh));
  };

  async function applyMode() {
    const m = mon();
    if (!m || !want || !changed()) return;
    await safe(async () => {
      const r = await invoke<{ pendingRevertSeconds: number }>('display_set_mode', { index: m.index, width: want!.width, height: want!.height, refresh: want!.refresh });
      await load();
      if (r.pendingRevertSeconds > 0 && countdown === null) startTimer(r.pendingRevertSeconds);
    });
  }

  async function keep() {
    await safe(async () => {
      await invoke('display_confirm');
      stopTimer();
      countdown = null;
      ctx.note(t('display.kept'));
      await load();
    });
  }
  async function revert() {
    await safe(async () => {
      await invoke('display_revert');
      stopTimer();
      countdown = null;
      ctx.note(t('display.reverted'));
      await load();
    });
  }

  async function toggleHdr() {
    const m = mon();
    if (!m || !m.hdr.supported) return;
    await safe(async () => {
      await invoke('display_set_hdr', { index: m.index, on: !m.hdr.enabled });
      await load();
    });
  }

  return {
    titleKey: 'display.title',
    load,
    rows(): SettingsRow[] {
      const rows: SettingsRow[] = [];
      // Pendant l'attente de confirmation, cette ligne passe en tête : Entrée garde, gauche annule.
      if (countdown !== null) {
        rows.push({ label: t('display.keep', { n: countdown }), value: t('display.keep.hint'), arrows: true, enter: keep, left: revert, right: keep });
      }
      const o = out();
      rows.push({
        label: t('display.audio'),
        value: o ? o.name + (o.default ? ' *' : '') : t('display.none'),
        arrows: true,
        left: () => {
          if (outputs.length) sel = wrap(sel, -1, outputs.length);
        },
        right: () => {
          if (outputs.length) sel = wrap(sel, 1, outputs.length);
        },
        enter: applyAudio,
      });
      rows.push({
        label: t('display.volume'),
        value: o ? `${o.volume} %${o.muted ? ' (' + t('display.muted') + ')' : ''}` : t('display.none'),
        arrows: true,
        left: () => volume(-5),
        right: () => volume(5),
        enter: () => volume(5),
      });
      const m = mon();
      if (m && want) {
        rows.push({ label: t('display.res'), value: `${want.width} x ${want.height}`, arrows: true, left: () => pickSize(-1), right: () => pickSize(1), enter: applyMode });
        rows.push({ label: t('display.refresh'), value: t('display.hz', { n: want.refresh }), arrows: true, left: () => pickRate(-1), right: () => pickRate(1), enter: applyMode });
        if (changed() && countdown === null) rows.push({ label: t('display.apply', { mode: modeLabel(want) }), enter: applyMode });
        rows.push({
          label: t('display.hdr'),
          value: !m.hdr.supported ? t('display.hdr.none') : m.hdr.enabled ? t('display.hdr.on') : t('display.hdr.off'),
          arrows: true,
          left: toggleHdr,
          right: toggleHdr,
          enter: toggleHdr,
        });
      }
      rows.push({ label: t('display.reload'), enter: load });
      return rows;
    },
  };
}
