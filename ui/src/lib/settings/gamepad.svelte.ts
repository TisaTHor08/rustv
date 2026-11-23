import { invoke } from '@tauri-apps/api/core';
import { t } from '../i18n';
import './gamepad.strings';

type SettingsRow = {
  label: string;
  value?: string;
  swatch?: string;
  left?: () => void | Promise<void>;
  right?: () => void | Promise<void>;
  enter?: () => void | Promise<void>;
  arrows?: boolean;
};

interface Cfg {
  enabled: boolean;
  deadzone: number;
  repeatDelayMs: number;
  repeatRateMs: number;
  vibration: number;
  swapAB: boolean;
  pointerMode: boolean;
  onlyWhenTv: boolean;
  allow: string[];
  deny: string[];
}
interface Pad {
  slot: number;
  connected: boolean;
  batteryType: string;
  batteryLevel: string;
}
interface Info {
  config: Cfg;
  pads: Pad[];
  paused: boolean;
  available: boolean;
}

const REPEATS = [250, 150, 90, 60]; // période de répétition en ms : lent -> rapide

export function createPage(ctx: { note: (m: string) => void }) {
  let info = $state<Info | null>(null);

  async function load() {
    try {
      info = await invoke<Info>('gamepad_get');
    } catch {
      info = null;
    }
  }
  async function save(patch: Partial<Cfg>) {
    if (!info) return;
    try {
      info.config = await invoke<Cfg>('gamepad_set', { cfg: { ...info.config, ...patch } });
    } catch (e) {
      ctx.note(String(e));
    }
  }
  const yn = (b: boolean) => t(b ? 'gamepad.on' : 'gamepad.off');
  const toggle = (label: string, key: 'enabled' | 'swapAB' | 'pointerMode' | 'onlyWhenTv'): SettingsRow => {
    const flip = () => save({ [key]: !info!.config[key] });
    return { label, value: yn(info!.config[key]), arrows: true, left: flip, right: flip, enter: flip };
  };
  const step = (cur: number, d: number, max: number) => Math.max(0, Math.min(max, cur + d));

  function rows(): SettingsRow[] {
    if (!info) return [{ label: t('gamepad.unavailable') }];
    const c = info.config;
    const out: SettingsRow[] = [];
    const pads = info.pads.filter((p) => p.connected);
    if (!info.available) out.push({ label: t('gamepad.unavailable') });
    else if (!pads.length) out.push({ label: t('gamepad.none') });
    for (const p of pads) {
      const bat = p.batteryType === 'wired' ? t('gamepad.bat.wired') : t(`gamepad.bat.${p.batteryLevel}`);
      out.push({ label: t('gamepad.pad', { n: String(p.slot + 1) }), value: bat });
    }
    if (info.paused) out.push({ label: t('gamepad.paused') });
    out.push(toggle(t('gamepad.enabled'), 'enabled'));
    const vib = (d: number) => save({ vibration: step(c.vibration, d, 3) });
    out.push({ label: t('gamepad.vibration'), value: t(`gamepad.vib.${c.vibration}`), arrows: true, left: () => vib(-1), right: () => vib(1) });
    out.push({
      label: t('gamepad.test'),
      enter: async () => {
        const p = pads[0];
        if (!p) return ctx.note(t('gamepad.testfail'));
        try {
          await invoke('gamepad_test_vibration', { slot: p.slot });
        } catch {
          ctx.note(t('gamepad.testfail'));
        }
      },
    });
    const dz = (d: number) => save({ deadzone: Math.round(Math.max(0.1, Math.min(0.8, c.deadzone + d)) * 100) / 100 });
    out.push({ label: t('gamepad.deadzone'), value: `${Math.round(c.deadzone * 100)} %`, arrows: true, left: () => dz(-0.05), right: () => dz(0.05) });
    const ri = Math.max(0, REPEATS.findIndex((r) => r <= c.repeatRateMs));
    const rep = (d: number) => save({ repeatRateMs: REPEATS[Math.max(0, Math.min(REPEATS.length - 1, ri + d))] });
    out.push({ label: t('gamepad.repeat'), value: `${c.repeatRateMs} ms`, arrows: true, left: () => rep(-1), right: () => rep(1) });
    out.push(toggle(t('gamepad.swap'), 'swapAB'));
    out.push(toggle(t('gamepad.pointer'), 'pointerMode'));
    out.push(toggle(t('gamepad.tvonly'), 'onlyWhenTv'));
    out.push({ label: t(c.swapAB ? 'gamepad.map1s' : 'gamepad.map1') });
    out.push({ label: t('gamepad.map2') });
    out.push({ label: t('gamepad.map3') });
    return out;
  }

  return { titleKey: 'gamepad.title', load, rows };
}
