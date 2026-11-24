// État des mods et de leurs contributions (thèmes, tuiles, widgets, pages, chaînes de langue).
import { api, type ModInfo } from './api';
import { registerStrings } from './i18n';
import { prefs, type Category } from './prefs.svelte';

export interface ModEntry extends ModInfo {
  base: string;
}

export const mods = $state<{ list: ModEntry[]; ready: boolean }>({ list: [], ready: false });

export async function loadMods() {
  try {
    const list = await api.modsList();
    const out: ModEntry[] = [];
    for (const m of list) {
      let base = '';
      if (m.enabled && m.manifest) {
        try {
          base = await api.modBaseUrl(m.id);
        } catch {}
      }
      out.push({ ...m, base });
    }
    mods.list = out;
    for (const m of out) {
      const s = m.enabled ? m.manifest?.contributes?.strings : null;
      if (s?.fr) registerStrings('fr', s.fr);
      if (s?.en) registerStrings('en', s.en);
    }
  } catch {}
  mods.ready = true;
}

const enabled = () => mods.list.filter((m) => m.enabled && m.manifest);

export interface ModTheme {
  key: string;
  modId: string;
  name: string;
  css?: string;
  vars?: Record<string, string>;
}

export function modThemes(): ModTheme[] {
  return enabled().flatMap((m) =>
    ((m.manifest.contributes?.themes as any[]) ?? []).map((t) => ({ key: `${m.id}/${t.id}`, modId: m.id, name: `${t.name} (${m.manifest.name})`, css: t.css, vars: t.vars })),
  );
}

export interface ModTile {
  id: string;
  name: string;
  desc?: string;
  kind: 'web' | 'native' | 'page';
  url?: string;
  target?: string;
  entry?: string;
  category: Category;
  color: string;
  color2: string;
  icon?: string;
  spatnav?: boolean;
  googleSignIn?: boolean;
  modId: string;
  source: 'mod';
}

const safeId = (s: string) => s.replace(/[^a-zA-Z0-9_-]/g, '-');

export function modTiles(): ModTile[] {
  const cats = ['video', 'tv', 'music', 'library', 'games'];
  return enabled().flatMap((m) =>
    ((m.manifest.contributes?.tiles as any[]) ?? [])
      .concat(((m.manifest.contributes?.pages as any[]) ?? []).map((p) => ({ ...p, kind: 'page' })))
      .map((t) => ({
        id: `mod-${m.id}-${safeId(String(t.id))}`,
        name: String(t.name ?? t.id),
        desc: t.desc ? String(t.desc) : m.manifest.name,
        kind: t.kind === 'native' ? 'native' : t.kind === 'page' ? 'page' : 'web',
        url: t.url,
        target: t.target,
        entry: t.entry,
        category: (cats.includes(t.category) ? t.category : 'library') as Category,
        color: t.color ?? '#3b4252',
        color2: t.color2 ?? '#1b1f2a',
        icon: t.icon ? m.base + String(t.icon).replace(/^\/+/, '') : undefined,
        spatnav: t.spatnav ?? true,
        googleSignIn: t.googleSignIn === true,
        modId: m.id,
        source: 'mod' as const,
      })),
  );
}

/** Icônes de tuiles apportées par les mods actifs (id de tuile → URL), pour remplacer la lettre par le vrai logo. */
export function modIcon(tileId: string): string | undefined {
  for (const m of enabled()) {
    const hit = ((m.manifest.contributes?.icons as any[]) ?? []).find((i) => i && i.tile === tileId && typeof i.src === 'string');
    if (hit) return m.base + String(hit.src).replace(/^\/+/, '');
  }
  return undefined;
}

/** Visuels d'écran apportés par les mods actifs (id de tuile → URL), affichés en grand dans le bandeau du haut. */
export function modBanner(tileId: string): string | undefined {
  for (const m of enabled()) {
    const hit = ((m.manifest.contributes?.banners as any[]) ?? []).find((i) => i && i.tile === tileId && typeof i.src === 'string');
    if (hit) return m.base + String(hit.src).replace(/^\/+/, '');
  }
  return undefined;
}

export interface ModWidget {
  key: string;
  modId: string;
  src: string;
  width: number;
  /** Emplacement demandé par le manifeste : topbar (défaut), hero, bar, corner-bl, corner-br. */
  slot: string;
}

export function modWidgets(): ModWidget[] {
  return enabled().flatMap((m) =>
    ((m.manifest.contributes?.widgets as any[]) ?? [])
      .filter((w) => w.entry)
      .map((w) => ({ key: `${m.id}/${w.id}`, modId: m.id, src: m.base + String(w.entry).replace(/^\/+/, ''), width: Number(w.width) || 12, slot: String(w.slot ?? 'topbar') })),
  );
}

export function pageSrc(modId: string, entry: string): string {
  const m = mods.list.find((x) => x.id === modId);
  return m ? m.base + entry.replace(/^\/+/, '') : '';
}

/** Description lisible d'une permission. */
export function permLabel(p: string, fr: boolean): string {
  if (p === 'storage') return fr ? 'Enregistrer ses propres données' : 'Store its own data';
  if (p === 'launch') return fr ? 'Lancer des applications et des sites' : 'Launch apps and websites';
  if (p.startsWith('network:')) return (fr ? 'Accéder à ' : 'Access ') + p.slice(8);
  if (p.startsWith('inject:')) return (fr ? 'Modifier les pages web : ' : 'Modify web pages: ') + p.slice(7);
  return p;
}

// ---------- thème de mod : variables CSS + feuille de style ----------

$effect.root(() => {
  let applied: string[] = [];
  $effect(() => {
    const key = prefs.modTheme;
    const root = document.documentElement;
    for (const k of applied) root.style.removeProperty(k);
    applied = [];
    let style = document.getElementById('rustv-mod-theme') as HTMLStyleElement | null;
    const th = modThemes().find((t) => t.key === key);
    if (!th) {
      style?.remove();
      return;
    }
    for (const [k, v] of Object.entries(th.vars ?? {})) {
      if (/^--[\w-]+$/.test(k)) {
        root.style.setProperty(k, String(v), 'important');
        applied.push(k);
      }
    }
    if (th.css) {
      if (!style) {
        style = document.createElement('style');
        style.id = 'rustv-mod-theme';
        document.head.appendChild(style);
      }
      api.modText(th.modId, th.css).then((css) => style && (style.textContent = css)).catch(() => {});
    } else style?.remove();
  });
});
