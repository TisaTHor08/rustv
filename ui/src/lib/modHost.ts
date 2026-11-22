// Pont entre les iframes de mods (sandbox, sans réseau) et l'hôte. Chaque appel est contrôlé par les
// permissions du mod, enregistrées côté Rust ; l'iframe ne peut pas appeler directement les commandes de RusTV.
import { api } from './api';
import { mods } from './mods.svelte';
import { prefs } from './prefs.svelte';

export interface HostContext {
  toast: (msg: string) => void;
  close?: () => void;
}

const safe = (s: string) => s.replace(/[^a-zA-Z0-9_-]/g, '-');

async function handle(modId: string, method: string, p: any, ctx: HostContext): Promise<unknown> {
  const m = mods.list.find((x) => x.id === modId);
  const has = (perm: string) => !!m?.enabled && m.permissions.includes(perm);
  switch (method) {
    case 'storage.get':
      return api.modStorageGet(modId, String(p.key));
    case 'storage.set':
      return api.modStorageSet(modId, String(p.key), p.value);
    case 'fetch':
      return api.modFetch(modId, String(p.url));
    case 'launch': {
      if (!has('launch')) throw new Error('permission « launch » manquante');
      const url = String(p.url ?? '');
      if (!/^https?:\/\//.test(url)) throw new Error('seules les adresses http(s) sont acceptées');
      const r = await api.launchCustom({ id: `mod-${modId}-open-${safe(new URL(url).hostname)}`, name: String(p.name ?? new URL(url).hostname), kind: 'web', url, spatnav: true });
      return r;
    }
    case 'toast':
      ctx.toast(String(p.msg ?? '').slice(0, 200));
      return null;
    case 'close':
      ctx.close?.();
      return null;
    case 'info':
      return { id: modId, lang: prefs.lang, theme: document.documentElement.dataset.theme };
    default:
      throw new Error(`méthode inconnue : ${method}`);
  }
}

/** Écoute les messages d'une iframe de mod. Renvoie une fonction de nettoyage. */
export function attachHost(frame: HTMLIFrameElement, modId: string, ctx: HostContext): () => void {
  const onMsg = async (e: MessageEvent) => {
    if (e.source !== frame.contentWindow) return;
    const d = e.data;
    if (!d || typeof d.id !== 'number' || typeof d.method !== 'string') return;
    try {
      const result = await handle(modId, d.method, d.params ?? {}, ctx);
      frame.contentWindow?.postMessage({ id: d.id, result }, '*');
    } catch (err) {
      frame.contentWindow?.postMessage({ id: d.id, error: String(err instanceof Error ? err.message : err) }, '*');
    }
  };
  const sendTheme = () => frame.contentWindow?.postMessage({ t: 'theme', mode: document.documentElement.dataset.theme }, '*');
  frame.addEventListener('load', sendTheme);
  window.addEventListener('message', onMsg);
  const mo = new MutationObserver(sendTheme);
  mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
  return () => {
    window.removeEventListener('message', onMsg);
    frame.removeEventListener('load', sendTheme);
    mo.disconnect();
  };
}

export function forwardKey(frame: HTMLIFrameElement | null, key: string) {
  frame?.contentWindow?.postMessage({ t: 'key', key }, '*');
}
