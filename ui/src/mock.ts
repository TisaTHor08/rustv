// Harnais de développement : faux Tauri (invoke) avec des données de démonstration, pour voir l'interface dans un
// navigateur ordinaire. Paramètres d'URL : os=<style>, keys=ArrowDown,ArrowRight,Enter (rejoués après le chargement),
// theme=light, scale=110, lang=en, cont=0 (sans « Reprendre »).
import catalog from '../../crates/rustv-shell/src/catalog.json';

const q = new URLSearchParams(location.search);
const prefsOver: Record<string, unknown> = {};
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
  transformCallback: () => cb++,
  unregisterCallback: () => {},
  convertFileSrc: (p: string) => p,
  invoke: async (cmd: string) => {
    switch (cmd) {
      case 'get_state': return state;
      case 'prefs_get': return prefsOver;
      case 'prefs_set': return null;
      case 'mods_list': return [];
      case 'continue_list': return cont;
      case 'update_check': return { available: false, latest: '', current: '1.2.0' };
      case 'plugin:event|listen': return cb++;
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
