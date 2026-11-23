// Gestion de la connexion : course entre toutes les voies, reconnexion, ping, état réactif.
import { btConnect, isBrowser, isNative, listenBeacon, tcpConnect, webSocketUrl, wsConnect } from './native.ts';
import { unb64 } from './protocol/crypto.ts';
import { applyState, resetOnDisconnect } from './protocol/appstate.ts';
import { Session } from './protocol/session.ts';
import { BEACON_PORT, type AppEntry, type PairingInfo, type PhoneMsg, type RemoteMod, type VaultItem, type VaultStatus } from './protocol/types.ts';

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
  /** Vrai dès que la télécommande a été connectée (jusqu'à l'arrêt manuel) : l'interface reste affichée pendant une reconnexion. */
  wasConnected: false,
  tvMode: false,
  /** Style d'accueil TV annoncé par le PC (`rustv` si absent) : choisit le modèle de télécommande en mode auto. */
  osStyle: 'rustv',
  now: { active: false, title: '', artist: '', playing: false, position: 0, duration: 0, source: '' },
  volume: { level: 0, muted: false, known: false },
  apps: [] as AppEntry[],
  remoteMods: [] as RemoteMod[],
  vault: { status: 'unknown' as VaultStatus, email: '', autolockMin: 5, items: [] as VaultItem[], seq: 0, op: '', ok: true, code: '' },
});

function resetVault() {
  app.vault = { status: 'unknown', email: '', autolockMin: 5, items: [], seq: 0, op: '', ok: true, code: '' };
}

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

function fromB64Url(s: string): Uint8Array {
  const b = s.replace(/-/g, '+').replace(/_/g, '/');
  return unb64(b + '='.repeat((4 - (b.length % 4)) % 4));
}

/**
 * Jumelage à partir de n'importe quelle forme : JSON du QR du PC, lien `…/#p=<code>` (QR « navigateur ») ou code seul.
 * Le code est le JSON en base64url : il se copie/colle, y compris quand la caméra est indisponible (HTTP).
 */
export function pairingFromText(text: string): PairingInfo | null {
  const s = text.trim();
  const m = s.match(/[#&?]p=([A-Za-z0-9_-]+)/);
  const code = m ? m[1] : /^[A-Za-z0-9_-]{40,}$/.test(s) ? s : null;
  if (code) {
    try {
      return parsePairing(new TextDecoder().decode(fromB64Url(code)));
    } catch {
      return null;
    }
  }
  return parsePairing(s);
}

/** Navigateur : un lien `#p=…` (QR scanné avec l'appareil photo) jumelle aussitôt ; la clé est effacée de l'adresse. */
export function consumeHashPairing() {
  if (!isBrowser || !location.hash.includes('p=')) return;
  const p = pairingFromText(location.hash);
  if (p) savePairing(p);
  try {
    history.replaceState(null, '', location.pathname + location.search);
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
          if (--pending === 0 && (beaconDone || isBrowser)) finish(e as Error);
        }
      })();
    }

    const tcp = (ip: string, label: string) => {
      if (tried.has(ip)) return;
      tried.add(ip);
      attempt(label, () => tcpConnect(ip, p.port));
    };

    let beaconDone = false;
    if (isBrowser) {
      // Navigateur : une seule voie, le WebSocket du PC qui a servi la page (mêmes trames et chiffrement que le TCP).
      attempt('Navigateur', () => wsConnect(webSocketUrl()));
      return;
    }
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
  try {
    localStorage.removeItem('rustv.manualOff');
  } catch {}
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
        app.wasConnected = true;
        app.error = '';
        backoff = 500;
        const ip = won.via.match(/(\d+\.\d+\.\d+\.\d+)/)?.[1];
        if (ip) {
          try {
            localStorage.setItem('rustv.lastIp', ip);
          } catch {}
        }
        void won.session.send({ t: 'getRemoteMods' }).catch(() => {});
        await serve(won.session, active);
      } catch (e) {
        app.error = (e as Error).message;
        // Appareil révoqué ou jamais jumelé : insister ne sert à rien et le PC bloque les échecs répétés.
        if (isBrowser && /refusé/.test(app.error) && active()) {
          deactivate();
          return;
        }
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
  let lastTick = Date.now();
  const ping = setInterval(() => {
    const now = Date.now();
    // L'appli a été suspendue (écran éteint, autre appli au premier plan) : les minuteurs de la WebView étaient arrêtés,
    // pas la liaison. On repart d'une fenêtre neuve au lieu de couper une connexion probablement saine.
    if (now - lastTick > PING_MS * 2) lastRx = now;
    lastTick = now;
    if (now - lastRx > DEAD_MS) s.close();
    else void s.send({ t: 'ping' }).catch(() => s.close());
  }, PING_MS);
  // Retour au premier plan : un ping immédiat vérifie la liaison tout de suite (échec = reconnexion rapide).
  const onVisible = () => {
    if (document.visibilityState !== 'visible') return;
    lastRx = Date.now();
    lastTick = lastRx;
    void s.send({ t: 'ping' }).catch(() => s.close());
  };
  document.addEventListener('visibilitychange', onVisible);
  try {
    while (active()) {
      const m = await s.recv();
      lastRx = Date.now();
      if (m.t === 'state') {
        applyState(app, m);
      } else if (m.t === 'volume') {
        app.volume = { level: m.level, muted: m.muted, known: true };
      } else if (m.t === 'nowPlaying') {
        app.now = { active: m.active, title: m.title, artist: m.artist, playing: m.playing, position: m.position, duration: m.duration, source: m.source };
      } else if (m.t === 'vaultState') {
        app.vault.status = m.status;
        app.vault.email = m.email;
        app.vault.autolockMin = m.autolockMin;
      } else if (m.t === 'vaultItems') app.vault.items = m.items;
      else if (m.t === 'vaultResult') {
        app.vault.op = m.op;
        app.vault.ok = m.ok;
        app.vault.code = m.code;
        app.vault.seq++;
      } else if (m.t === 'remoteMods') app.remoteMods = Array.isArray(m.mods) ? m.mods : [];
      else if (m.t === 'apps') app.apps = m.apps;
      else if (m.t === 'icons') {
        // Les icônes arrivent par lots après la liste : on les rattache aux apps.
        const byId = new Map(m.icons.map((i) => [i.id, i.icon]));
        app.apps = app.apps.map((a) => (byId.has(a.id) ? { ...a, icon: byId.get(a.id)! } : a));
      }
    }
  } finally {
    clearInterval(ping);
    document.removeEventListener('visibilitychange', onVisible);
    s.close();
    // Plus de liaison : l'état « champ actif » est périmé, la reconnexion renverra l'état réel.
    resetOnDisconnect(app);
    resetVault();
  }
}

/** Arrêt volontaire (bouton Annuler) : on ne se reconnecte plus tout seul à l'ouverture suivante. */
export function stopByUser() {
  try {
    localStorage.setItem('rustv.manualOff', '1');
  } catch {}
  deactivate();
}

export function deactivate() {
  runId++;
  session?.close();
  session = null;
  app.status = 'idle';
  app.wasConnected = false;
  app.via = '';
  resetOnDisconnect(app);
}

export function send(msg: PhoneMsg) {
  void session?.send(msg).catch(() => session?.close());
}
