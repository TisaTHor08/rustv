// Page « Cast and DLNA » : activer la réception, nom de l'appareil, état en lecture seule.
import { invoke } from '@tauri-apps/api/core';
import { t } from '../i18n';
import { askConfirm, askText } from '../overlays.svelte';
import './cast.strings';

type SettingsRow = {
  label: string;
  value?: string;
  swatch?: string;
  left?: () => void | Promise<void>;
  right?: () => void | Promise<void>;
  enter?: () => void | Promise<void>;
  arrows?: boolean;
};

interface CastInfo {
  enabled: boolean;
  name: string;
  port: number;
  ip: string;
  status: { running: boolean; lastClient: string; lastEvent: string };
}

export function createPage(ctx: { note: (m: string) => void }) {
  let info = $state<CastInfo | null>(null);

  async function load() {
    try {
      info = await invoke<CastInfo>('cast_get');
    } catch {
      info = null;
    }
  }

  async function apply(cfg: { enabled?: boolean; name?: string }) {
    try {
      info = await invoke<CastInfo>('cast_set', { cfg });
    } catch (e) {
      ctx.note(t('cast.err', { e: String(e) }));
      await load();
    }
  }

  async function toggle() {
    if (!info) return;
    if (!info.enabled) {
      const ok = await askConfirm(t('cast.confirm.t'), t('cast.confirm.x'), t('cast.confirm.ok'), t('common.cancel'));
      if (!ok) return;
    }
    await apply({ enabled: !info.enabled });
  }

  function rows(): SettingsRow[] {
    const i = info;
    const none = t('cast.none');
    return [
      { label: t('cast.enabled'), value: i?.enabled ? t('cast.on') : t('cast.off'), arrows: true, left: toggle, right: toggle, enter: toggle },
      {
        label: t('cast.name'),
        value: i?.name ?? none,
        enter: async () => {
          const n = await askText(t('cast.nameask'), i?.name ?? '');
          if (n !== null) await apply({ name: n });
        },
      },
      { label: t('cast.address'), value: i?.ip ? `${i.ip}:${i.port}` : t('cast.noaddr') },
      { label: t('cast.status'), value: i?.status.running ? t('cast.running') : t('cast.stopped'), enter: load },
      { label: t('cast.last'), value: i?.status.lastClient || none },
      { label: t('cast.event'), value: i?.status.lastEvent || none },
      { label: t('cast.works'), value: t('cast.worksv') },
    ];
  }

  return { titleKey: 'settings.cast', load, rows };
}
