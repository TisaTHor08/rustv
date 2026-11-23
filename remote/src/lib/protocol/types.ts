export type KeyName = 'up' | 'down' | 'left' | 'right' | 'enter' | 'back' | 'backlong' | 'menu' | 'search';
export type AltTabAction = 'open' | 'left' | 'right' | 'confirm';

export type PhoneMsg =
  | { t: 'key'; k: KeyName }
  | { t: 'altTab'; a: AltTabAction }
  | { t: 'pin' }
  | { t: 'text'; backspaces: number; text: string }
  | { t: 'mouse'; dx: number; dy: number; click?: 'left' | 'right'; scroll?: number }
  | { t: 'launch'; appId: string }
  | { t: 'getApps' }
  | { t: 'getRemoteMods' }
  | { t: 'openTile'; id: string }
  | { t: 'home' }
  | { t: 'tv'; on: boolean }
  | { t: 'media'; a: MediaAction }
  | { t: 'setVolume'; level: number }
  | { t: 'vaultStatus' }
  | { t: 'vaultUnlock'; password: string }
  | { t: 'vaultLogin'; email: string; password: string; code?: string; method?: 0 | 1 | 3 }
  | { t: 'vaultLock' }
  | { t: 'vaultSearch'; q: string }
  | { t: 'vaultFill'; id: string; what: VaultFill; enter?: boolean }
  | { t: 'ping' };

export type VaultFill = 'both' | 'username' | 'password' | 'totp';
export type VaultStatus = 'unknown' | 'notInstalled' | 'loggedOut' | 'locked' | 'unlocked';
/** Métadonnées non secrètes : les mots de passe ne quittent jamais le PC. */
export interface VaultItem {
  id: string;
  name: string;
  username: string;
  host: string;
  hasTotp: boolean;
}

export type MediaAction = 'playPause' | 'next' | 'prev' | 'stop' | 'volUp' | 'volDown' | 'mute';

export interface AppEntry {
  id: string;
  name: string;
  icon: string;
}

export type Label = string | { fr?: string; en?: string };
export interface RemoteTheme {
  id: string;
  name: Label;
  vars: Record<string, string>;
  css: string;
}
/** Action d'un bouton de mod ; `wait` est propre au téléphone (jamais envoyée au PC). */
export type RemoteAction = PhoneMsg | { t: 'wait'; ms: number };
export interface RemoteButton {
  label: Label;
  glyph: string;
  actions: RemoteAction[];
}
export interface RemotePanel {
  id: string;
  name: Label;
  glyph: string;
  buttons: RemoteButton[];
}
export interface RemoteMod {
  id: string;
  name: string;
  i18n?: { fr?: { name?: string } };
  themes: RemoteTheme[];
  panels: RemotePanel[];
}

export type PcMsg =
  | { t: 'apps'; apps: AppEntry[] }
  | { t: 'icons'; icons: { id: string; icon: string }[] }
  | { t: 'state'; pinned: boolean; textFocus: boolean; tvMode: boolean; osStyle?: string }
  | { t: 'nowPlaying'; active: boolean; title: string; artist: string; playing: boolean; position: number; duration: number; source: string }
  | { t: 'vaultState'; status: VaultStatus; email: string; autolockMin: number }
  | { t: 'vaultItems'; items: VaultItem[] }
  | { t: 'vaultResult'; op: string; ok: boolean; code: string }
  | { t: 'volume'; level: number; muted: boolean }
  | { t: 'remoteMods'; mods: RemoteMod[] }
  | { t: 'pong' };

/** Contenu du QR code affiché par le PC. */
export interface PairingInfo {
  v: number;
  id: string;
  name: string;
  device: string;
  key: string; // base64, 32 octets
  ips: string[];
  port: number;
  bt?: string | null;
}

/** Flux d'octets ordonné et fiable (TCP ou Bluetooth RFCOMM). */
export interface ByteStream {
  write(data: Uint8Array): Promise<void>;
  /** Prochain bloc reçu ; null si le flux est fermé. */
  read(): Promise<Uint8Array | null>;
  close(): void;
}

export const PROTOCOL_VERSION = 1;
export const TCP_PORT = 47820;
export const BEACON_PORT = 47821;
