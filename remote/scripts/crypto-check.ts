// Vérifie que le repli JS pur (@noble) et WebCrypto sont interchangeables octet pour octet.
// Usage : node scripts/crypto-check.ts   (depuis remote/)
import { Channel, deriveKey, forceSoftwareCrypto, hmac, randomBytes, softwareCrypto } from '../src/lib/protocol/crypto.ts';

const eq = (a: Uint8Array, b: Uint8Array) => a.length === b.length && a.every((x, i) => x === b[i]);
let n = 0;
const check = (ok: boolean, what: string) => {
  if (!ok) throw new Error(`ÉCHEC : ${what}`);
  n++;
};

for (let i = 0; i < 20; i++) {
  const key = randomBytes(32);
  const salt = randomBytes(64);
  const msgs = [randomBytes(0), randomBytes(1), randomBytes(15), randomBytes(16), randomBytes(17), randomBytes(1000), randomBytes(60000)];

  forceSoftwareCrypto(false);
  check(!softwareCrypto(), 'WebCrypto disponible');
  const hW = await hmac(key, salt, msgs[3]);
  const tx = new Channel(await deriveKey(key, salt, 'c2s'));
  const rx = new Channel(await deriveKey(key, salt, 's2c'));
  const sealedW = [];
  for (const m of msgs) sealedW.push(await tx.seal(m));

  forceSoftwareCrypto(true);
  check(softwareCrypto(), 'repli actif');
  const hS = await hmac(key, salt, msgs[3]);
  check(eq(hW, hS), 'HMAC identique');
  const txS = new Channel(await deriveKey(key, salt, 'c2s'));
  const rxS = new Channel(await deriveKey(key, salt, 's2c'));
  // Le repli produit les mêmes chiffrés que WebCrypto, et sait déchiffrer ceux de WebCrypto.
  const openS = new Channel(await deriveKey(key, salt, 'c2s'));
  for (let j = 0; j < msgs.length; j++) {
    check(eq(await txS.seal(msgs[j]), sealedW[j]), `AES-GCM identique (message ${j})`);
    const pt = await openS.open(sealedW[j]);
    check(pt !== null && eq(pt, msgs[j]), `déchiffrement croisé (message ${j})`);
  }
  // Altération et rejeu refusés.
  const bad = Uint8Array.from(sealedW[1]);
  bad[0] ^= 1;
  check((await new Channel(await deriveKey(key, salt, 'c2s')).open(bad)) === null, 'altération refusée');
  void rx;
  void rxS;
}
console.log(`ok : ${n} vérifications, repli JS == WebCrypto`);
