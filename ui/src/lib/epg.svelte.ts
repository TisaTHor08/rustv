// Guide TV (EPG) : état partagé entre la page Live TV, la recherche et les réglages (commandes de epg.rs).
import { invoke } from '@tauri-apps/api/core';
import { registerStrings, t } from './i18n';
import { strings } from './epg.strings';
import { askConfirm } from './overlays.svelte';
import { prefs } from './prefs.svelte';

registerStrings('fr', strings.fr);
registerStrings('en', strings.en);

export interface EpgStatus {
  loaded: boolean;
  channels: number;
  programmes: number;
  from: number;
  to: number;
  ageSecs: number | null;
  truncated: boolean;
  refreshing: boolean;
  error?: string;
}
export interface EpgConfig {
  enabled: boolean;
  preset: 'tnt' | 'fr' | 'custom';
  url: string;
  effectiveUrl: string;
  host: string;
  icons: boolean;
  lastFetch: number;
  lastOk: number;
  lastError: string;
  presets: { tnt: string; fr: string };
}
export interface EpgChannel {
  id: string;
  name: string;
  icon: string | null;
  playable: boolean;
  liveChannelId: string | null;
}
export interface EpgHit {
  channelId: string;
  channelName: string;
  channelIcon: string | null;
  title: string;
  desc: string;
  category: string;
  icon: string | null;
  start: number;
  stop: number;
  now: boolean;
  upcomingIndex: number;
  playable: boolean;
  liveChannelId: string | null;
  streamUrl: string | null;
}

export const eg = $state<{ ready: boolean; enabled: boolean; loaded: boolean; host: string; channels: EpgChannel[]; from: number; to: number }>({ ready: false, enabled: false, loaded: false, host: '', channels: [], from: 0, to: 0 });

/** Programme à montrer à l'ouverture de la page Live TV (« Voir dans le guide » depuis la recherche). */
export const guideTarget = $state<{ v: { channelId: string; liveChannelId: string | null; start: number } | null }>({ v: null });

export const epgError = (e: unknown) => {
  const k = `epg.err.${String(e)}`;
  return t(k) === k ? String(e) : t(k);
};

export async function getEpg() {
  return invoke<{ config: EpgConfig; status: EpgStatus }>('epg_get');
}

/** Recharge l'état et la liste des chaînes du guide. */
export async function loadEpg() {
  try {
    const r = await getEpg();
    eg.enabled = r.config.enabled;
    eg.host = r.config.host;
    eg.loaded = r.status.loaded;
    eg.from = r.status.from;
    eg.to = r.status.to;
    eg.channels = r.config.enabled && r.status.loaded ? await invoke<EpgChannel[]>('epg_channels') : [];
  } catch {
    eg.enabled = false;
    eg.loaded = false;
    eg.channels = [];
  } finally {
    eg.ready = true;
  }
}

/** Active le guide après confirmation (nomme l'hôte contacté), puis le télécharge. `true` si le guide est prêt. */
export async function enableEpg(note: (m: string) => void): Promise<boolean> {
  try {
    const host = (await getEpg()).config.host;
    if (!(await askConfirm(t('epg.confirm.title'), t('epg.confirm.text', { host }), t('epg.confirm.ok'), t('common.cancel')))) return false;
    await invoke('epg_set', { cfg: { enabled: true } });
    note(t('epg.fetching'));
    try {
      await invoke('epg_refresh', { force: true });
    } catch (e) {
      note(epgError(e));
    }
    await loadEpg();
    if (eg.loaded) note(t('epg.ready', { n: eg.channels.length }));
    return eg.loaded;
  } catch (e) {
    note(epgError(e));
    return false;
  }
}

export interface GuideProg {
  start: number;
  stop: number;
  title: string;
  desc: string;
  category?: string;
  icon?: string | null;
  rating?: string | null;
  left: number;
  width: number;
  clipL: boolean;
  clipR: boolean;
}

/** Programmes de [from, to) pour ces chaînes du guide : { id du guide → programmes }. */
export async function fetchBuiltinGuide(from: number, to: number, ids: string[]): Promise<Record<string, GuideProg[]>> {
  const r = await invoke<{ channels: { id: string; programs: GuideProg[] }[] }>('epg_guide', { fromTs: Math.floor(from), toTs: Math.floor(to), channelIds: ids });
  const out: Record<string, GuideProg[]> = {};
  for (const c of r.channels) out[c.id] = c.programs;
  return out;
}

/** « Ce soir » (20 h 50, demain après 22 h 30), calculé par epg.rs. */
export const tonightTs = () => invoke<number>('epg_tonight', { tz: -new Date().getTimezoneOffset() });

/** Seules les images https sont chargées ; les échecs de chargement sont silencieux (voir `hideBroken`). */
export const safeImg = (u?: string | null) => (u && u.startsWith('https://') ? u : '');
export const hideBroken = (e: Event) => ((e.currentTarget as HTMLElement).style.display = 'none');

const loc = () => (prefs.lang === 'en' ? 'en-US' : 'fr-FR');
const hm = (ts: number) => new Date(ts * 1000).toLocaleTimeString(loc(), { hour: '2-digit', minute: '2-digit' });
const dayStart = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();

/** « En ce moment, jusqu'à 21:30 », « Aujourd'hui 20:55 », « Demain 14:10 », sinon jour et heure. */
export function whenLabel(start: number, stop: number, now = Date.now() / 1000): string {
  if (start <= now && now < stop) return t('epg.when.now', { t: hm(stop) });
  const d = Math.round((dayStart(new Date(start * 1000)) - dayStart(new Date(now * 1000))) / 86400000);
  if (d === 0) return t('epg.when.today', { t: hm(start) });
  if (d === 1) return t('epg.when.tomorrow', { t: hm(start) });
  return new Date(start * 1000).toLocaleString(loc(), { weekday: 'short', day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });
}
