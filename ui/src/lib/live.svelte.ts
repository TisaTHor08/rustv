// TV en direct : liste de chaînes chargée (partagée entre la page et la recherche), guide, enregistreur.
import { invoke } from '@tauri-apps/api/core';
import { api, type LiveChannel, type Prog } from './api';
import { registerStrings, t } from './i18n';
import { strings } from './live.strings';
import { eg } from './epg.svelte';
import { prefs } from './prefs.svelte';

registerStrings('fr', strings.fr);
registerStrings('en', strings.en);

export const live = $state<{
  channels: LiveChannel[];
  groups: string[];
  nn: Record<string, { now: Prog | null; next: Prog | null }>;
  loading: boolean;
  error: string;
  sourceId: string;
  hasEpg: boolean;
}>({ channels: [], groups: [], nn: {}, loading: false, error: '', sourceId: '', hasEpg: false });

export const activeSource = () => prefs.live.sources.find((s) => s.id === prefs.live.active) ?? prefs.live.sources[0];

/** `{ m3u, xmltv }` d'une source : directement pour une liste M3U, via `tuner_build_source` pour un tuner. */
export async function resolveSource(s: NonNullable<ReturnType<typeof activeSource>>): Promise<{ m3u: string; xmltv: string }> {
  if (s.kind && s.kind !== 'm3u') {
    return invoke<{ m3u: string; xmltv: string }>('tuner_build_source', { spec: { kind: s.kind, host: s.host ?? '', port: s.port, user: s.user, password: s.password, xmltv: s.xmltv || undefined } });
  }
  return { m3u: s.m3u, xmltv: s.xmltv };
}

export async function loadLive(force = false) {
  const s = activeSource();
  if (!s) {
    live.channels = [];
    live.groups = [];
    live.sourceId = '';
    return;
  }
  if (!force && live.sourceId === s.id && live.channels.length) return;
  live.loading = true;
  live.error = '';
  try {
    const src = await resolveSource(s);
    const r = await api.liveLoad(src.m3u, src.xmltv || undefined);
    live.channels = r.channels;
    live.groups = r.groups;
    live.hasEpg = r.hasEpg;
    live.sourceId = s.id;
    if (r.epgError) live.error = r.epgError;
    await refreshGuide();
  } catch (e) {
    live.error = String(e);
    live.channels = [];
    live.groups = [];
  } finally {
    live.loading = false;
  }
}

export async function refreshGuide() {
  try {
    live.nn = await api.liveNowNext();
  } catch {}
  // Source sans guide : le guide TV automatique (s'il est activé) fournit « en cours / suivant ».
  if (!Object.keys(live.nn).length && eg.enabled) {
    try {
      live.nn = await invoke<Record<string, { now: Prog | null; next: Prog | null }>>('epg_live_nownext');
    } catch {}
  }
}

// ---------- guide ----------

export interface EpgProg extends Prog {
  /** Guide TV automatique seulement. */
  category?: string;
  icon?: string | null;
  rating?: string | null;
  left: number;
  width: number;
  clipL: boolean;
  clipR: boolean;
}

/** Programmes de [from, to) pour ces chaînes : { id → programmes }. */
export async function fetchEpg(from: number, to: number, ids: string[]): Promise<Record<string, EpgProg[]>> {
  const r = await invoke<{ channels: { id: string; programs: EpgProg[] }[] }>('live_epg', { fromTs: Math.floor(from), toTs: Math.floor(to), channelIds: ids });
  const out: Record<string, EpgProg[]> = {};
  for (const c of r.channels) out[c.id] = c.programs;
  return out;
}

/** Position d'un bloc dans la fenêtre [w0, w0 + span) (mêmes formules que `live::layout` côté Rust), en %. */
export function blockPos(p: { start: number; stop: number }, w0: number, span: number): { left: number; width: number } | null {
  if (p.stop <= w0 || p.start >= w0 + span || p.stop <= p.start) return null;
  const a = Math.max(p.start, w0);
  const b = Math.min(p.stop, w0 + span);
  return { left: ((a - w0) / span) * 100, width: ((b - a) / span) * 100 };
}

// ---------- favoris (par nom de chaîne, stockés dans le navigateur) ----------

const FAV_KEY = 'rustv.live.fav';
function readFav(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(FAV_KEY) ?? '[]');
    return Array.isArray(v) ? v.filter((x) => typeof x === 'string') : [];
  } catch {
    return [];
  }
}
export const favs = $state<{ names: string[] }>({ names: readFav() });
export const isFav = (name: string) => favs.names.includes(name);
export function toggleFav(name: string) {
  favs.names = isFav(name) ? favs.names.filter((n) => n !== name) : [...favs.names, name];
  try {
    localStorage.setItem(FAV_KEY, JSON.stringify(favs.names));
  } catch {}
}

// ---------- enregistreur ----------

export type RecStatus = 'scheduled' | 'recording' | 'done' | 'failed' | 'cancelled';
export interface Recording {
  id: string;
  channelId: string;
  channelName: string;
  url: string;
  title: string;
  start: number;
  stop: number;
  padBefore: number;
  padAfter: number;
  status: RecStatus;
  file: string;
  size: number;
  error: string;
}
export interface SeriesRule {
  title: string;
  channelId: string;
  channelName: string;
  url: string;
}

export const dvr = $state<{ recs: Recording[]; series: SeriesRule[]; freeBytes: number | null; running: number }>({ recs: [], series: [], freeBytes: null, running: 0 });

export async function loadDvr() {
  try {
    const r = await invoke<{ recordings: Recording[]; series: SeriesRule[]; freeBytes: number | null; running: number }>('dvr_list', { tz: -new Date().getTimezoneOffset() });
    dvr.recs = r.recordings;
    dvr.series = r.series;
    dvr.freeBytes = r.freeBytes;
    dvr.running = r.running;
  } catch {}
}

/** Programmé ou en cours pour cette chaîne et ce début ? (point rouge du guide) */
export const isScheduled = (channelName: string, start: number) => dvr.recs.some((r) => (r.status === 'scheduled' || r.status === 'recording') && r.channelName === channelName && r.start === start);

export function dvrError(e: unknown): string {
  const k = `lv.err.${String(e)}`;
  return t(k) === k ? String(e) : t(k);
}

export async function scheduleProg(ch: LiveChannel, p: { title: string; start: number; stop: number }): Promise<string> {
  const r = await invoke<{ id: string; warning: string | null }>('dvr_schedule', { spec: { channelId: ch.id, channelName: ch.name, url: ch.url, title: p.title, start: p.start, stop: p.stop } });
  await loadDvr();
  return r.warning === 'overlap' ? t('lv.rec.overlap') : t('lv.rec.ok');
}

export async function recordNow(ch: LiveChannel, minutes: number) {
  await invoke('dvr_start_now', { channel: { id: ch.id, name: ch.name, url: ch.url }, minutes });
  await loadDvr();
}

export async function addSeries(ch: LiveChannel, title: string) {
  await invoke('dvr_series_add', { rule: { title, channelId: ch.id, channelName: ch.name, url: ch.url } });
}

// ---------- pause / retour au direct (commandes de player.rs) ----------

/** Recule (négatif) ou avance (positif) dans le cache du direct ; appelé par la télécommande ou un mod. */
export const timeshift = (seconds: number) => invoke<void>('player_timeshift', { seconds });
export const goLive = () => invoke<void>('player_go_live');

// ---------- rappels (UI seulement, gardés dans le navigateur) ----------

export interface Reminder {
  channelName: string;
  title: string;
  start: number;
}
const REM_KEY = 'rustv.live.reminders';
function readRem(): Reminder[] {
  try {
    const v = JSON.parse(localStorage.getItem(REM_KEY) ?? '[]');
    return Array.isArray(v) ? v : [];
  } catch {
    return [];
  }
}
export const reminders = $state<{ list: Reminder[] }>({ list: readRem() });
function saveRem() {
  try {
    localStorage.setItem(REM_KEY, JSON.stringify(reminders.list));
  } catch {}
}
export function addReminder(r: Reminder) {
  if (!reminders.list.some((x) => x.channelName === r.channelName && x.start === r.start)) reminders.list = [...reminders.list, r];
  saveRem();
}

let remTimer: ReturnType<typeof setInterval> | null = null;
/** Démarre la surveillance des rappels (une seule fois) : un message 1 minute avant le début. */
export function startReminders(note: (m: string) => void) {
  if (remTimer) return;
  remTimer = setInterval(() => {
    const now = Date.now() / 1000;
    const due = reminders.list.filter((r) => r.start - now <= 60);
    if (!due.length) return;
    for (const r of due) if (r.start - now > -600) note(`${t('lv.remind.now', { t: r.title })} · ${r.channelName}`);
    reminders.list = reminders.list.filter((r) => r.start - now > 60);
    saveRem();
  }, 15000);
}
