// Page « Linux » : état des touches du téléphone (uinput) et ouverture des applications de streaming (DRM)
// dans un navigateur Chromium. À n'afficher que si `platform_features().os === 'linux'`.
import { invoke } from '@tauri-apps/api/core';
import { t } from '../i18n';
import { platformFeatures, type PlatformFeatures } from '../platform';
import './linux.strings';

type SettingsRow = {
  label: string;
  value?: string;
  swatch?: string;
  left?: () => void | Promise<void>;
  right?: () => void | Promise<void>;
  enter?: () => void | Promise<void>;
  arrows?: boolean;
};

interface LinuxInfo {
  supported: boolean;
  streamingBrowser: boolean;
  streamingBrowserKind: string;
  browsersFound: string[];
}

const KINDS = ['auto', 'chrome', 'brave', 'vivaldi', 'chromium'];

export function createPage(ctx: { note: (m: string) => void }) {
  let info = $state<LinuxInfo | null>(null);
  let feat = $state<PlatformFeatures | null>(null);

  async function load() {
    feat = await platformFeatures(true);
    try {
      info = await invoke<LinuxInfo>('linux_get');
    } catch {
      info = null;
    }
  }

  async function apply(args: { streamingBrowser?: boolean; streamingBrowserKind?: string }) {
    try {
      await invoke('linux_set', args);
    } catch (e) {
      ctx.note(t('linux.err', { e: String(e) }));
    }
    await load();
  }

  function cycle(dir: number) {
    const cur = Math.max(0, KINDS.indexOf(info?.streamingBrowserKind ?? 'auto'));
    return apply({ streamingBrowserKind: KINDS[(cur + dir + KINDS.length) % KINDS.length] });
  }

  function rows(): SettingsRow[] {
    const i = info;
    const input = feat?.input_error ? t(`linux.input.${feat.input_error}`) : t('linux.input.ok');
    const kind = i?.streamingBrowserKind ?? 'auto';
    const toggle = () => apply({ streamingBrowser: !(i?.streamingBrowser ?? true) });
    return [
      { label: t('linux.input'), value: input, enter: load },
      { label: t('linux.streaming'), value: i?.streamingBrowser === false ? t('linux.streaming.off') : t('linux.streaming.on'), arrows: true, left: toggle, right: toggle, enter: toggle },
      { label: t('linux.kind'), value: kind === 'auto' ? t('linux.kind.auto') : kind, arrows: true, left: () => cycle(-1), right: () => cycle(1) },
      { label: t('linux.found'), value: i?.browsersFound?.length ? i.browsersFound.join(', ') : t('linux.none') },
      { label: t('linux.drmnote') },
    ];
  }

  return { titleKey: 'settings.linux', load, rows };
}
