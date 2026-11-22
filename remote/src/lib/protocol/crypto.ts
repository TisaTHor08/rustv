// HMAC-SHA256, HKDF et canal AES-256-GCM (WebCrypto) — identique au côté Rust.
const subtle = globalThis.crypto.subtle;
const enc = new TextEncoder();

export const b64 = (b: Uint8Array): string => {
  let s = '';
  for (const x of b) s += String.fromCharCode(x);
  return btoa(s);
};
export const unb64 = (s: string): Uint8Array => Uint8Array.from(atob(s), (c) => c.charCodeAt(0));

export function concat(...parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let o = 0;
  for (const p of parts) {
    out.set(p, o);
    o += p.length;
  }
  return out;
}

export const utf8 = (s: string): Uint8Array => enc.encode(s);

export function randomBytes(n: number): Uint8Array {
  return globalThis.crypto.getRandomValues(new Uint8Array(n));
}

export async function hmac(key: Uint8Array, ...parts: Uint8Array[]): Promise<Uint8Array> {
  const k = await subtle.importKey('raw', key as BufferSource, { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
  return new Uint8Array(await subtle.sign('HMAC', k, concat(...parts) as BufferSource));
}

/** Comparaison en temps constant (les deux tags ont 32 octets). */
export async function hmacVerify(key: Uint8Array, parts: Uint8Array[], tag: Uint8Array): Promise<boolean> {
  const expected = await hmac(key, ...parts);
  if (expected.length !== tag.length) return false;
  let diff = 0;
  for (let i = 0; i < expected.length; i++) diff |= expected[i] ^ tag[i];
  return diff === 0;
}

export async function deriveKey(secret: Uint8Array, salt: Uint8Array, info: string): Promise<CryptoKey> {
  const base = await subtle.importKey('raw', secret as BufferSource, 'HKDF', false, ['deriveKey']);
  return subtle.deriveKey(
    { name: 'HKDF', hash: 'SHA-256', salt: salt as BufferSource, info: utf8(info) as BufferSource },
    base,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt'],
  );
}

/** Un sens : le compteur strictement croissant sert de nonce (octets 4..12, big-endian). */
export class Channel {
  private counter = 0n;
  private key: CryptoKey;
  constructor(key: CryptoKey) {
    this.key = key;
  }

  private iv(): Uint8Array {
    const iv = new Uint8Array(12);
    new DataView(iv.buffer).setBigUint64(4, this.counter);
    return iv;
  }
  async seal(plain: Uint8Array): Promise<Uint8Array> {
    const ct = await subtle.encrypt({ name: 'AES-GCM', iv: this.iv() as BufferSource }, this.key, plain as BufferSource);
    this.counter++;
    return new Uint8Array(ct);
  }
  async open(ct: Uint8Array): Promise<Uint8Array | null> {
    try {
      const pt = await subtle.decrypt({ name: 'AES-GCM', iv: this.iv() as BufferSource }, this.key, ct as BufferSource);
      this.counter++;
      return new Uint8Array(pt);
    } catch {
      return null;
    }
  }
}
