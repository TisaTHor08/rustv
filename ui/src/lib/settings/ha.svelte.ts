// Page de réglages « Home Assistant » (MQTT). Montée par Settings.svelte via createPage.
import { invoke } from '@tauri-apps/api/core';
import { t } from '../i18n';
import { askConfirm, askText } from '../overlays.svelte';
import './ha.strings';

type SettingsRow = {
  label: string;
  value?: string;
  swatch?: string;
  left?: () => void | Promise<void>;
  right?: () => void | Promise<void>;
  enter?: () => void | Promise<void>;
  arrows?: boolean;
};

interface HaConfig {
  enabled: boolean;
  host: string;
  port: number;
  username: string;
  hasPassword: boolean;
  tls: boolean;
  clientId: string;
  baseTopic: string;
  discoveryPrefix: string;
  defaultBaseTopic: string;
  defaultClientId: string;
}
interface HaStatus {
  connected: boolean;
  lastError: string | null;
  entities: number;
  lastPublish: number | null;
}

export function createPage(ctx: { note: (m: string) => void }) {
  let cfg = $state<HaConfig | null>(null);
  let status = $state<HaStatus | null>(null);
  let testResult = $state('');
  let busy = $state(false);
  let timer: ReturnType<typeof setInterval> | null = null;

  const errText = (e: unknown) => {
    const k = `ha.err.${String(e)}`;
    const v = t(k);
    return v === k ? (String(e).length < 140 ? String(e) : t('ha.err.failed')) : v;
  };

  async function refresh() {
    try {
      const r = await invoke<{ config: HaConfig; status: HaStatus }>('ha_get');
      cfg = r.config;
      status = r.status;
    } catch {
      /* page sans données : lignes en lecture seule */
    }
  }

  async function save(patch: Partial<HaConfig> & { password?: string }) {
    if (!cfg) return;
    const c = { ...cfg, ...patch };
    try {
      await invoke('ha_set', {
        cfg: {
          enabled: c.enabled,
          host: c.host,
          port: c.port,
          username: c.username,
          tls: c.tls,
          clientId: c.clientId,
          baseTopic: c.baseTopic,
          discoveryPrefix: c.discoveryPrefix,
          ...(patch.password !== undefined ? { password: patch.password } : {}),
        },
      });
      ctx.note(t('ha.saved'));
    } catch (e) {
      ctx.note(errText(e));
    }
    await refresh();
  }

  async function editText(key: 'host' | 'username' | 'baseTopic' | 'discoveryPrefix', q: string, hint = '') {
    if (!cfg) return;
    const v = await askText(t(q), cfg[key], hint);
    if (v !== null) await save({ [key]: v.trim() });
  }

  async function editPort() {
    if (!cfg) return;
    const v = await askText(t('ha.port.q'), String(cfg.port));
    if (v === null) return;
    const n = parseInt(v, 10);
    if (!Number.isFinite(n) || n < 1 || n > 65535) return ctx.note(t('ha.err.bad_port'));
    await save({ port: n });
  }

  async function editPassword() {
    const v = await askText(t('ha.password.q'), '');
    if (v !== null) await save({ password: v });
  }

  async function runTest() {
    if (busy) return;
    busy = true;
    testResult = t('ha.test.run');
    try {
      testResult = await invoke<string>('ha_test');
    } catch (e) {
      testResult = errText(e);
    }
    busy = false;
  }

  async function publish() {
    try {
      const n = await invoke<number>('ha_publish_discovery');
      ctx.note(t('ha.publish.done', { n }));
    } catch (e) {
      ctx.note(errText(e));
    }
    await refresh();
  }

  async function remove() {
    if (!(await askConfirm(t('ha.remove.q'), t('ha.remove.t'), t('ha.remove.ok'), t('common.cancel')))) return;
    try {
      await invoke('ha_remove_discovery');
      ctx.note(t('ha.remove.done'));
    } catch (e) {
      ctx.note(errText(e));
    }
    await refresh();
  }

  const yn = (b: boolean) => t(b ? 'common.on' : 'common.off');

  return {
    titleKey: 'ha.title',
    async load() {
      await refresh();
      if (timer) clearInterval(timer);
      timer = setInterval(refresh, 3000);
    },
    rows(): SettingsRow[] {
      const c = cfg;
      if (!c) return [{ label: t('ha.status'), value: '…' }];
      const toggle = (k: 'enabled' | 'tls') => () => save({ [k]: !c[k] });
      const statusText = !c.enabled
        ? t('ha.status.disabled')
        : status?.connected
          ? `${t('ha.status.on')} · ${t('ha.entities', { n: status.entities })}`
          : `${t('ha.status.off')}${status?.lastError ? ' · ' + status.lastError : ''}`;
      return [
        { label: t('ha.enabled'), value: yn(c.enabled), arrows: true, left: toggle('enabled'), right: toggle('enabled'), enter: toggle('enabled') },
        { label: t('ha.host'), value: c.host || '—', enter: () => editText('host', 'ha.host.q', t('ha.host.hint')) },
        { label: t('ha.port'), value: String(c.port), enter: editPort },
        { label: t('ha.user'), value: c.username || '—', enter: () => editText('username', 'ha.user.q') },
        { label: t('ha.password'), value: c.hasPassword ? t('ha.password.set') : t('ha.password.none'), enter: editPassword },
        { label: t('ha.tls'), value: yn(c.tls), arrows: true, left: toggle('tls'), right: toggle('tls'), enter: toggle('tls') },
        { label: t('ha.base'), value: c.baseTopic || c.defaultBaseTopic, enter: () => editText('baseTopic', 'ha.base.q', c.defaultBaseTopic) },
        { label: t('ha.prefix'), value: c.discoveryPrefix || 'homeassistant', enter: () => editText('discoveryPrefix', 'ha.prefix.q') },
        { label: t('ha.test'), value: testResult || t('ha.test.never'), enter: runTest },
        { label: t('ha.publish'), enter: publish },
        { label: t('ha.remove'), enter: remove },
        { label: t('ha.status'), value: statusText },
      ];
    },
  };
}
