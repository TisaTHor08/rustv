// Trames + handshake HMAC + session chiffrée (client). Spécification : voir protocol/ (Rust).
import { Channel, b64, concat, deriveKey, hmac, hmacVerify, randomBytes, unb64, utf8 } from './crypto.ts';
import { PROTOCOL_VERSION, type ByteStream, type PcMsg, type PhoneMsg } from './types.ts';

const MAX_FRAME = 1 << 20;
const dec = new TextDecoder();

/** Réassemble des trames [u32 BE longueur | payload] à partir d'un flux d'octets. */
export class FrameReader {
  private buf: Uint8Array = new Uint8Array(0);
  private stream: ByteStream;
  constructor(stream: ByteStream) {
    this.stream = stream;
  }

  private async fill(n: number): Promise<void> {
    while (this.buf.length < n) {
      const chunk = await this.stream.read();
      if (chunk === null) throw new Error('connexion fermée');
      this.buf = concat(this.buf, chunk);
    }
  }

  async readFrame(): Promise<Uint8Array> {
    await this.fill(4);
    const len = new DataView(this.buf.buffer, this.buf.byteOffset, 4).getUint32(0);
    if (len > MAX_FRAME) throw new Error('trame trop grande');
    await this.fill(4 + len);
    const frame = this.buf.slice(4, 4 + len);
    this.buf = this.buf.slice(4 + len);
    return frame;
  }
}

async function writeFrame(stream: ByteStream, payload: Uint8Array): Promise<void> {
  const out = new Uint8Array(4 + payload.length);
  new DataView(out.buffer).setUint32(0, payload.length);
  out.set(payload, 4);
  await stream.write(out); // un seul write
}

const sendJson = (s: ByteStream, v: unknown) => writeFrame(s, utf8(JSON.stringify(v)));

export class Session {
  private stream: ByteStream;
  private reader: FrameReader;
  private tx: Channel;
  private rx: Channel;
  private constructor(stream: ByteStream, reader: FrameReader, tx: Channel, rx: Channel) {
    this.stream = stream;
    this.reader = reader;
    this.tx = tx;
    this.rx = rx;
  }

  /** Handshake mutuel : le PC doit prouver qu'il connaît la clé avant que nous prouvions la nôtre. */
  static async connect(stream: ByteStream, key: Uint8Array, deviceId: string): Promise<Session> {
    const reader = new FrameReader(stream);
    const nc = randomBytes(32);
    await sendJson(stream, { v: PROTOCOL_VERSION, device_id: deviceId, nc: b64(nc) });

    const ch = JSON.parse(dec.decode(await reader.readFrame()));
    if (typeof ch.ns !== 'string' || typeof ch.proof_s !== 'string') throw new Error('appareil refusé par le PC');
    const ns = unb64(ch.ns);
    const id = utf8(deviceId);
    if (!(await hmacVerify(key, [utf8('S'), id, nc, ns], unb64(ch.proof_s)))) {
      throw new Error('le serveur ne connaît pas la clé');
    }
    await sendJson(stream, { proof_c: b64(await hmac(key, utf8('C'), id, nc, ns)) });
    const verdict = JSON.parse(dec.decode(await reader.readFrame()));
    if (!verdict.ok) throw new Error('refusé par le PC');

    const salt = concat(nc, ns);
    const tx = new Channel(await deriveKey(key, salt, 'c2s'));
    const rx = new Channel(await deriveKey(key, salt, 's2c'));
    return new Session(stream, reader, tx, rx);
  }

  private chain: Promise<void> = Promise.resolve();

  /** Envois sérialisés : le compteur de nonce doit suivre exactement l'ordre d'écriture. */
  send(msg: PhoneMsg): Promise<void> {
    const run = async () => {
      await writeFrame(this.stream, await this.tx.seal(utf8(JSON.stringify(msg))));
    };
    const p = this.chain.then(run);
    this.chain = p.catch(() => {});
    return p;
  }

  async recv(): Promise<PcMsg> {
    const ct = await this.reader.readFrame();
    const pt = await this.rx.open(ct);
    if (!pt) throw new Error('déchiffrement échoué (altération ou rejeu)');
    return JSON.parse(dec.decode(pt));
  }

  close(): void {
    this.stream.close();
  }
}
