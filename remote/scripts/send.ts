// Envoie des messages au PC comme le ferait le téléphone (pour les tests manuels).
// Usage : node scripts/send.ts <host:port> <deviceId> <keyBase64> '<json>' ['<json>' ...]
// Chaque message est suivi d'une pause de 700 ms ; les réponses reçues sont affichées.
import net from 'node:net';
import { Session } from '../src/lib/protocol/session.ts';
import { unb64 } from '../src/lib/protocol/crypto.ts';
import type { ByteStream } from '../src/lib/protocol/types.ts';

function wrap(sock: net.Socket): ByteStream {
  const queue: (Uint8Array | null)[] = [];
  let wake: (() => void) | null = null;
  const push = (v: Uint8Array | null) => {
    queue.push(v);
    wake?.();
  };
  sock.on('data', (d) => push(new Uint8Array(d)));
  sock.on('close', () => push(null));
  sock.on('error', () => push(null));
  return {
    write: (d) => new Promise((res, rej) => sock.write(d, (e) => (e ? rej(e) : res()))),
    async read() {
      while (queue.length === 0) await new Promise<void>((r) => (wake = r));
      return queue.shift()!;
    },
    close: () => sock.destroy(),
  };
}

const [hostport, deviceId, key, ...msgs] = process.argv.slice(2);
const [host, port] = hostport.split(':');
const sock = net.connect(Number(port), host);
await new Promise((r) => sock.once('connect', r));
const s = await Session.connect(wrap(sock), unb64(key), deviceId);
(async () => {
  try {
    for (;;) {
      const m = await s.recv();
      if (m.t === 'state' || m.t === 'nowPlaying') console.log('<-', JSON.stringify(m));
    }
  } catch {}
})();
for (const m of msgs) {
  await s.send(JSON.parse(m));
  console.log('->', m);
  await new Promise((r) => setTimeout(r, 700));
}
await new Promise((r) => setTimeout(r, 800));
s.close();
process.exit(0);
