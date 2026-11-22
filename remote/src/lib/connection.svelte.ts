// Gestion de la connexion : course entre toutes les voies, reconnexion, ping, état réactif.
import { btConnect, isNative, listenBeacon, tcpConnect } from './native.ts';
import { unb64 } from './protocol/crypto.ts';
import { Session } from './protocol/session.ts';
import { BEACON_PORT, type AppEntry, type PairingInfo, type PhoneMsg } from './protocol/types.ts';

const RFCOMM_UUID = '7c1d3a52-5b0e-4f8a-9d63-2e41c0a8b9f7';
const BT_DELAY_MS = 1500; // le Wi-Fi est plus rapide : le Bluetooth ne prend le relais que s'il tarde
const RACE_TIMEOUT_MS = 20000;
const PING_MS = 4000;
const DEAD_MS = 12000;

export type Status = 'idle' | 'connecting' | 'connected';

export const app = $state({
  pairing: loadPairing() as PairingInfo | null,
  status: 'idle' as Status,
  via: '',
  error: '',
  pinned: false,
  textFocus: false,
  tvMode: false,
  now: { active: false, title: '', artist: '', playing: false, position: 0, duration: 0, source: '' },
  apps: [] as AppEntry[],
});

function loadPairing(): PairingInfo | null {
  try {
    const s = localStorage.getItem('rustv.pairing');
    return s ? (JSON.parse(s) as PairingInfo) : null;
  } catch {
    return null;
  }
}

export function savePairing(p: PairingInfo) {
  app.pairing = p;
  try {
    localStorage.setItem('rustv.pairing', JSON.stringify(p));
  } catch {}
}

export function unpair() {
  deactivate();
  app.pairing = null;
  try {
    localStorage.removeItem('rustv.pairing');
    localStorage.removeItem('rustv.lastIp');
  } catch {}
}

/** Valide le contenu d'un QR code. */
export function parsePairing(text: string): PairingInfo | null {
  try {
    const p = JSON.parse(text) as PairingInfo;
    if (p.v !== 1 || !p.id || !p.device || !Array.isArray(p.ips)) return null;
    if (unb64(p.key).length !== 32) return null;
    return p;
  } catch {
    return null;
  }
}

// ---- course des voies ----

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));
const lastIp = () => {
  try {
    return localStorage.getItem('rustv.lastIp');
  } catch {
    return null;
  }
};

interface Won {
  session: Session;
  via: string;
}

async function race(p: PairingInfo, isActive: () => boolean): Promise<Won> {
  const key = unb64(p.key);
  return new Promise<Won>((resolve, reject) => {
    let done = false;
    let pending = 0;
    const tried = new Set<string>();
    let stopBeacon = () => {};
    const timer = setTimeout(() => finish(new Error('aucune voie disponible')), RACE_TIMEOUT_MS);

    function finish(err?: Error, won?: Won) {
      if (done) {
        won?.session.close(); // une autre voie a déjà gagné
        return;
      }
      done = true;
      clearTimeout(timer);
      stopBeacon();
      if (won) resolve(won);
      else reject(err);
    }

    function attempt(label: string, open: () => Promise<import('./protocol/types.ts').ByteStream>) {
      pending++;
      (async () => {
        let stream;
        try {
          stream = await open();
          const session = await Session.connect(stream, key, p.device);
          finish(undefined, { session, via: label });
        } catch (e) {
          stream?.close();
          if (--pending === 0 && beaconDone) finish(e as Error);
        }
      })();
    }

    const tcp = (ip: string, label: string) => {
      if (tried.has(ip)) return;
      tried.add(ip);
      attempt(label, () => tcpConnect(ip, p.port));
    };

    let beaconDone = false;
    const last = lastIp();
    if (last) tcp(last, `Wi-Fi ${last}`);
    for (const ip of p.ips) tcp(ip, `Wi-Fi ${ip}`);

    if (p.bt && isNative) {
      setTimeout(() => {
        if (!done && isActive()) attempt('Bluetooth', () => btConnect(p.bt!, RFCOMM_UUID));
      }, BT_DELAY_MS);
    }

    listenBeacon(BEACON_PORT, (ip, payload) => {
      try {
        if (JSON.parse(payload).id === p.id) tcp(ip, `Wi-Fi ${ip} (découverte)`);
      } catch {}
    })
      .then((stop) => {
        if (done) stop();
        else stopBeacon = stop;
      })
      .catch(() => {})
      .finally(() => {
        beaconDone = true;
      });
  });
}

// ---- boucle principale ----

let session: Session | null = null;
let runId = 0;

export function activate() {
  const p = app.pairing;
  if (!p || app.status !== 'idle') return;
  const myRun = ++runId;
  const active = () => runId === myRun;
  app.status = 'connecting';
  app.error = '';

  (async () => {
    let backoff = 500;
    while (active()) {
      try {
        app.status = 'connecting';
        const won = await race(p, active);
        if (!active()) {
          won.session.close();
          return;
        }
        session = won.session;
        app.via = won.via;
        app.status = 'connected';
        app.error = '';
        backoff = 500;
        const ip = won.via.match(/(\d+\.\d+\.\d+\.\d+)/)?.[1];
        if (ip) {
          try {
            localStorage.setItem('rustv.lastIp', ip);
          } catch {}
        }
        await serve(won.session, active);
      } catch (e) {
        app.error = (e as Error).message;
      }
      session = null;
      if (!active()) return;
      app.status = 'connecting'; // bascule de voie : on relance aussitôt la course
      await sleep(backoff);
      backoff = Math.min(backoff * 2, 4000);
    }
  })();
}

async function serve(s: Session, active: () => boolean) {
  let lastRx = Date.now();
  const ping = setInterval(() => {
    if (Date.now() - lastRx > DEAD_MS) s.close();
    else void s.send({ t: 'ping' }).catch(() => s.close());
  }, PING_MS);
  try {
    while (active()) {
      const m = await s.recv();
      lastRx = Date.now();
      if (m.t === 'state') {
        app.pinned = m.pinned;
        app.textFocus = m.textFocus;
        app.tvMode = m.tvMode;
      } else if (m.t === 'nowPlaying') {
        app.now = { active: m.active, title: m.title, artist: m.artist, playing: m.playing, position: m.position, duration: m.duration, source: m.source };
      } else if (m.t === 'apps') app.apps = m.apps;
      else if (m.t === 'icons') {
        // Les icônes arrivent par lots après la liste : on les rattache aux apps.
        const byId = new Map(m.icons.map((i) => [i.id, i.icon]));
        app.apps = app.apps.map((a) => (byId.has(a.id) ? { ...a, icon: byId.get(a.id)! } : a));
      }
    }
  } finally {
    clearInterval(ping);
    s.close();
    app.textFocus = false;
  }
}

export function deactivate() {
  runId++;
  session?.close();
  session = null;
  app.status = 'idle';
  app.via = '';
}

export function send(msg: PhoneMsg) {
  void session?.send(msg).catch(() => session?.close());
}
