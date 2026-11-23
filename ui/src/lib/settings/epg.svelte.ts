// Page de réglages « Guide TV » : activation, source, actualisation, état.
import { invoke } from '@tauri-apps/api/core';
import { registerStrings, t } from '../i18n';
import { epgError, getEpg, loadEpg, type EpgConfig, type EpgStatus } from '../epg.svelte';
import { askConfirm, askText } from '../overlays.svelte';
import { prefs } from '../prefs.svelte';
import { strings } from './epg.strings';

registerStrings('fr', strings.fr);
registerStrings('en', strings.en);

interface SettingsRow {
  label: string;
  value?: string;
  swatch?: string;
  left?: () => void | Promise<void>;
  right?: () => void | Promise<void>;
  enter?: () => void | Promise<void>;
  arrows?: boolean;
}

const PRESETS = ['tnt', 'fr', 'custom'] as const;

export function createPage(ctx: { note: (m: string) => void }) {
  let cfg = $state<EpgConfig | null>(null);
  let status = $state<EpgStatus | null>(null);
  let busy = $state(false);

  async function load() {
    try {
      const r = await getEpg();
      cfg = r.config;
      status = r.status;
    } catch (e) {
      ctx.note(epgError(e));
    }
  }

  async function set(patch: Record<string, unknown>): Promise<boolean> {
    try {
      const r = await invoke<{ config: EpgConfig; status: EpgStatus }>('epg_set', { cfg: patch });
      cfg = r.config;
      status = r.status;
      return true;
    } catch (e) {
      ctx.note(epgError(e));
      return false;
    }
  }

  async function refresh(force = true) {
    if (busy) return;
    busy = true;
    try {
      await invoke('epg_refresh', { force });
    } catch (e) {
      ctx.note(epgError(e));
    } finally {
      busy = false;
    }
    await load();
    await loadEpg();
  }

  async function toggle() {
    if (!cfg) return;
    if (cfg.enabled) {
      await set({ enabled: false });
      await loadEpg();
      return;
    }
    if (!(await askConfirm(t('epg.confirm.title'), t('epg.confirm.text', { host: cfg.host }), t('epg.confirm.ok'), t('common.cancel')))) return;
    if (await set({ enabled: true })) await refresh();
  }

  async function cyclePreset(d: number) {
    if (!cfg) return;
    const i = PRESETS.indexOf(cfg.preset);
    const next = PRESETS[(i + d + PRESETS.length) % PRESETS.length];
    if (next === 'fr' && cfg.enabled && !(await askConfirm(t('epg.set.fr.confirm', { host: new URL(cfg.presets.fr).host }), '', t('epg.set.fr.ok'), t('common.cancel')))) return;
    if (!(await set({ preset: next }))) return;
    if (cfg?.enabled && next !== 'custom') await refresh();
  }

  async function editUrl() {
    if (!cfg) return;
    const u = await askText(t('epg.set.url.prompt'), cfg.url || 'https://');
    if (u == null) return;
    if (await set({ url: u.trim(), preset: 'custom' }) && cfg?.enabled) await refresh();
  }

  const age = (s: number) => (s < 5400 ? t('epg.set.age.min', { n: Math.max(1, Math.round(s / 60)) }) : t('epg.set.age.h', { n: Math.round(s / 3600) }));
  const day = (ts: number) => new Date(ts * 1000).toLocaleDateString(prefs.lang === 'en' ? 'en-US' : 'fr-FR', { weekday: 'short', day: 'numeric', month: 'short' });

  function statusText(): string {
    if (!cfg?.enabled) return t('epg.set.status.off');
    if (!status?.loaded) return t('epg.set.status.none');
    return t('epg.set.status.ok', { c: status.channels, p: status.programmes, a: day(status.from), b: day(status.to), age: status.ageSecs == null ? '?' : age(status.ageSecs) });
  }

  function rows(): SettingsRow[] {
    if (!cfg) return [];
    const c = cfg;
    const r: SettingsRow[] = [
      { label: t('epg.set.enabled'), value: c.enabled ? t('epg.set.on') : t('epg.set.off'), arrows: true, left: toggle, right: toggle, enter: toggle },
      { label: t('epg.set.preset'), value: t(`epg.set.preset.${c.preset}`), arrows: true, left: () => cyclePreset(-1), right: () => cyclePreset(1), enter: () => cyclePreset(1) },
    ];
    if (c.preset === 'custom') r.push({ label: t('epg.set.url'), value: c.url || t('epg.set.url.none'), enter: editUrl });
    r.push({ label: t('epg.set.icons'), value: c.icons ? t('epg.set.on') : t('epg.set.off'), arrows: true, left: () => void set({ icons: !c.icons }), right: () => void set({ icons: !c.icons }), enter: () => void set({ icons: !c.icons }) });
    if (c.enabled) r.push({ label: t('epg.set.refresh'), value: busy ? t('epg.set.refreshing') : '', enter: () => refresh(true) });
    r.push({ label: t('epg.set.status'), value: statusText() });
    if (c.lastError) r.push({ label: t('epg.set.err'), value: epgError(c.lastError) });
    r.push({ label: t('epg.set.notice') });
    r.push({ label: t('epg.set.tuner') });
    return r;
  }

  return { titleKey: 'settings.epg', load, rows };
}
