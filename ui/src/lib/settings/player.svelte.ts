// Page de réglages « Lecteur » : langues, reprise, épisode suivant, direct différé, historique.
import { invoke } from '@tauri-apps/api/core';
import { t } from '../i18n';
import { askConfirm } from '../overlays.svelte';
import { continueClear } from '../player.svelte';
import './player.strings';

interface Row {
  label: string;
  value?: string;
  swatch?: string;
  left?: () => void | Promise<void>;
  right?: () => void | Promise<void>;
  enter?: () => void | Promise<void>;
  arrows?: boolean;
}

interface Cfg {
  audio_lang: string;
  sub_lang: string;
  subs_default: boolean;
  auto_next: boolean;
  resume: boolean;
  timeshift_mb: number;
}

const LANGS = ['auto', 'fr', 'en', 'de', 'es', 'it', 'ja'];
const SIZES = [64, 128, 256, 512, 1024];

export function createPage(ctx: { note: (m: string) => void }) {
  let cfg = $state<Cfg>({ audio_lang: 'auto', sub_lang: 'auto', subs_default: false, auto_next: true, resume: true, timeshift_mb: 256 });

  async function save(patch: Partial<Cfg>) {
    const next = { ...cfg, ...patch };
    try {
      cfg = await invoke<Cfg>('player_set_config', { cfg: next });
    } catch (e) {
      ctx.note(String(e));
    }
  }

  const cycle = <T>(arr: T[], cur: T, d: number): T => arr[(Math.max(0, arr.indexOf(cur)) + d + arr.length) % arr.length];
  const yn = (b: boolean) => t(b ? 'player.on' : 'player.off');

  const lang = (label: string, key: 'audio_lang' | 'sub_lang'): Row => ({
    label,
    value: t(`player.lang.${cfg[key]}`),
    arrows: true,
    left: () => save({ [key]: cycle(LANGS, cfg[key], -1) }),
    right: () => save({ [key]: cycle(LANGS, cfg[key], 1) }),
    enter: () => save({ [key]: cycle(LANGS, cfg[key], 1) }),
  });
  const toggle = (label: string, key: 'subs_default' | 'auto_next' | 'resume'): Row => ({
    label,
    value: yn(cfg[key]),
    arrows: true,
    left: () => save({ [key]: !cfg[key] }),
    right: () => save({ [key]: !cfg[key] }),
    enter: () => save({ [key]: !cfg[key] }),
  });

  return {
    titleKey: 'player.title',
    async load() {
      try {
        cfg = await invoke<Cfg>('player_get_config');
      } catch {}
    },
    rows(): Row[] {
      return [
        lang(t('player.audioLang'), 'audio_lang'),
        lang(t('player.subLang'), 'sub_lang'),
        toggle(t('player.subsOn'), 'subs_default'),
        toggle(t('player.autoNext'), 'auto_next'),
        toggle(t('player.resume'), 'resume'),
        {
          label: t('player.timeshift'),
          value: `${cfg.timeshift_mb} Mo`,
          arrows: true,
          left: () => save({ timeshift_mb: cycle(SIZES, cfg.timeshift_mb, -1) }),
          right: () => save({ timeshift_mb: cycle(SIZES, cfg.timeshift_mb, 1) }),
          enter: () => save({ timeshift_mb: cycle(SIZES, cfg.timeshift_mb, 1) }),
        },
        {
          label: t('player.clearHistory'),
          enter: async () => {
            if (await askConfirm(t('player.clearTitle'), t('player.clearText'), 'OK', t('common.cancel'))) {
              await continueClear();
              ctx.note(t('player.cleared'));
            }
          },
        },
      ];
    },
  };
}
