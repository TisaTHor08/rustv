// Page « Télécommande web » : interrupteur, port, adresse (QR), jumelage d'un navigateur, appareils connectés.
import { invoke } from '@tauri-apps/api/core';
import { t } from '../i18n';
import { askConfirm, askText, showQr } from '../overlays.svelte';
import './webremote.strings';

type SettingsRow = {
  label: string;
  value?: string;
  swatch?: string;
  left?: () => void | Promise<void>;
  right?: () => void | Promise<void>;
  enter?: () => void | Promise<void>;
  arrows?: boolean;
};

interface WebClient {
  device: string;
  name: string;
  ip: string;
  secs: number;
}
interface WebInfo {
  enabled: boolean;
  port: number;
  running: boolean;
  bundled: boolean;
  urls: string[];
  qr: string;
  clients: WebClient[];
  error: string;
}

export function createPage(ctx: { note: (m: string) => void }) {
  let info = $state<WebInfo | null>(null);

  async function load() {
    try {
      info = await invoke<WebInfo>('webremote_get');
    } catch {
      info = null;
    }
  }

  function why(e: unknown): string {
    const k = `webremote.err.${String(e)}`;
    return t(k) === k ? String(e) : t(k);
  }

  async function apply(cfg: { enabled?: boolean; port?: number }) {
    try {
      info = await invoke<WebInfo>('webremote_set', { cfg });
    } catch (e) {
      ctx.note(t('webremote.err', { e: why(e) }));
      await load();
    }
  }

  async function toggle() {
    if (!info) return;
    if (!info.enabled) {
      const ok = await askConfirm(t('webremote.confirm.t'), t('webremote.confirm.x'), t('webremote.confirm.ok'), t('common.cancel'));
      if (!ok) return;
    }
    await apply({ enabled: !info.enabled });
  }

  async function pair() {
    if (!info?.running) return ctx.note(t('webremote.pair.off'));
    try {
      const r = await invoke<{ svg: string; device: string; url: string }>('webremote_pair');
      await showQr(t('webremote.pair.t'), r.svg, t('webremote.pair.x'), t('webremote.pair.close'));
      // Un QR non utilisé ne doit pas rester valide.
      await invoke('pairing_cancel', { device: r.device }).catch(() => {});
      await load();
    } catch (e) {
      ctx.note(t('webremote.err', { e: why(e) }));
    }
  }

  function mins(secs: number): string {
    return secs < 60 ? `${secs} s` : `${Math.floor(secs / 60)} min`;
  }

  function rows(): SettingsRow[] {
    const i = info;
    const none = t('webremote.none');
    const out: SettingsRow[] = [
      { label: t('webremote.enabled'), value: i?.enabled ? t('webremote.on') : t('webremote.off'), arrows: true, left: toggle, right: toggle, enter: toggle },
      {
        label: t('webremote.port'),
        value: String(i?.port ?? 47822),
        enter: async () => {
          const v = await askText(t('webremote.portask'), String(i?.port ?? 47822));
          const n = Number(v);
          if (v !== null && Number.isInteger(n)) await apply({ port: n });
        },
      },
      {
        label: t('webremote.address'),
        value: i?.urls[0] ?? t('webremote.noaddr'),
        enter: async () => {
          if (i?.qr) await showQr(t('webremote.address.qr'), i.qr, `${i.urls.join(' · ')}\n${t('webremote.address.x')}`, t('webremote.pair.close'));
        },
      },
      { label: t('webremote.pair'), enter: pair },
      { label: t('webremote.status'), value: i?.running ? t('webremote.running') : t('webremote.stopped'), enter: load },
    ];
    if (i && !i.bundled) out.push({ label: t('webremote.notbundled') });
    out.push({ label: t('webremote.clients'), value: i?.clients.length ? String(i.clients.length) : none });
    for (const c of i?.clients ?? []) out.push({ label: `${c.name || c.device} · ${c.ip}`, value: `${t('webremote.client')} · ${mins(c.secs)}` });
    out.push({
      label: t('webremote.security'),
      value: t('webremote.securityv'),
      enter: () => void askConfirm(t('webremote.security.t'), t('webremote.security.x'), t('webremote.security.ok'), t('common.cancel')),
    });
    return out;
  }

  return { titleKey: 'settings.webremote', load, rows };
}
