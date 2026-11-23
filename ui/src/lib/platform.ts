// Fonctionnalités réellement disponibles sur ce système (commande Tauri `platform_features`).
// Les pages de réglages s'en servent pour masquer ou signaler ce dont le backend est un stub.
import { invoke } from '@tauri-apps/api/core';

export interface PlatformFeatures {
  os: 'windows' | 'linux' | 'other';
  input: boolean;
  /** `uinput_denied` (Linux), `unsupported_platform`… quand `input` est faux. */
  input_error: string | null;
  volume: boolean;
  audio_outputs: boolean;
  apps: boolean;
  media_session: boolean;
  text_focus: boolean;
  secrets: boolean;
  autostart: boolean;
  guard: boolean;
  display_modes: boolean;
  hdr: boolean;
  gamepad: boolean;
  bluetooth: boolean;
  self_install: boolean;
  streaming_browser: boolean;
}

let cached: Promise<PlatformFeatures | null> | null = null;

/** Lit une fois les fonctionnalités de la plateforme (null si la commande échoue : on suppose tout disponible). */
export function platformFeatures(force = false): Promise<PlatformFeatures | null> {
  if (!cached || force) cached = invoke<PlatformFeatures>('platform_features').catch(() => null);
  return cached;
}
