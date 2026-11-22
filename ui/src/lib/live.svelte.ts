// TV en direct : liste de chaînes chargée (partagée entre la page et la recherche).
import { api, type LiveChannel, type Prog } from './api';
import { prefs } from './prefs.svelte';

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
    const r = await api.liveLoad(s.m3u, s.xmltv || undefined);
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
}
