// Page de réglages « Enregistrement » : dossier, marges, enregistrements simultanés, rétention, tuners.
import { invoke } from '@tauri-apps/api/core';
import { registerStrings, t } from '../i18n';
import { loadLive } from '../live.svelte';
import { askText, pickFile } from '../overlays.svelte';
import { prefs, savePrefs } from '../prefs.svelte';
import { strings } from './dvr.strings';

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

interface Cfg {
  folder: string;
  padBefore: number;
  padAfter: number;
  maxParallel: number;
  retentionGb: number;
  retentionDays: number;
  minFreeGb: number;
  tz: number;
}
interface Found {
  kind: string;
  name: string;
  host: string;
  channels: number | null;
  tuners: number | null;
}

const GB_STEPS = [0, 10, 25, 50, 100, 250, 500, 1000, 2000];
const DAY_STEPS = [0, 3, 7, 14, 30, 60, 90, 180, 365];
const FREE_STEPS = [1, 2, 5, 10, 20, 50];

function step(arr: number[], cur: number, d: number) {
  let i = arr.findIndex((x) => x >= cur);
  if (i < 0) i = arr.length - 1;
  return arr[Math.max(0, Math.min(arr.length - 1, i + d))];
}

export function createPage(ctx: { note: (m: string) => void }) {
  let cfg = $state<Cfg | null>(null);
  let found = $state<Found[]>([]);
  let searching = $state(false);
  let searched = $state(false);

  async function load() {
    try {
      cfg = await invoke<Cfg>('dvr_get_config');
    } catch (e) {
      ctx.note(String(e));
    }
  }

  async function save(patch: Partial<Cfg>) {
    if (!cfg) return;
    try {
      cfg = await invoke<Cfg>('dvr_set_config', { cfg: { ...cfg, ...patch } });
    } catch (e) {
      const k = `dvr.err.${String(e)}`;
      ctx.note(t(k) === k ? String(e) : t(k));
    }
  }

  const num = (label: string, key: 'padBefore' | 'padAfter' | 'maxParallel', min: number, max: number): SettingsRow => ({
    label,
    value: String(cfg![key]),
    arrows: true,
    left: () => save({ [key]: Math.max(min, cfg![key] - 1) }),
    right: () => save({ [key]: Math.min(max, cfg![key] + 1) }),
  });

  const stepped = (label: string, key: 'retentionGb' | 'retentionDays' | 'minFreeGb', steps: number[], fmt: (n: number) => string): SettingsRow => ({
    label,
    value: fmt(cfg![key]),
    arrows: true,
    left: () => save({ [key]: step(steps, cfg![key], -1) }),
    right: () => save({ [key]: step(steps, cfg![key], 1) }),
  });

  async function pickFolder() {
    const p = await pickFile(t('dvr.folder.pick'), 'folder');
    if (p) await save({ folder: p });
  }

  async function search() {
    searching = true;
    try {
      found = await invoke<Found[]>('tuner_discover');
      searched = true;
      ctx.note(found.length ? t('dvr.found', { n: found.length }) : t('dvr.none'));
    } catch (e) {
      ctx.note(String(e));
    } finally {
      searching = false;
    }
  }

  async function addSource(src: { name: string; kind: 'hdhomerun' | 'tvheadend'; host: string; port?: number; user?: string; password?: string; xmltv?: string }, tuners?: number | null) {
    try {
      // Valide la configuration (télécharge le lineup d'un HDHomeRun) avant d'enregistrer la source.
      await invoke('tuner_build_source', { spec: { kind: src.kind, host: src.host, port: src.port, user: src.user, password: src.password, xmltv: src.xmltv } });
    } catch (e) {
      ctx.note(String(e));
      return;
    }
    const id = `src${Date.now().toString(36)}`;
    prefs.live.sources = [...prefs.live.sources, { id, name: src.name, m3u: '', xmltv: src.xmltv ?? '', kind: src.kind, host: src.host, port: src.port, user: src.user, password: src.password }];
    prefs.live.active = id;
    savePrefs();
    ctx.note(t('dvr.added', { name: src.name }));
    if (tuners && cfg && cfg.maxParallel > tuners) {
      await save({ maxParallel: tuners });
      ctx.note(t('dvr.tuners.limit', { n: tuners }));
    }
    loadLive(true).catch(() => {});
  }

  async function addHdhr(host?: string, name?: string, tuners?: number | null) {
    const h = host ?? (await askText(t('dvr.hdhr.ip'), ''));
    if (!h) return;
    const xmltv = (await askText(t('dvr.xmltv'), '', t('dvr.xmltv'))) ?? '';
    await addSource({ name: name ?? `HDHomeRun ${h}`, kind: 'hdhomerun', host: h.trim(), xmltv: xmltv.trim() }, tuners);
  }

  async function addTvh() {
    const host = await askText(t('dvr.tvh.host'), '');
    if (!host) return;
    const port = Number(await askText(t('dvr.tvh.port'), '9981')) || 9981;
    const user = (await askText(t('dvr.tvh.user'), '')) ?? '';
    const password = user ? ((await askText(t('dvr.tvh.pass'), '')) ?? '') : '';
    const xmltv = (await askText(t('dvr.xmltv'), '', t('dvr.xmltv'))) ?? '';
    ctx.note(t('dvr.note.creds'));
    await addSource({ name: `TVHeadend ${host.trim()}`, kind: 'tvheadend', host: host.trim(), port, user: user || undefined, password: password || undefined, xmltv: xmltv.trim() });
  }

  function rows(): SettingsRow[] {
    const r: SettingsRow[] = [];
    if (cfg) {
      r.push({ label: t('dvr.folder'), value: cfg.folder, enter: pickFolder });
      r.push(num(t('dvr.padbefore'), 'padBefore', 0, 30));
      r.push(num(t('dvr.padafter'), 'padAfter', 0, 60));
      r.push(num(t('dvr.parallel'), 'maxParallel', 1, 8));
      r.push(stepped(t('dvr.retgb'), 'retentionGb', GB_STEPS, (n) => (n ? `${n}` : t('dvr.off'))));
      r.push(stepped(t('dvr.retdays'), 'retentionDays', DAY_STEPS, (n) => (n ? t('dvr.days', { n }) : t('dvr.off'))));
      r.push(stepped(t('dvr.minfree'), 'minFreeGb', FREE_STEPS, (n) => String(n)));
    }
    r.push({ label: t('dvr.search'), value: searching ? t('dvr.searching') : searched ? t('dvr.found', { n: found.length }) : '', enter: search });
    for (const f of found) {
      const info = [f.tuners ? t('dvr.tuners', { n: f.tuners }) : '', f.channels != null ? t('dvr.channels', { n: f.channels }) : ''].filter(Boolean).join(' · ');
      r.push({ label: `${f.name} (${f.host})`, value: info, enter: () => addHdhr(f.host, f.name, f.tuners) });
    }
    r.push({ label: t('dvr.add.hdhr'), enter: () => addHdhr() });
    r.push({ label: t('dvr.add.tvh'), enter: addTvh });
    return r;
  }

  return { titleKey: 'settings.dvr', load, rows };
}
