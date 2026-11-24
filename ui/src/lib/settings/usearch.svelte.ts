// Page de réglages « Recherche universelle ». Montée par Settings.svelte via createPage.
import { locale, t } from '../i18n';
import { askConfirm, askText, pickFile } from '../overlays.svelte';
import { usearch, type UConfig, type UStatus } from '../usearch';
import './usearch.strings';

type SettingsRow = {
  label: string;
  value?: string;
  swatch?: string;
  left?: () => void | Promise<void>;
  right?: () => void | Promise<void>;
  enter?: () => void | Promise<void>;
  arrows?: boolean;
};

type SourceKey = keyof UConfig['sources'];
const SOURCES: SourceKey[] = ['tmdb', 'library', 'files', 'plex', 'jellyfin', 'ugreen'];

export function createPage(ctx: { note: (m: string) => void }) {
  let cfg = $state<UConfig | null>(null);
  let status = $state<UStatus | null>(null);
  let tests = $state<Record<string, string>>({});
  let listening = false;
  let polling = false;

  const errText = (e: unknown) => {
    const k = `usearch.err.${String(e)}`;
    const v = t(k);
    return v === k ? t('usearch.err.failed') : v;
  };

  async function refresh() {
    try {
      cfg = await usearch.getConfig();
      status = await usearch.status();
    } catch {
      /* page en lecture seule */
    }
    pollWhileRunning();
  }

  // Suit l'indexation tant qu'elle tourne (s'arrête toute seule).
  function pollWhileRunning() {
    if (polling || !status?.running) return;
    polling = true;
    const tick = async () => {
      try {
        status = await usearch.status();
      } catch {
        /* ignoré */
      }
      if (status?.running) setTimeout(tick, 1500);
      else polling = false;
    };
    setTimeout(tick, 1500);
  }

  async function save(patch: Record<string, unknown>) {
    try {
      await usearch.setConfig(patch);
      ctx.note(t('usearch.saved'));
    } catch (e) {
      ctx.note(errText(e));
    }
    await refresh();
  }

  async function ask(q: string, initial: string, hint: string, make: (v: string) => Record<string, unknown>) {
    const v = await askText(t(q), initial, hint);
    if (v !== null) await save(make(v.trim()));
  }

  async function runTest(source: string) {
    tests[source] = t('usearch.test.run');
    try {
      const r = await usearch.test(source);
      tests[source] = r && r !== 'ok' ? `${t('usearch.test.ok')} · ${r}` : t('usearch.test.ok');
    } catch (e) {
      tests[source] = errText(e);
    }
  }

  async function rescan(force: boolean) {
    try {
      const started = await usearch.scan(force);
      if (force) ctx.note(t(started ? 'usearch.rescan.started' : 'usearch.rescan.busy'));
    } catch (e) {
      ctx.note(errText(e));
    }
    await refresh();
  }

  async function addFolder(path: string | null) {
    if (!cfg || !path) return;
    await save({ folders: [...cfg.folders, path.trim()] });
    await rescan(false);
  }

  async function removeFolder(p: string) {
    if (!cfg) return;
    if (!(await askConfirm(p, t('usearch.folders.remove.q'), t('usearch.folders.remove.ok'), t('common.cancel')))) return;
    await save({ folders: cfg.folders.filter((x) => x !== p) });
    await rescan(false);
  }

  async function clearAll() {
    if (!(await askConfirm(t('usearch.clear'), t('usearch.clear.q'), t('usearch.clear.ok'), t('common.cancel')))) return;
    try {
      await usearch.clearCache();
      ctx.note(t('usearch.cleared'));
    } catch (e) {
      ctx.note(errText(e));
    }
    await refresh();
  }

  const yn = (b: boolean) => t(b ? 'common.on' : 'common.off');
  const secretText = (b: boolean, set = 'usearch.secret.set') => t(b ? set : 'usearch.secret.none');

  function indexText(): string {
    const s = status;
    if (!s) return '…';
    if (s.running) return t('usearch.index.running', { n: s.progress });
    const when = s.lastScan ? new Date(s.lastScan * 1000).toLocaleString(locale()) : t('usearch.index.never');
    return t('usearch.index.value', { n: s.total, when });
  }

  const rootText = (p: string) => {
    const r = status?.roots.find((x) => x.path.toLowerCase() === p.toLowerCase());
    return r ? `${r.count} · ${t(`usearch.root.${r.status}`)}` : '';
  };

  return {
    titleKey: 'usearch.title',
    async load() {
      if (!listening) {
        listening = true;
        void usearch.onScan((s) => {
          status = s;
          pollWhileRunning();
        });
      }
      await refresh();
      void rescan(false); // sans effet si les dossiers ont été parcourus il y a moins d'une heure
    },
    rows(): SettingsRow[] {
      const c = cfg;
      if (!c) return [{ label: t('usearch.title'), value: '…' }];
      const rows: SettingsRow[] = [];
      for (const k of SOURCES) {
        const flip = () => save({ sources: { [k]: !c.sources[k] } });
        rows.push({ label: t(`usearch.src.${k}`), value: yn(c.sources[k]), arrows: true, left: flip, right: flip, enter: flip });
      }
      rows.push(
        { label: t('usearch.tmdb'), value: secretText(c.hasTmdb, 'usearch.tmdb.set'), enter: () => ask('usearch.tmdb.q', '', t('usearch.tmdb.hint'), (v) => ({ tmdbToken: v })) },
        { label: t('usearch.country'), value: c.country || t('usearch.country.auto'), enter: () => ask('usearch.country.q', c.country, '', (v) => ({ country: v })) },
        { label: t('usearch.test', { name: 'TMDB' }), value: tests.tmdb ?? t('usearch.test.never'), enter: () => runTest('tmdb') },
        { label: t('usearch.plex.url'), value: c.plex.url || t('usearch.none'), enter: () => ask('usearch.plex.url.q', c.plex.url, t('usearch.plex.url.hint'), (v) => ({ plex: { url: v } })) },
        { label: t('usearch.plex.token'), value: secretText(c.plex.hasToken), enter: () => ask('usearch.plex.token.q', '', '', (v) => ({ plex: { token: v } })) },
        { label: t('usearch.test', { name: 'Plex' }), value: tests.plex ?? t('usearch.test.never'), enter: () => runTest('plex') },
        { label: t('usearch.jf.url'), value: c.jellyfin.url || c.jellyfin.tileUrl || t('usearch.none'), enter: () => ask('usearch.jf.url.q', c.jellyfin.url, t('usearch.jf.url.hint'), (v) => ({ jellyfin: { url: v } })) },
        { label: t('usearch.jf.key'), value: secretText(c.jellyfin.hasKey), enter: () => ask('usearch.jf.key.q', '', '', (v) => ({ jellyfin: { key: v } })) },
        { label: t('usearch.test', { name: 'Jellyfin' }), value: tests.jellyfin ?? t('usearch.test.never'), enter: () => runTest('jellyfin') },
        { label: t('usearch.ugreen.label'), value: c.ugreen.url || t('usearch.ugreen.none') },
        { label: t('usearch.test', { name: 'UGREEN' }), value: tests.ugreen ?? t('usearch.ugreen.note'), enter: () => runTest('ugreen') },
        { label: t('usearch.folders.add'), enter: async () => addFolder(await pickFile(t('picker.title.folder'), 'folder')) },
        { label: t('usearch.folders.type'), enter: async () => addFolder(await askText(t('usearch.folders.type.q'), '', '')) },
      );
      for (const p of c.folders) rows.push({ label: t('usearch.folders.remove', { path: p }), value: rootText(p), enter: () => removeFolder(p) });
      for (const p of c.libraryFolders) rows.push({ label: t('usearch.folders.lib', { path: p }), value: rootText(p) });
      rows.push(
        { label: t('usearch.index'), value: indexText() },
        status?.running ? { label: t('usearch.rescan.stop'), enter: () => usearch.scanCancel() } : { label: t('usearch.rescan'), enter: () => rescan(true) },
        { label: t('usearch.clear'), enter: clearAll },
        { label: t('usearch.about') },
      );
      return rows;
    },
  };
}
