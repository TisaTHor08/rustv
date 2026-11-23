import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface Tile {
  id: string;
  name: string;
  desc: string;
  category: 'video' | 'tv' | 'music' | 'library' | 'games';
  kind: 'web' | 'native';
  color: string;
  color2: string;
  dark_text?: boolean;
  needs_url?: boolean;
  url_hint?: string;
}

export interface Monitor {
  index: number;
  name: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Device {
  id: string;
  name: string;
  used: boolean;
  connected: boolean;
}

export interface Settings {
  blackout: boolean;
  urls: Record<string, string>;
  monitor?: number | null;
}

export interface ShellState {
  tv: boolean;
  monitors: Monitor[];
  settings: Settings;
  catalog: Tile[];
  devices: Device[];
  pcName: string;
  version: string;
  autostart: 'off' | 'tray' | 'tv';
}

export interface PairingStart {
  svg: string;
  device: string;
  name: string;
  bluetooth: boolean;
}

export interface FsEntry {
  name: string;
  path: string;
  dir: boolean;
  size: number;
}
export interface FsListing {
  path: string;
  parent: string | null;
  entries: FsEntry[];
}
export interface WinApp {
  id: string;
  name: string;
  target: string;
}

export interface ModInfo {
  id: string;
  manifest: any | null;
  permissions: string[];
  enabled: boolean;
  error: string | null;
}

export interface PlayItem {
  name: string;
  url: string;
}
export interface LiveChannel {
  id: string;
  name: string;
  logo: string;
  group: string;
  url: string;
  tvg: string;
}
export interface Prog {
  start: number;
  stop: number;
  title: string;
  desc: string;
}
export interface LiveLoad {
  channels: LiveChannel[];
  groups: string[];
  hasEpg: boolean;
  epgError: string | null;
}

export interface VaultInfo {
  installed: boolean;
  path: string | null;
  status: 'unknown' | 'notInstalled' | 'loggedOut' | 'locked' | 'unlocked';
  email: string;
  server: string;
  autolockMin: number;
}

export interface UpdateInfo {
  current: string;
  latest: string;
  available: boolean;
  page: string;
  notes: string;
  asset: { name: string; url: string; size: number; digest: string | null } | null;
}

export const api = {
  updateCheck: () => invoke<UpdateInfo>('update_check'),
  updateInstall: (asset: unknown) => invoke<void>('update_install', { asset }),
  onUpdateProgress: (cb: (p: { pct: number }) => void) => listen<{ pct: number }>('update-progress', (e) => cb(e.payload)),
  playerStatus: () => invoke<{ installed: boolean; path: string | null }>('player_status'),
  playerInstall: () => invoke<string>('player_install'),
  playerPlay: (req: { kind: 'live' | 'video' | 'audio'; items: PlayItem[]; index?: number }) => invoke<void>('player_play', { req }),
  playerStop: () => invoke<void>('player_stop'),
  liveLoad: (m3u: string, xmltv?: string) => invoke<LiveLoad>('live_load', { m3u, xmltv }),
  liveNowNext: () => invoke<Record<string, { now: Prog | null; next: Prog | null }>>('live_nownext'),
  libraryScan: (movies: string[], series: string[], photos: string[], music: string[]) => invoke<any>('library_scan', { movies, series, photos, music }),
  openPath: (path: string) => invoke<void>('open_path', { path }),
  modsList: () => invoke<ModInfo[]>('mods_list'),
  modSetEnabled: (id: string, enabled: boolean) => invoke<void>('mod_set_enabled', { id, enabled }),
  modUninstall: (id: string) => invoke<void>('mod_uninstall', { id }),
  modText: (id: string, rel: string) => invoke<string>('mod_text', { id, rel }),
  modBaseUrl: (id: string) => invoke<string>('mod_base_url', { id }),
  modStorageGet: (id: string, key: string) => invoke<unknown>('mod_storage_get', { id, key }),
  modStorageSet: (id: string, key: string, value: unknown) => invoke<void>('mod_storage_set', { id, key, value }),
  modFetch: (id: string, url: string) => invoke<{ status: number; body: string }>('mod_fetch', { id, url }),
  registryFetch: () => invoke<any>('registry_fetch'),
  modInstallPrepare: (entry: unknown) => invoke<{ manifest: any; permissions: string[] }>('mod_install_prepare', { entry }),
  modInstallCommit: (id: string) => invoke<void>('mod_install_commit', { id }),
  modInstallCancel: (id: string) => invoke<void>('mod_install_cancel', { id }),
  modsOpenFolder: () => invoke<void>('mods_open_folder'),
  vaultStatus: () => invoke<VaultInfo>('vault_status'),
  vaultLock: () => invoke<void>('vault_lock'),
  vaultLogout: () => invoke<void>('vault_logout'),
  vaultSetAutolock: (minutes: number) => invoke<void>('vault_set_autolock', { minutes }),
  vaultSetServer: (url: string) => invoke<void>('vault_set_server', { url }),
  prefsGet: () => invoke<Record<string, unknown> | null>('prefs_get'),
  prefsSet: (prefs: unknown) => invoke<void>('prefs_set', { prefs }),
  mediaUrl: (path: string) => invoke<string>('media_url', { path }),
  folderImages: (path: string) => invoke<string[]>('folder_images', { path }),
  fsRoots: () => invoke<{ name: string; path: string }[]>('fs_roots'),
  fsList: (path: string, filter: string) => invoke<FsListing>('fs_list', { path, filter }),
  windowsApps: () => invoke<WinApp[]>('windows_apps'),
  appIcon: (target: string) => invoke<string | null>('app_icon', { target }),
  launchCustom: (tile: unknown) => invoke<string>('launch_custom', { tile }),
  state: () => invoke<ShellState>('get_state'),
  tvStart: () => invoke<void>('tv_start'),
  tvExit: () => invoke<void>('tv_exit'),
  pickerMove: (dir: number) => invoke<void>('picker_move', { dir }),
  pickerChoose: (index: number) => invoke<void>('picker_choose', { index }),
  launch: (id: string, url?: string, browser?: string) => invoke<string>('launch_tile', { id, url, browser }),
  browsersDetected: () => invoke<{ kind: 'brave' | 'vivaldi'; path: string }[]>('browsers_detected'),
  goHome: () => invoke<void>('go_home'),
  pairingStart: () => invoke<PairingStart>('pairing_start'),
  pairingCancel: (device: string) => invoke<void>('pairing_cancel', { device }),
  deviceRevoke: (id: string) => invoke<void>('device_revoke', { id }),
  setBlackout: (on: boolean) => invoke<void>('set_blackout', { on }),
  setAutostart: (mode: string) => invoke<void>('set_autostart', { mode }),
  quit: () => invoke<void>('quit_app'),
  onDevicesChanged: (cb: () => void) => listen('devices-changed', cb),
  onPlayerInstall: (cb: (p: { pct: number; stage?: string }) => void) => listen<{ pct: number; stage?: string }>('player-install', (e) => cb(e.payload)),
  onPlayerClosed: (cb: () => void) => listen('player-closed', cb),
  /** Le téléphone demande d'ouvrir une tuile (mod de télécommande, scène…). */
  onOpenTile: (cb: (id: string) => void) => listen<string>('open-tile', (e) => cb(e.payload)),
};
