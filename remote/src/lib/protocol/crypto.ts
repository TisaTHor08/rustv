// HMAC-SHA256, HKDF et canal AES-256-GCM — identique au côté Rust.
// WebCrypto (`crypto.subtle`) quand il existe ; sinon repli sur @noble (JS pur, audité). `crypto.subtle` n'existe que
// dans un « contexte sécurisé » (HTTPS ou localhost) : la télécommande web servie en HTTP sur le LAN en a besoin.
import { gcm } from '@noble/ciphers/aes.js';
import { hkdf } from '@noble/hashes/hkdf.js';
import { hmac as nobleHmac } from '@noble/hashes/hmac.js';
import { sha256 } from '@noble/hashes/sha2.js';

let subtle: SubtleCrypto | undefined = globalThis.crypto?.subtle;
/** Vrai quand WebCrypto est absent (contexte non sécurisé) : le chiffrement tourne alors en JS pur. */
export const softwareCrypto = (): boolean => !subtle;
/** Tests : force le repli JS pur (`true`) ou rend WebCrypto s'il existe (`false`). */
export function forceSoftwareCrypto(on: boolean): void {
  subtle = on ? undefined : globalThis.crypto?.subtle;
}
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
  if (!subtle) return nobleHmac(sha256, key, concat(...parts));
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

/** Clé de sens : WebCrypto si disponible, sinon octets bruts pour le repli JS. */
export type AeadKey = { web: CryptoKey } | { raw: Uint8Array };

export async function deriveKey(secret: Uint8Array, salt: Uint8Array, info: string): Promise<AeadKey> {
  if (!subtle) return { raw: hkdf(sha256, secret, salt, utf8(info), 32) };
  const base = await subtle.importKey('raw', secret as BufferSource, 'HKDF', false, ['deriveKey']);
  const web = await subtle.deriveKey(
    { name: 'HKDF', hash: 'SHA-256', salt: salt as BufferSource, info: utf8(info) as BufferSource },
    base,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt'],
  );
  return { web };
}

/** Un sens : le compteur strictement croissant sert de nonce (octets 4..12, big-endian). */
export class Channel {
  private counter = 0n;
  private key: AeadKey;
  constructor(key: AeadKey) {
    this.key = key;
  }

  private iv(): Uint8Array {
    const iv = new Uint8Array(12);
    new DataView(iv.buffer).setBigUint64(4, this.counter);
    return iv;
  }
  async seal(plain: Uint8Array): Promise<Uint8Array> {
    const iv = this.iv();
    const ct =
      'web' in this.key
        ? new Uint8Array(await subtle!.encrypt({ name: 'AES-GCM', iv: iv as BufferSource }, this.key.web, plain as BufferSource))
        : gcm(this.key.raw, iv).encrypt(plain);
    this.counter++;
    return ct;
  }
  async open(ct: Uint8Array): Promise<Uint8Array | null> {
    try {
      const iv = this.iv();
      const pt =
        'web' in this.key
          ? new Uint8Array(await subtle!.decrypt({ name: 'AES-GCM', iv: iv as BufferSource }, this.key.web, ct as BufferSource))
          : gcm(this.key.raw, iv).decrypt(ct);
      this.counter++;
      return pt;
    } catch {
      return null;
    }
  }
}
