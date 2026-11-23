// Page « Ambilight » (Hyperion) des réglages : état, installation, démarrage, LED, luminosité, effets, comportement.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { t } from '../i18n';
import { askConfirm, askText, overlay } from '../overlays.svelte';
import './hyperion.strings';

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
  manage: boolean;
  host: string;
  port: number;
  hasToken: boolean;
  ambient: 'tile' | 'grabber' | 'off';
  ledsOnTvStart: boolean;
  ledsOffTvExit: boolean;
  stopOnTvExit: boolean;
  captureOnVideo: boolean;
  dimOnPause: boolean;
  dimAfterSecs: number;
  favourites: string[];
}
interface Status {
  installed: boolean;
  version?: string;
  running: boolean;
  connected: boolean;
  external: boolean;
  ledOn: boolean;
  brightness?: number;
  effects: string[];
  installerPath?: string;
}
interface Snap {
  config: Cfg;
  status: Status;
}

const AMBIENT = ['tile', 'grabber', 'off'] as const;
const DELAYS = [10, 30, 60, 120, 300];

export function createPage(ctx: { note: (m: string) => void }) {
  let snap = $state<Snap | null>(null);
  let sel = $state('');

  const yn = (v: boolean) => t(v ? 'hyperion.yes' : 'hyperion.no');
  const errText = (e: unknown) => {
    const k = `hyperion.err.${String(e)}`;
    const s = t(k);
    return t('hyperion.err', { e: s === k ? String(e) : s });
  };
  async function run<T>(fn: () => Promise<T>): Promise<T | undefined> {
    try {
      return await fn();
    } catch (e) {
      ctx.note(errText(e));
      return undefined;
    }
  }
  async function load() {
    const s = await run(() => invoke<Snap>('hyperion_get'));
    if (s) snap = s;
  }
  async function patch(p: Record<string, unknown>) {
    const s = await run(() => invoke<Snap>('hyperion_set', { cfg: p }));
    if (s) snap = s;
  }
  function cycle<T>(arr: T[], cur: T, d: number): T {
    const i = Math.max(0, arr.indexOf(cur));
    return arr[(i + d + arr.length) % arr.length];
  }
  const toggle = (label: string, key: keyof Cfg): SettingsRow => {
    const f = () => patch({ [key]: !snap!.config[key] });
    return { label, value: yn(snap!.config[key] as boolean), arrows: true, left: f, right: f, enter: f };
  };

  function effectList(): string[] {
    const s = snap!;
    const fav = s.config.favourites.filter((f) => s.status.effects.includes(f));
    return fav.length ? fav : s.status.effects;
  }
  const current = () => (sel && effectList().includes(sel) ? sel : (effectList()[0] ?? ''));

  async function install() {
    const ok = await askConfirm(t('hyperion.title'), t('hyperion.install.ask'), t('hyperion.install.go'), t('common.cancel'));
    if (!ok) return;
    overlay.busy = { title: t('hyperion.installing'), text: 'Hyperion', pct: 0 };
    const off = await listen<{ pct: number; stage?: string }>('hyperion-install', (e) => {
      if (!overlay.busy) return;
      overlay.busy.pct = e.payload.pct;
      overlay.busy.title = e.payload.stage === 'extract' ? t('hyperion.extracting') : t('hyperion.installing');
    });
    let r: { kind: string; path?: string } | undefined;
    try {
      r = await invoke('hyperion_install');
    } catch (e) {
      ctx.note(errText(e));
    }
    off();
    overlay.busy = null;
    await load();
    if (!r) return;
    if (r.kind === 'installer') await openInstaller(r.path);
    else ctx.note(t('hyperion.installed'));
  }
  async function openInstaller(path?: string) {
    const p = path ?? snap?.status.installerPath;
    if (!p) return;
    const ok = await askConfirm(t('hyperion.title'), t('hyperion.installer.ask'), t('hyperion.installer.go'), t('common.cancel'));
    if (ok) await run(() => invoke('open_path', { path: p }));
  }

  function status(): string {
    const s = snap!;
    if (!s.config.enabled) return t('hyperion.st.off');
    if (s.status.connected) return t(s.status.external ? 'hyperion.st.external' : 'hyperion.st.connected') + (s.status.version ? ` (${s.status.version})` : '');
    if (!s.status.installed && s.config.manage) return t('hyperion.st.notinstalled');
    return t(s.status.installed && !s.status.running ? 'hyperion.st.stopped' : 'hyperion.st.unreachable');
  }

  function rows(): SettingsRow[] {
    if (!snap) return [{ label: t('hyperion.refresh'), enter: load }];
    const c = snap.config;
    const s = snap.status;
    const out: SettingsRow[] = [{ label: t('hyperion.status'), value: status(), enter: load }, toggle(t('hyperion.enabled'), 'enabled')];
    if (!c.enabled) return out;
    if (c.manage) {
      out.push({ label: t(s.installed ? 'hyperion.update' : 'hyperion.install'), enter: install });
      if (s.installerPath) out.push({ label: t('hyperion.openinstaller'), enter: () => openInstaller() });
      if (s.installed) {
        out.push({
          label: t('hyperion.process'),
          value: t(s.running ? 'hyperion.process.stop' : 'hyperion.process.start'),
          enter: async () => {
            const r = await run(() => invoke<Snap>(s.running ? 'hyperion_stop' : 'hyperion_start'));
            if (r) snap = r;
          },
        });
      }
    }
    if (s.connected) {
      const power = async () => {
        await run(() => invoke('hyperion_power', { on: !s.ledOn }));
        await load();
      };
      out.push({ label: t('hyperion.power'), value: t(s.ledOn ? 'hyperion.on' : 'hyperion.offv'), arrows: true, left: power, right: power, enter: power });
      const bri = (d: number) => async () => {
        const v = Math.max(0, Math.min(100, (s.brightness ?? 100) + d));
        await run(() => invoke('hyperion_brightness', { level: v }));
        await load();
      };
      out.push({ label: t('hyperion.brightness'), value: s.brightness == null ? '' : `${s.brightness} %`, arrows: true, left: bri(-10), right: bri(10), enter: bri(10) });
      const list = effectList();
      if (list.length) {
        const pick = (d: number) => () => {
          sel = cycle(list, current(), d);
        };
        out.push({
          label: t('hyperion.effect'),
          value: current() + (c.favourites.includes(current()) ? ' *' : ''),
          arrows: true,
          left: pick(-1),
          right: pick(1),
          enter: async () => {
            await run(() => invoke('hyperion_effect', { name: current() }));
            await load();
          },
        });
        out.push({
          label: t('hyperion.effect.fav'),
          value: yn(c.favourites.includes(current())),
          enter: () => patch({ favourites: c.favourites.includes(current()) ? c.favourites.filter((f) => f !== current()) : [...c.favourites, current()] }),
        });
      }
      out.push({
        label: t('hyperion.effect.clear'),
        enter: async () => {
          await run(() => invoke('hyperion_clear'));
          await load();
        },
      });
    }
    const amb = (d: number) => () => patch({ ambient: cycle([...AMBIENT], c.ambient, d) });
    out.push({ label: t('hyperion.ambient'), value: t(`hyperion.ambient.${c.ambient}`), arrows: true, left: amb(-1), right: amb(1), enter: amb(1) });
    out.push(toggle(t('hyperion.capture'), 'captureOnVideo'));
    out.push(toggle(t('hyperion.dim'), 'dimOnPause'));
    if (c.dimOnPause) {
      const dl = (d: number) => () => patch({ dimAfterSecs: cycle(DELAYS, c.dimAfterSecs, d) });
      out.push({ label: t('hyperion.dimafter'), value: `${c.dimAfterSecs} s`, arrows: true, left: dl(-1), right: dl(1), enter: dl(1) });
    }
    out.push(toggle(t('hyperion.tvstart'), 'ledsOnTvStart'));
    out.push(toggle(t('hyperion.tvexit'), 'ledsOffTvExit'));
    out.push(toggle(t('hyperion.manage'), 'manage'));
    if (c.manage) out.push(toggle(t('hyperion.stopexit'), 'stopOnTvExit'));
    out.push({
      label: t('hyperion.host'),
      value: c.host,
      enter: async () => {
        const v = await askText(t('hyperion.host'), c.host);
        if (v) await patch({ host: v.trim() });
      },
    });
    out.push({
      label: t('hyperion.port'),
      value: String(c.port),
      enter: async () => {
        const v = await askText(t('hyperion.port'), String(c.port));
        const n = Number(v);
        if (v && Number.isInteger(n) && n > 0 && n < 65536) await patch({ port: n });
        else if (v) ctx.note(t('hyperion.err.invalid_port'));
      },
    });
    out.push({
      label: t('hyperion.token'),
      value: t(c.hasToken ? 'hyperion.token.set' : 'hyperion.token.none'),
      enter: async () => {
        const v = await askText(t('hyperion.token.ask'), '');
        if (v !== null) await patch({ token: v });
      },
    });
    out.push({ label: t('hyperion.webui'), value: t('hyperion.webui.d'), enter: () => void run(() => invoke('hyperion_open_web_ui')) });
    return out;
  }

  return { titleKey: 'hyperion.title', load, rows };
}
