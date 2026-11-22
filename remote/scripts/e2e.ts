// Test d'intégration Node : ce même code de protocole TypeScript contre le vrai service Rust.
// Usage : node scripts/e2e.ts <host:port> <deviceId> <keyBase64>
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

const [hostport, deviceId, key] = process.argv.slice(2);
const [host, port] = hostport.split(':');
const sock = net.connect(Number(port), host);
await new Promise((r) => sock.once('connect', r));
try {
  const s = await Session.connect(wrap(sock), unb64(key), deviceId);
  console.log('authentifié');
  await s.send({ t: 'ping' });
  let icons = 0;
  for (;;) {
    const m = await s.recv();
    if (m.t === 'icons') icons += m.icons.length;
    else if (m.t === 'apps') console.log(`apps: ${m.apps.length}`);
    else if (m.t === 'state') console.log('state', JSON.stringify(m));
    else if (m.t === 'pong') {
      console.log('pong');
      break;
    }
  }
  // laisse arriver quelques lots d'icônes
  const t0 = Date.now();
  while (Date.now() - t0 < 4000 && icons < 5) {
    const m = await Promise.race([s.recv(), new Promise<null>((r) => setTimeout(() => r(null), 1000))]);
    if (m && m.t === 'icons') icons += m.icons.length;
  }
  console.log(`icônes reçues: ${icons}`);
  s.close();
} catch (e) {
  console.log('REFUSE:', (e as Error).message);
  process.exit(2);
}
process.exit(0);
