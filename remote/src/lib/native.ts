// Pont vers le plugin natif Capacitor « RusTVTransport » (TCP, RFCOMM, beacon UDP).
import { Capacitor, registerPlugin } from '@capacitor/core';
import { b64, unb64 } from './protocol/crypto.ts';
import type { ByteStream } from './protocol/types.ts';

interface RusTVTransportPlugin {
  tcpConnect(o: { host: string; port: number; timeoutMs: number }): Promise<{ id: number }>;
  btConnect(o: { address: string; uuid: string }): Promise<{ id: number }>;
  write(o: { id: number; data: string }): Promise<void>;
  close(o: { id: number }): Promise<void>;
  startBeacon(o: { port: number }): Promise<void>;
  stopBeacon(): Promise<void>;
  addListener(ev: 'data', cb: (e: { id: number; data: string }) => void): Promise<{ remove(): Promise<void> }>;
  addListener(ev: 'closed', cb: (e: { id: number }) => void): Promise<{ remove(): Promise<void> }>;
  addListener(ev: 'beacon', cb: (e: { ip: string; payload: string }) => void): Promise<{ remove(): Promise<void> }>;
}

export const isNative = Capacitor.isNativePlatform();
/** Vrai dans un navigateur (PWA / page servie par le PC) : transport WebSocket au lieu du TCP natif. */
export const isBrowser = !isNative;
const Native = registerPlugin<RusTVTransportPlugin>('RusTVTransport');

class NativeStream implements ByteStream {
  private queue: (Uint8Array | null)[] = [];
  private waiter: (() => void) | null = null;
  private closed = false;
  private id: number;
  constructor(id: number) {
    this.id = id;
  }

  push(v: Uint8Array | null) {
    if (v === null) this.closed = true;
    this.queue.push(v);
    this.waiter?.();
  }
  async write(data: Uint8Array) {
    await Native.write({ id: this.id, data: b64(data) });
  }
  async read(): Promise<Uint8Array | null> {
    while (this.queue.length === 0) await new Promise<void>((r) => (this.waiter = r));
    const v = this.queue.shift()!;
    if (v === null) this.queue.push(null); // la fin de flux reste visible pour les lectures suivantes
    return v;
  }
  close() {
    if (!this.closed) void Native.close({ id: this.id }).catch(() => {});
  }
}

const streams = new Map<number, NativeStream>();
const early = new Map<number, (Uint8Array | null)[]>(); // données arrivées avant la création du flux
let wired = false;

function wire() {
  if (wired || !isNative) return;
  wired = true;
  void Native.addListener('data', (e) => {
    const bytes = unb64(e.data);
    const s = streams.get(e.id);
    if (s) s.push(bytes);
    else early.set(e.id, [...(early.get(e.id) ?? []), bytes]);
  });
  void Native.addListener('closed', (e) => {
    const s = streams.get(e.id);
    if (s) {
      s.push(null);
      streams.delete(e.id);
    } else early.set(e.id, [...(early.get(e.id) ?? []), null]);
  });
}

function adopt(id: number): ByteStream {
  const s = new NativeStream(id);
  streams.set(id, s);
  for (const v of early.get(id) ?? []) s.push(v);
  early.delete(id);
  return s;
}

export async function tcpConnect(host: string, port: number, timeoutMs = 4000): Promise<ByteStream> {
  wire();
  const { id } = await Native.tcpConnect({ host, port, timeoutMs });
  return adopt(id);
}

export async function btConnect(address: string, uuid: string): Promise<ByteStream> {
  wire();
  const { id } = await Native.btConnect({ address, uuid });
  return adopt(id);
}

/** Écoute le beacon UDP du PC ; renvoie une fonction d'arrêt. */
export async function listenBeacon(port: number, cb: (ip: string, payload: string) => void): Promise<() => void> {
  if (!isNative) return () => {};
  const h = await Native.addListener('beacon', (e) => cb(e.ip, e.payload));
  await Native.startBeacon({ port });
  return () => {
    void h.remove();
    void Native.stopBeacon().catch(() => {});
  };
}

/** Adresse du WebSocket du PC qui a servi cette page (même origine : `ws://` en HTTP, `wss://` en HTTPS). */
export function webSocketUrl(): string {
  return `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/ws`;
}

class WsStream implements ByteStream {
  private queue: (Uint8Array | null)[] = [];
  private waiter: (() => void) | null = null;
  private ws: WebSocket;
  constructor(ws: WebSocket) {
    this.ws = ws;
    ws.binaryType = 'arraybuffer';
    ws.onmessage = (e) => this.push(new Uint8Array(e.data as ArrayBuffer));
    ws.onclose = ws.onerror = () => this.push(null);
  }
  private push(v: Uint8Array | null) {
    this.queue.push(v);
    this.waiter?.();
  }
  async write(data: Uint8Array) {
    if (this.ws.readyState !== WebSocket.OPEN) throw new Error('connexion fermée');
    this.ws.send(data as BufferSource);
  }
  async read(): Promise<Uint8Array | null> {
    while (this.queue.length === 0) await new Promise<void>((r) => (this.waiter = r));
    const v = this.queue.shift()!;
    if (v === null) this.queue.push(null); // la fin de flux reste visible
    return v;
  }
  close() {
    try {
      this.ws.close();
    } catch {}
  }
}

/** Flux d'octets au-dessus d'un WebSocket : mêmes trames et même chiffrement que le TCP (voir docs/WEB-REMOTE.md). */
export function wsConnect(url: string, timeoutMs = 5000): Promise<ByteStream> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    const timer = setTimeout(() => {
      ws.close();
      reject(new Error('délai dépassé'));
    }, timeoutMs);
    ws.onopen = () => {
      clearTimeout(timer);
      resolve(new WsStream(ws));
    };
    ws.onerror = () => {
      clearTimeout(timer);
      reject(new Error('PC injoignable'));
    };
  });
}
