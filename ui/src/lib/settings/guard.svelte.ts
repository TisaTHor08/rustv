// Page « Garde TV » : démarrage sur la TV + protection du premier plan. Utilise les commandes Tauri
// `guard_*` et la commande existante `set_autostart`.
import { invoke } from '@tauri-apps/api/core';
import { registerStrings, t } from '../i18n';
import { askConfirm } from '../overlays.svelte';
import { strings } from './guard.strings';

registerStrings('fr', strings.fr);
registerStrings('en', strings.en);

type SettingsRow = {
  label: string;
  value?: string;
  swatch?: string;
  left?: () => void | Promise<void>;
  right?: () => void | Promise<void>;
  enter?: () => void | Promise<void>;
  arrows?: boolean;
};

interface GuardStatus {
  active: boolean;
  guardRunning: boolean;
  actions: number;
  hiddenWindows: number;
}
interface GuardView {
  enabled: boolean;
  mode: 'minimize' | 'move';
  keepAwake: boolean;
  hideTaskbar: boolean;
  suppressToasts: boolean;
  allowProcesses: string[];
  allowClasses: string[];
  bootToTv: 'off' | 'tray' | 'tv';
  status: GuardStatus;
}

const BOOT = ['off', 'tray', 'tv'] as const;
const MODES = ['minimize', 'move'] as const;

export function createPage(ctx: { note: (m: string) => void }) {
  let g = $state<GuardView | null>(null);

  async function load() {
    try {
      g = await invoke<GuardView>('guard_get');
    } catch {
      g = null;
    }
  }

  async function patch(p: Partial<GuardView>) {
    if (!g) return;
    const { status: _s, ...cfg } = g;
    try {
      await invoke('guard_set', { cfg: { ...cfg, ...p } });
    } catch {
      ctx.note(t('guard.err'));
    }
    await load();
  }

  const yn = (v: boolean) => t(v ? 'guard.yes' : 'guard.no');
  const toggle = (label: string, get: () => boolean, key: 'enabled' | 'keepAwake' | 'hideTaskbar' | 'suppressToasts'): SettingsRow => {
    const flip = () => patch({ [key]: !get() });
    return { label, value: yn(get()), arrows: true, left: flip, right: flip, enter: flip };
  };
  const cycle = <T,>(arr: readonly T[], cur: T, d: number): T => arr[(arr.indexOf(cur) + d + arr.length) % arr.length];

  async function panic() {
    if (!(await askConfirm(t('guard.panic.ask'), t('guard.panic.text'), t('guard.panic.ok'), t('common.cancel')))) return;
    try {
      await invoke('guard_panic');
      ctx.note(t('guard.panic.done'));
    } catch {
      ctx.note(t('guard.err'));
    }
    await load();
  }

  async function setBoot(m: (typeof BOOT)[number]) {
    try {
      await invoke('set_autostart', { mode: m });
    } catch {
      ctx.note(t('guard.err'));
    }
    await load();
  }

  return {
    titleKey: 'guard.title',
    load,
    rows(): SettingsRow[] {
      const v = g;
      const boot = v?.bootToTv ?? 'off';
      const mode = v?.mode ?? 'minimize';
      return [
        {
          label: t('guard.boot'),
          value: t(`guard.boot.${boot}`),
          arrows: true,
          left: () => setBoot(cycle(BOOT, boot, -1)),
          right: () => setBoot(cycle(BOOT, boot, 1)),
          enter: () => setBoot(cycle(BOOT, boot, 1)),
        },
        toggle(t('guard.enabled'), () => v?.enabled ?? true, 'enabled'),
        {
          label: t('guard.mode'),
          value: t(`guard.mode.${mode}`),
          arrows: true,
          left: () => patch({ mode: cycle(MODES, mode, -1) }),
          right: () => patch({ mode: cycle(MODES, mode, 1) }),
          enter: () => patch({ mode: cycle(MODES, mode, 1) }),
        },
        toggle(t('guard.awake'), () => v?.keepAwake ?? true, 'keepAwake'),
        toggle(t('guard.taskbar'), () => v?.hideTaskbar ?? true, 'hideTaskbar'),
        toggle(t('guard.toasts'), () => v?.suppressToasts ?? true, 'suppressToasts'),
        {
          label: t('guard.status'),
          value: t('guard.status.v', { state: t(v?.status.active ? 'guard.status.on' : 'guard.status.off'), n: v?.status.actions ?? 0, h: v?.status.hiddenWindows ?? 0 }),
          enter: load,
        },
        { label: t('guard.allow'), value: String((v?.allowProcesses.length ?? 0) + (v?.allowClasses.length ?? 0)) },
        { label: t('guard.limits') },
        { label: t('guard.panic'), enter: panic },
      ];
    },
  };
}
