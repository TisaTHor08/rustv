// Compte Google partagé : types et appels Tauri (voir crates/rustv-shell/src/google.rs).
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface GoogleApp {
  id: string;
  name: string;
  on: boolean;
}
export interface GoogleState {
  enabled: boolean;
  connected: boolean;
  connecting: boolean;
  apps: GoogleApp[];
  overrides: Record<string, boolean>;
}

export const google = {
  get: () => invoke<GoogleState>('google_get'),
  setEnabled: (on: boolean) => invoke<void>('google_set_enabled', { on }),
  connect: () => invoke<void>('google_connect'),
  disconnect: () => invoke<void>('google_disconnect'),
  setApp: (id: string, on: boolean | null) => invoke<void>('google_set_app', { id, on }),
  onState: (cb: () => void) => listen('google-state', cb),
};
