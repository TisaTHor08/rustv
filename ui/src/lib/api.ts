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

export const api = {
  state: () => invoke<ShellState>('get_state'),
  tvStart: () => invoke<void>('tv_start'),
  tvExit: () => invoke<void>('tv_exit'),
  pickerMove: (dir: number) => invoke<void>('picker_move', { dir }),
  pickerChoose: (index: number) => invoke<void>('picker_choose', { index }),
  launch: (id: string, url?: string) => invoke<string>('launch_tile', { id, url }),
  goHome: () => invoke<void>('go_home'),
  pairingStart: () => invoke<PairingStart>('pairing_start'),
  pairingCancel: (device: string) => invoke<void>('pairing_cancel', { device }),
  deviceRevoke: (id: string) => invoke<void>('device_revoke', { id }),
  setBlackout: (on: boolean) => invoke<void>('set_blackout', { on }),
  setAutostart: (mode: string) => invoke<void>('set_autostart', { mode }),
  quit: () => invoke<void>('quit_app'),
  onDevicesChanged: (cb: () => void) => listen('devices-changed', cb),
};
