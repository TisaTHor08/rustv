// Harnais de développement : faux Tauri (invoke) avec des données de démonstration, pour voir l'interface dans un
// navigateur ordinaire. Paramètres d'URL : os=<style>, keys=ArrowDown,ArrowRight,Enter (rejoués après le chargement),
// theme=light, scale=110, lang=en, cont=0 (sans « Reprendre »).
// screen=settings|display|search|quick|osd|mute|ambient|tilemenu|pairing|toast|launch : ouvre cet écran après le chargement (après keys).
// q=<texte> : saisit ce texte dans la recherche. weather=1 : météo fictive activée. osd/ambient : voir Home.svelte (__rustvHome).
import catalog from '../../crates/rustv-shell/src/catalog.json';

const q = new URLSearchParams(location.search);
(window as any).__RUSTV_MOCK__ = true;
const prefsOver: Record<string, unknown> = {};
if (q.get('weather')) prefsOver.weather = { enabled: true, city: 'Paris', lat: 48.85, lon: 2.35, unit: 'c' };
const outputs = [
  { id: 'o1', name: 'Haut-parleurs (TV HDMI)', default: true, volume: 42, muted: false },
  { id: 'o2', name: 'Casque Bluetooth', default: false, volume: 30, muted: false },
];
const listeners = new Map<string, number[]>();
const callbacks = new Map<number, (e: unknown) => void>();
if (q.get('os')) prefsOver.osStyle = q.get('os');
if (q.get('theme')) prefsOver.theme = q.get('theme');
if (q.get('lang')) prefsOver.lang = q.get('lang');
if (q.get('scale')) prefsOver.uiScale = +q.get('scale')!;
if (q.get('accent')) prefsOver.accent = q.get('accent');
if (q.get('wp')) prefsOver.widgetPos = q.get('wp');

const state = {
  tv: true,
  monitors: [],
  settings: { blackout: false, urls: {}, monitor: null },
  catalog,
  devices: [{ id: 'd1', name: 'Pixel', used: true, connected: true }],
  pcName: 'PC-SALON',
  version: '1.2.0',
  autostart: 'off',
};
const cont = q.get('cont') === '0' ? [] : [
  { url: 'file:///films/Big.Buck.Bunny.mkv', title: 'Big Buck Bunny', kind: 'video', position: 310, duration: 596, poster: null },
  { url: 'file:///films/Sintel.mkv', title: 'Sintel', kind: 'video', position: 900, duration: 888 * 2, poster: null },
  { url: 'file:///music/album.flac', title: 'Concerto en ré', kind: 'audio', position: 60, duration: 1200, poster: null },
];

let cb = 1;
(window as any).__TAURI_INTERNALS__ = {
  transformCallback: (fn?: (e: unknown) => void) => {
    const id = cb++;
    if (fn) callbacks.set(id, fn);
    return id;
  },
  unregisterCallback: () => {},
  convertFileSrc: (p: string) => p,
  invoke: async (cmd: string, args?: any) => {
    switch (cmd) {
      case 'get_state': return state;
      case 'prefs_get': return prefsOver;
      case 'prefs_set': return null;
      case 'mods_list': return [];
      case 'continue_list': return cont;
      case 'update_check': return { available: false, latest: '', current: '1.2.0' };
      case 'plugin:event|listen': {
        const ids = listeners.get(args.event) ?? [];
        ids.push(args.handler);
        listeners.set(args.event, ids);
        return args.handler;
      }
      case 'audio_outputs': return outputs;
      case 'audio_set_default': outputs.forEach((o) => (o.default = o.id === args.id)); return outputs;
      case 'audio_set_output_volume': { const o = outputs.find((x) => x.id === args.id); if (o) o.volume = args.level; return null; }
      case 'display_info': return { monitors: [{ index: 0, name: 'TV', width: 3840, height: 2160, refresh: 60, hdr: { supported: true, enabled: false }, modes: [{ width: 3840, height: 2160, refresh: 60 }, { width: 1920, height: 1080, refresh: 60 }] }], tvMonitor: 0, pendingRevertSeconds: null };
      case 'media_url': return '';
      default: return null;
    }
  },
};
(window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };

await import('./main');

const keys = (q.get('keys') ?? '').split(',').filter(Boolean);
let i = 0;
const step = () => {
  if (i >= keys.length) return;
  const key = keys[i++];
  window.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  setTimeout(step, 250);
};
setTimeout(step, 1200);

// ?dbg=<selector>,... : écrit les rectangles des éléments dans document.title (lu avec --dump-dom).
if (q.get('dbg')) {
  setTimeout(() => {
    document.title = q.get('dbg')!.split(',').map((sel) => { const e = document.querySelector(sel) as HTMLElement | null; const r = e?.getBoundingClientRect(); return sel + ':' + (e?.textContent?.slice(0,12) ?? '') + (r ? [r.x, r.y, r.width, r.height].map((n) => Math.round(n)).join('/') : 'none'); }).join(' | ');
  }, 1200 + 250 * (keys.length + 6));
}

// ?still=1 : coupe les transitions CSS (les captures en temps virtuel les figent à mi-course).
if (q.get('still')) {
  const st = document.createElement('style');
  st.textContent = '*{transition:none!important}';
  document.head.appendChild(st);
}

// Émet un événement Tauri factice : window.__emit('volume-changed', { level: 30, muted: false })
(window as any).__emit = (event: string, payload: unknown) => {
  for (const id of listeners.get(event) ?? []) callbacks.get(id)?.({ event, id, payload });
};

// ?screen=... : ouvre un écran sans touches (après les touches rejouées).
if (q.get('screen')) {
  setTimeout(() => {
    const h = (window as any).__rustvHome;
    const k = q.get('screen')!;
    if (k === 'osd') (window as any).__emit('volume-changed', { level: 42, muted: false });
    else if (k === 'mute') (window as any).__emit('volume-changed', { level: 42, muted: true });
    else h?.show(k);
    const text = q.get('q');
    if (text) {
      setTimeout(() => {
        const input = document.querySelector('input') as HTMLInputElement | null;
        if (input) {
          input.value = text;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, 400);
    }
  }, 1200 + 250 * (keys.length + 1));
}
