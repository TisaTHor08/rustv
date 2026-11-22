// Lecture via mpv : installation à la demande (avec accord de l'utilisateur) puis lecture.
import { api, type PlayItem } from './api';
import { t } from './i18n';
import { askConfirm, overlay } from './overlays.svelte';

export type PlayKind = 'live' | 'video' | 'audio';

export async function playMedia(kind: PlayKind, items: PlayItem[], index = 0, note: (m: string) => void = () => {}): Promise<boolean> {
  try {
    await api.playerPlay({ kind, items, index });
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
    await api.playerPlay({ kind, items, index });
    return true;
  } catch (e) {
    note(String(e));
    return false;
  }
}
