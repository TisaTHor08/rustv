// Test d'intégration Node : le transport WebSocket du navigateur (native.ts) contre le vrai serveur Rust.
// Lancer d'abord : cargo run -p rustv-core --example webremote_dev -- remote/dist
// Usage : node scripts/e2e-ws.ts <host:port> <deviceId> <keyBase64> [software]
//   « software » force le chiffrement JS pur (contexte HTTP sans crypto.subtle).
import { forceSoftwareCrypto, softwareCrypto, unb64 } from '../src/lib/protocol/crypto.ts';
import { Session } from '../src/lib/protocol/session.ts';
import { wsConnect } from '../src/lib/native.ts';

const [addr, deviceId, key, mode] = process.argv.slice(2);
if (mode === 'software') forceSoftwareCrypto(true);
console.log(`chiffrement : ${softwareCrypto() ? 'JS pur (@noble)' : 'WebCrypto'}`);

// 1. Appareil jumelé : handshake, ping/pong, flux d'état.
const s = await Session.connect(await wsConnect(`ws://${addr}/ws`), unb64(key), deviceId);
console.log('authentifié');
await s.send({ t: 'ping' });
const seen: string[] = [];
for (;;) {
  const m = await s.recv();
  seen.push(m.t);
  if (m.t === 'pong') break;
}
console.log(`reçu : ${seen.join(', ')}`);
s.close();

// 2. Appareil inconnu et mauvaise clé : refus.
for (const [id, k] of [['intrus', unb64(key)], [deviceId, new Uint8Array(32)]] as const) {
  try {
    await Session.connect(await wsConnect(`ws://${addr}/ws`), k, id);
    console.error(`ÉCHEC : ${id} accepté`);
    process.exit(1);
  } catch (e) {
    console.log(`refusé comme attendu (${id}) : ${(e as Error).message}`);
  }
}
console.log('ok');
process.exit(0);
