// Recherche universelle : types et appels Tauri (ne touche pas à api.ts).
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface UProvider {
  name: string;
  kind: 'flatrate' | 'free' | 'rent' | 'buy';
  id?: string;
  tileId?: string;
  url?: string;
  mapped: boolean;
}
export interface UTitle {
  id: number;
  kind: 'movie' | 'tv';
  title: string;
  year: number | null;
  poster: string | null;
  overview: string;
  providers: UProvider[];
}
export interface ULib {
  source: 'plex' | 'jellyfin' | 'ugreen';
  title: string;
  year?: number | null;
  kind: string;
  parent?: string | null;
  tileId: string;
  url: string | null;
}
export interface UFile {
  source: 'files';
  title: string;
  year: number | null;
  kind: 'video' | 'audio' | 'photo';
  sub: string;
  path: string;
  playable: boolean;
}
export interface UService {
  id: string;
  name: string;
  tileId: string;
  url: string;
  origin: string;
  kinds: string[];
}
export interface UResult {
  seq: number;
  titles: UTitle[];
  library: ULib[];
  files: UFile[];
  services: UService[];
  tmdb: 'idle' | 'off' | 'nokey' | 'ok' | 'error';
  country?: string;
  errors?: Record<string, string>;
  attribution?: { justwatch: string; tmdb: string } | null;
}
export type UPartial = Pick<UResult, 'seq' | 'files' | 'library' | 'services'>;

export interface UStatus {
  running: boolean;
  scanning: string;
  progress: number;
  total: number;
  lastScan: number;
  roots: { path: string; status: 'ok' | 'truncated' | 'unreachable' | 'cancelled'; count: number; scanned: number }[];
}

export interface UConfig {
  sources: { tmdb: boolean; library: boolean; files: boolean; plex: boolean; jellyfin: boolean; ugreen: boolean };
  country: string;
  folders: string[];
  libraryFolders: string[];
  hasTmdb: boolean;
  plex: { url: string; hasToken: boolean };
  jellyfin: { url: string; hasKey: boolean; tileUrl: string };
  ugreen: { url: string };
}

export const usearch = {
  query: (q: string, opts: { seq: number; lang: 'fr' | 'en'; country?: string }) => invoke<UResult>('usearch_query', { q, opts }),
  launch: (tileId: string, url?: string, tile?: unknown) => invoke<string>('usearch_launch', { tileId, url, tile }),
  getConfig: () => invoke<UConfig>('usearch_get_config'),
  setConfig: (patch: Record<string, unknown>) => invoke<void>('usearch_set_config', { patch }),
  status: () => invoke<UStatus>('usearch_status'),
  scan: (force = false) => invoke<boolean>('usearch_scan', { force }),
  scanCancel: () => invoke<void>('usearch_scan_cancel'),
  clearCache: () => invoke<void>('usearch_clear_cache'),
  test: (source: string) => invoke<string>('usearch_test', { source }),
  onPartial: (cb: (p: UPartial) => void) => listen<UPartial>('usearch-partial', (e) => cb(e.payload)),
  onScan: (cb: (s: UStatus) => void) => listen<UStatus>('usearch-scan', (e) => cb(e.payload)),
};
