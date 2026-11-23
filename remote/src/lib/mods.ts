// Contributions des mods à la télécommande : libellés, validation des actions, thèmes.
import { lang } from './i18n.ts';
import type { Label, PhoneMsg, RemoteAction, RemoteMod, RemotePanel, RemoteTheme } from './protocol/types.ts';

export function label(l: Label | undefined, language: 'fr' | 'en' = lang): string {
  if (typeof l === 'string') return l;
  if (!l || typeof l !== 'object') return '';
  const other = language === 'fr' ? 'en' : 'fr';
  return l[language] || l[other] || Object.values(l).find((v) => typeof v === 'string' && v) || '';
}

export const modName = (m: RemoteMod, language: 'fr' | 'en' = lang) => (language === 'fr' && m.i18n?.fr?.name) || m.name;

const KEYS = ['up', 'down', 'left', 'right', 'enter', 'back', 'backlong', 'menu', 'search'];
const MEDIA = ['playPause', 'next', 'prev', 'stop', 'volUp', 'volDown', 'mute'];
const ALT = ['open', 'left', 'right', 'confirm'];
const isNum = (n: unknown): n is number => typeof n === 'number' && Number.isFinite(n);

/** Même liste blanche que le PC : tout le reste est ignoré (null). */
export function sanitizeAction(a: any): RemoteAction | null {
  if (!a || typeof a !== 'object') return null;
  switch (a.t) {
    case 'key':
      return KEYS.includes(a.k) ? { t: 'key', k: a.k } : null;
    case 'media':
      return MEDIA.includes(a.a) ? { t: 'media', a: a.a } : null;
    case 'setVolume':
      return isNum(a.level) ? { t: 'setVolume', level: Math.min(100, Math.max(0, Math.round(a.level))) } : null;
    case 'openTile':
      return typeof a.id === 'string' && a.id ? { t: 'openTile', id: a.id } : null;
    case 'home':
      return { t: 'home' };
    case 'tv':
      return typeof a.on === 'boolean' ? { t: 'tv', on: a.on } : null;
    case 'altTab':
      return ALT.includes(a.a) ? { t: 'altTab', a: a.a } : null;
    case 'text':
      return typeof a.text === 'string' ? { t: 'text', backspaces: 0, text: a.text } : null;
    case 'wait':
      return isNum(a.ms) ? { t: 'wait', ms: Math.min(5000, Math.max(0, a.ms)) } : null;
    default:
      return null;
  }
}

export function sanitizeActions(list: unknown): RemoteAction[] {
  return Array.isArray(list) ? list.map(sanitizeAction).filter((a): a is RemoteAction => a !== null) : [];
}

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** Exécute les actions dans l'ordre (120 ms entre deux, `wait` honoré). */
export async function runActions(actions: RemoteAction[], send: (m: PhoneMsg) => void, gap = 120) {
  const list = sanitizeActions(actions);
  for (let i = 0; i < list.length; i++) {
    const a = list[i];
    if (a.t === 'wait') await sleep(a.ms);
    else {
      send(a);
      if (i < list.length - 1) await sleep(gap);
    }
  }
}

// ---- thèmes ----

export const themeKey = (m: RemoteMod, t: RemoteTheme) => `${m.id}/${t.id}`;
const STORE = 'rustv.remoteTheme';

export function loadThemeChoice(): string {
  try {
    return localStorage.getItem(STORE) ?? '';
  } catch {
    return '';
  }
}
export function saveThemeChoice(v: string) {
  try {
    localStorage.setItem(STORE, v);
  } catch {}
}

export function findTheme(mods: RemoteMod[], key: string): RemoteTheme | null {
  if (!key) return null;
  for (const m of mods) for (const t of m.themes ?? []) if (themeKey(m, t) === key) return t;
  return null;
}

let appliedVars: string[] = [];

/** Applique (ou retire, si null) un thème sur le document. */
export function applyTheme(theme: RemoteTheme | null) {
  const root = document.documentElement;
  for (const v of appliedVars) root.style.removeProperty(v);
  appliedVars = [];
  let el = document.getElementById('rustv-mod-theme');
  if (!theme) {
    el?.remove();
    return;
  }
  for (const [k, v] of Object.entries(theme.vars ?? {})) {
    if (typeof v !== 'string' || !k.startsWith('--')) continue;
    root.style.setProperty(k, v);
    appliedVars.push(k);
  }
  if (!el) {
    el = document.createElement('style');
    el.id = 'rustv-mod-theme';
    document.head.appendChild(el);
  }
  el.textContent = typeof theme.css === 'string' ? theme.css : '';
}

export const panelsOf = (mods: RemoteMod[]) =>
  mods.flatMap((m) => (m.panels ?? []).map((p) => ({ mod: m, panel: p as RemotePanel }))).filter((x) => (x.panel.buttons ?? []).length > 0);
