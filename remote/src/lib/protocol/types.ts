export type KeyName = 'up' | 'down' | 'left' | 'right' | 'enter' | 'back' | 'backlong' | 'menu';
export type AltTabAction = 'open' | 'left' | 'right' | 'confirm';

export type PhoneMsg =
  | { t: 'key'; k: KeyName }
  | { t: 'altTab'; a: AltTabAction }
  | { t: 'pin' }
  | { t: 'text'; backspaces: number; text: string }
  | { t: 'mouse'; dx: number; dy: number; click?: 'left' | 'right'; scroll?: number }
  | { t: 'launch'; appId: string }
  | { t: 'getApps' }
  | { t: 'home' }
  | { t: 'tv'; on: boolean }
  | { t: 'media'; a: MediaAction }
  | { t: 'ping' };

export type MediaAction = 'playPause' | 'next' | 'prev' | 'stop' | 'volUp' | 'volDown' | 'mute';

export interface AppEntry {
  id: string;
  name: string;
  icon: string;
}

export type PcMsg =
  | { t: 'apps'; apps: AppEntry[] }
  | { t: 'icons'; icons: { id: string; icon: string }[] }
  | { t: 'state'; pinned: boolean; textFocus: boolean; tvMode: boolean }
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
