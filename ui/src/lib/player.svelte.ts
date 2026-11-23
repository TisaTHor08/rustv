// Lecture via mpv : installation à la demande (avec accord de l'utilisateur) puis lecture.
import { invoke } from '@tauri-apps/api/core';
import { api, type PlayItem } from './api';
import { prefs } from './prefs.svelte';
import { t } from './i18n';
import { askConfirm, overlay } from './overlays.svelte';

export type PlayKind = 'live' | 'video' | 'audio';

export type PlayItemEx = PlayItem & { poster?: string };
export interface ContinueEntry {
  url: string;
  title: string;
  kind: string;
  position: number;
  duration: number;
  updated: number;
  poster: string | null;
  finished: boolean;
}

export const continueList = () => invoke<ContinueEntry[]>('continue_list');
export const continueRemove = (url: string) => invoke<void>('continue_remove', { url });
export const continueClear = () => invoke<void>('continue_clear');

/** `start` : position de reprise en secondes (rangée « Reprendre la lecture »). */
export async function playMedia(kind: PlayKind, items: PlayItemEx[], index = 0, note: (m: string) => void = () => {}, start?: number): Promise<boolean> {
  const go = () => invoke<void>('player_play', { req: { kind, items, index, start, lang: prefs.lang } });
  try {
    await go();
    return true;
  } catch (e) {
    if (!String(e).includes('mpv_missing')) {
      note(String(e));
      return false;
    }
  }
  // mpv n'est pas installé : on demande l'accord avant de télécharger quoi que ce soit.
  const ok = await askConfirm(t('player.missing.title'), t('player.missing.text'), t('player.install'), t('common.cancel'));
  if (!ok) return false;
  overlay.busy = { title: t('player.downloading'), text: 'mpv', pct: 0 };
  const off = await api.onPlayerInstall((p) => {
    if (!overlay.busy) return;
    overlay.busy.pct = p.pct;
    overlay.busy.title = p.stage === 'extract' ? t('player.extracting') : t('player.downloading');
  });
  try {
    await api.playerInstall();
  } catch (err) {
    off();
    overlay.busy = null;
    note(t('player.failed', { e: String(err) }));
    return false;
  }
  off();
  overlay.busy = null;
  try {
    await go();
    return true;
  } catch (e) {
    note(String(e));
    return false;
  }
}
