// Page de réglages « Compte Google ». Montée par Settings.svelte via createPage.
import { t } from '../i18n';
import { askConfirm } from '../overlays.svelte';
import { google, type GoogleState } from '../google';
import { modTiles } from '../mods.svelte';
import './google.strings';

type SettingsRow = {
  label: string;
  value?: string;
  left?: () => void | Promise<void>;
  right?: () => void | Promise<void>;
  enter?: () => void | Promise<void>;
  arrows?: boolean;
};

export function createPage(ctx: { note: (m: string) => void }) {
  let st = $state<GoogleState | null>(null);
  let busy = $state(false);
  let listening = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const errText = (e: unknown) => {
    const k = `google.err.${String(e)}`;
    const v = t(k);
    return v === k ? t('google.err.failed') : v;
  };

  async function refresh() {
    try {
      st = await google.get();
    } catch {
      /* page en lecture seule */
    }
    // Tant que la fenêtre de connexion est ouverte, on revérifie (l'événement de fin la rafraîchit aussi).
    clearTimeout(timer);
    if (st?.connecting) timer = setTimeout(() => void refresh(), 2000);
  }

  async function run(fn: () => Promise<unknown>) {
    try {
      await fn();
    } catch (e) {
      ctx.note(errText(e));
    }
    await refresh();
  }

  const yn = (b: boolean) => t(b ? 'common.on' : 'common.off');

  // Apps proposées : catalogue (googleSignIn) + tuiles de mods qui le déclarent. Réglage par app, défaut = activé.
  function appRows(c: GoogleState): SettingsRow[] {
    const items = c.apps.map((a) => ({ id: a.id, name: a.name, on: a.on }));
    for (const m of modTiles()) {
      if (m.kind === 'web' && m.googleSignIn) items.push({ id: m.id, name: m.name, on: c.overrides[m.id] ?? true });
    }
    return items.map((a) => {
      const flip = () => run(() => google.setApp(a.id, !a.on));
      return { label: t('google.app', { name: a.name }), value: yn(a.on), arrows: true, left: flip, right: flip, enter: flip };
    });
  }

  return {
    titleKey: 'google.title',
    async load() {
      if (!listening) {
        listening = true;
        void google.onState(() => {
          busy = false;
          void refresh();
        });
      }
      await refresh();
    },
    rows(): SettingsRow[] {
      const c = st;
      if (!c) return [{ label: t('google.title'), value: '…' }];
      const flip = () => run(() => google.setEnabled(!c.enabled));
      const status = busy ? t('google.disconnecting') : !c.enabled ? t('google.status.off') : c.connecting ? t('google.status.connecting') : c.connected ? t('google.status.ok') : t('google.status.none');
      const rows: SettingsRow[] = [
        { label: t('google.enable'), value: yn(c.enabled), arrows: true, left: flip, right: flip, enter: flip },
        { label: t('google.status'), value: status },
      ];
      if (c.enabled) {
        rows.push({ label: t('google.connect'), enter: () => run(() => google.connect()) });
        if (c.connected) {
          rows.push({
            label: t('google.disconnect'),
            enter: async () => {
              if (!(await askConfirm(t('google.disconnect'), t('google.disconnect.q'), t('google.disconnect.ok'), t('common.cancel')))) return;
              busy = true;
              await run(() => google.disconnect());
            },
          });
        }
        rows.push(...appRows(c));
      }
      rows.push({ label: t('google.about') });
      return rows;
    },
  };
}
