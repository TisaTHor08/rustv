// Applique le style d'interface choisi : jetons CSS, attribut data-os, police, composant de mise en page.
// Le changement est « à chaud » : le composant est chargé d'abord, puis tout bascule d'un coup (fondu côté Home).
import { invoke } from '@tauri-apps/api/core';
import type { Component } from 'svelte';
import { osAccent, prefs } from '../prefs.svelte';
import type { HomeModel } from './model.svelte';
import { resolveStyle, type OsStyle } from './registry';
import './strings';

export const osState = $state<{ style: OsStyle; Layout: Component<{ model: HomeModel }> | null }>({ style: resolveStyle('rustv'), Layout: null });

// Les jetons de palette suivent le thème sombre uniquement : en thème clair on garde la palette claire de RusTV.
const PALETTE = new Set(['--bg', '--bg-rgb', '--text', '--muted', '--glass', '--glass-strong', '--sheet', '--field', '--ring']);

function writeTokens(s: OsStyle) {
  let el = document.getElementById('rustv-os-tokens') as HTMLStyleElement | null;
  if (!el) {
    el = document.createElement('style');
    el.id = 'rustv-os-tokens';
    document.head.appendChild(el); // avant les variables en ligne (thème de mod, accent), qui gagnent donc toujours
  }
  const sel = `html[data-os="${s.id}"]`;
  const pal: string[] = [];
  const rest: string[] = [];
  for (const [k, v] of Object.entries(s.tokens)) if (/^--[\w-]+$/.test(k)) (PALETTE.has(k) ? pal : rest).push(`${k}:${v};`);
  el.textContent = `${sel}:not([data-theme='light']){${pal.join('')}}${sel}{${rest.join('')}}`;
}

let seq = 0;
async function apply(id: string) {
  const my = ++seq;
  const s = resolveStyle(id);
  let comp: Component<{ model: HomeModel }>;
  try {
    const f = s.font;
    if (f) void document.fonts?.load(`400 1rem "${f}"`).catch(() => {}), void document.fonts?.load(`700 1rem "${f}"`).catch(() => {});
    comp = (await s.load()).default;
  } catch {
    return;
  }
  if (my !== seq) return; // un choix plus récent est passé devant
  writeTokens(s);
  document.documentElement.dataset.os = s.id;
  osAccent.owned = '--accent' in s.tokens;
  osState.style = s;
  osState.Layout = comp;
}

$effect.root(() => {
  $effect(() => {
    void apply(prefs.osStyle);
  });
  // Annonce le style au PC : la télécommande du téléphone prend le modèle de télécommande assorti (aussi au démarrage).
  $effect(() => {
    const id = osState.style?.id ?? prefs.osStyle;
    invoke('set_os_style', { id }).catch(() => {});
  });
});
