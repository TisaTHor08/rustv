<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import { api } from './api';
  import { t } from './i18n';
  import { loadMods, mods, permLabel, type ModEntry } from './mods.svelte';
  import { prefs } from './prefs.svelte';

  let { onclose, note }: { onclose: () => void; note: (m: string) => void } = $props();

  type View = 'list' | 'detail' | 'confirm' | 'busy';
  let tab = $state<'installed' | 'catalog'>('installed');
  let view = $state<View>('list');
  let idx = $state(0);
  let catalog = $state<any[] | null>(null);
  let catError = $state('');
  let busyText = $state('');
  let staged = $state<{ entry: any; manifest: any; permissions: string[] } | null>(null);
  let current = $state<{ kind: 'installed'; mod: ModEntry } | { kind: 'catalog'; entry: any } | null>(null);

  const fr = $derived(prefs.lang === 'fr');
  const installedById = $derived(new Map(mods.list.map((m) => [m.id, m])));

  async function loadCatalog() {
    catError = '';
    catalog = null;
    try {
      const r = await api.registryFetch();
      catalog = Array.isArray(r?.mods) ? r.mods : [];
    } catch (e) {
      catError = String(e);
      catalog = [];
    }
  }

  $effect(() => {
    if (tab === 'catalog' && catalog === null && !catError) loadCatalog();
  });

  interface Row {
    label: string;
    sub?: string;
    badge?: string;
    run: () => void | Promise<void>;
  }

  const listRows = $derived.by<Row[]>(() => {
    if (tab === 'installed') {
      const rows: Row[] = mods.list.map((m) => ({
        label: m.manifest?.name ?? m.id,
        sub: m.error ? m.error : `${m.manifest?.version ?? ''} · ${m.manifest?.description ?? ''}`,
        badge: m.error ? '⚠' : m.enabled ? t('common.on') : t('common.off'),
        run: () => open({ kind: 'installed', mod: m }),
      }));
      rows.push({ label: t('mods.openfolder'), run: () => api.modsOpenFolder() });
      return rows;
    }
    const rows: Row[] = (catalog ?? []).map((e) => {
      const ins = installedById.get(e.id);
      return {
        label: e.name,
        sub: `${e.version} · ${e.description ?? ''}`,
        badge: ins ? (ins.manifest?.version === e.version ? t('mods.installed') : t('mods.update')) : '',
        run: () => open({ kind: 'catalog', entry: e }),
      };
    });
    rows.push({ label: t('mods.reload'), run: loadCatalog });
    return rows;
  });

  function backToList() {
    view = 'list';
    idx = 0;
  }

  function open(c: NonNullable<typeof current>) {
    current = c;
    view = 'detail';
    idx = 0;
  }

  async function refresh() {
    await loadMods();
  }

  const detailActions = $derived.by<Row[]>(() => {
    if (!current) return [];
    if (current.kind === 'installed') {
      const m = current.mod;
      const live = installedById.get(m.id) ?? m;
      const acts: Row[] = [];
      if (!live.error) {
        acts.push({
          label: live.enabled ? t('mods.disable') : t('mods.enable'),
          run: async () => {
            try {
              await api.modSetEnabled(live.id, !live.enabled);
              await refresh();
            } catch (e) {
              note(String(e));
            }
          },
        });
      }
      acts.push({
        label: t('mods.uninstall'),
        run: async () => {
          await api.modUninstall(live.id);
          await refresh();
          view = 'list';
          idx = 0;
        },
      });
      acts.push({ label: t('common.back'), run: backToList });
      return acts;
    }
    const e = current.entry;
    const ins = installedById.get(e.id);
    return [
      { label: ins ? (ins.manifest?.version === e.version ? t('mods.reinstall') : t('mods.update')) : t('mods.install'), run: () => prepare(e) },
      { label: t('common.back'), run: backToList },
    ];
  });

  async function prepare(entry: any) {
    view = 'busy';
    busyText = t('mods.downloading');
    try {
      const r = await api.modInstallPrepare(entry);
      staged = { entry, manifest: r.manifest, permissions: r.permissions };
      view = 'confirm';
      idx = 0;
    } catch (e) {
      note(String(e));
      view = 'detail';
    }
  }

  const confirmActions = $derived.by<Row[]>(() => [
    {
      label: t('mods.installenable'),
      run: async () => {
        if (!staged) return;
        view = 'busy';
        busyText = t('mods.installing');
        try {
          await api.modInstallCommit(staged.manifest.id);
          await refresh();
          note(t('mods.done', { name: staged.manifest.name }));
        } catch (e) {
          note(String(e));
        }
        staged = null;
        tab = 'installed';
        view = 'list';
        idx = 0;
      },
    },
    {
      label: t('common.cancel'),
      run: async () => {
        if (staged) await api.modInstallCancel(staged.manifest.id);
        staged = null;
        view = 'detail';
        idx = 0;
      },
    },
  ]);

  const rows = $derived(view === 'list' ? listRows : view === 'detail' ? detailActions : view === 'confirm' ? confirmActions : []);

  function onKey(e: KeyboardEvent) {
    if (view === 'busy') return void e.preventDefault();
    if (e.key === 'ArrowDown') idx = Math.min(rows.length - 1, idx + 1);
    else if (e.key === 'ArrowUp') idx = Math.max(0, idx - 1);
    else if (e.key === 'ArrowLeft' && view === 'list') ((tab = 'installed'), (idx = 0));
    else if (e.key === 'ArrowRight' && view === 'list') ((tab = 'catalog'), (idx = 0));
    else if (e.key === 'Enter') rows[idx]?.run();
    else if (e.key === 'Escape' || e.key === 'Backspace') {
      if (view === 'confirm') confirmActions[1].run();
      else if (view === 'detail') ((view = 'list'), (idx = 0));
      else onclose();
    } else return;
    e.preventDefault();
  }

  const detail = $derived.by(() => {
    if (!current) return null;
    if (current.kind === 'installed') {
      const m = current.mod.manifest;
      return { name: m?.name ?? current.mod.id, version: m?.version, author: m?.author, desc: m?.description, perms: current.mod.permissions, error: current.mod.error };
    }
    const e = current.entry;
    return { name: e.name, version: e.version, author: e.author, desc: e.description, perms: (e.permissions ?? null) as string[] | null, error: null, source: `${e.source?.repo}@${String(e.source?.commit ?? '').slice(0, 7)}` };
  });
</script>

<svelte:window onkeydown={onKey} />

<div class="sheet" in:fly={{ y: 30, duration: 360, easing: cubicOut }}>
  {#if view === 'list'}
    <div class="tabs">
      <span class:on={tab === 'installed'}>{t('mods.tab.installed')}</span>
      <span class:on={tab === 'catalog'}>{t('mods.tab.catalog')}</span>
    </div>
    {#if tab === 'catalog' && catalog === null}<p class="empty">{t('apps.loading')}</p>{/if}
    {#if catError}<p class="err">{catError}</p>{/if}
    {#if tab === 'installed' && mods.list.length === 0}<p class="empty">{t('mods.none')}</p>{/if}
    <div class="rows">
      {#each rows as r, i}
        <button class="item" class:sel={i === idx} onmouseenter={() => (idx = i)} onclick={() => r.run()}>
          <span class="lb"><b>{r.label}</b>{#if r.sub}<small>{r.sub}</small>{/if}</span>
          {#if r.badge}<span class="badge">{r.badge}</span>{/if}
        </button>
      {/each}
    </div>
    <p class="hint">{t('mods.hint')}</p>
  {:else if view === 'busy'}
    <h3>{busyText}</h3>
    <div class="spin"></div>
  {:else}
    {#if view === 'detail' && detail}
      <h3>{detail.name} <small>{detail.version ?? ''}</small></h3>
      {#if detail.author}<p class="meta">{detail.author}{detail.source ? ' · ' + detail.source : ''}</p>{/if}
      {#if detail.desc}<p class="desc">{detail.desc}</p>{/if}
      {#if detail.error}<p class="err">{detail.error}</p>{/if}
      {#if detail.perms === null}
        <p class="meta">{t('mods.permsbefore')}</p>
      {:else if detail.perms.length}
        <h4>{t('mods.permissions')}</h4>
        <ul>{#each detail.perms as p}<li>{permLabel(p, fr)}</li>{/each}</ul>
      {:else}<p class="meta">{t('mods.noperms')}</p>{/if}
    {:else if view === 'confirm' && staged}
      <h3>{t('mods.confirm', { name: staged.manifest.name })}</h3>
      <p class="meta">{staged.manifest.author ?? ''} · {staged.manifest.version}</p>
      <h4>{t('mods.permissions')}</h4>
      {#if staged.permissions.length}
        <ul>{#each staged.permissions as p}<li>{permLabel(p, fr)}</li>{/each}</ul>
      {:else}<p class="meta">{t('mods.noperms')}</p>{/if}
    {/if}
    <div class="rows">
      {#each rows as r, i}
        <button class="item" class:sel={i === idx} onmouseenter={() => (idx = i)} onclick={() => r.run()}><span class="lb"><b>{r.label}</b></span></button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .sheet { width: 62rem; padding: 2.2rem 2.4rem; border-radius: 1.8rem; background: var(--sheet); box-shadow: 0 3rem 6rem rgba(0, 0, 0, 0.55), inset 0 0 0 0.08rem var(--glass-strong); color: var(--text); }
  h3 { margin: 0 0 0.4rem; font-size: 1.9rem; }
  h3 small { font-size: 1.1rem; color: var(--muted); font-weight: 500; }
  h4 { margin: 1.1rem 0 0.4rem; font-size: 1.05rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.1em; }
  .meta { margin: 0 0 0.6rem; color: var(--muted); font-size: 1.05rem; }
  .desc { margin: 0.6rem 0; font-size: 1.25rem; }
  ul { margin: 0 0 1rem; padding-left: 1.4rem; font-size: 1.15rem; line-height: 1.7; }
  .tabs { display: flex; gap: 0.6rem; margin-bottom: 1rem; }
  .tabs span { padding: 0.5rem 1.4rem; border-radius: 99rem; background: var(--glass); color: var(--muted); font-size: 1.2rem; font-weight: 600; }
  .tabs span.on { background: var(--accent); color: #fff; }
  .rows { max-height: 28rem; overflow: hidden auto; scrollbar-width: none; margin-top: 0.6rem; }
  .item { display: flex; justify-content: space-between; align-items: center; gap: 1rem; width: 100%; text-align: left; margin: 0.2rem 0; padding: 0.8rem 1.1rem; border: 0; border-radius: 0.9rem; background: transparent; color: var(--text); font-family: inherit; font-size: 1.25rem; transition: background 0.2s, transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .item.sel { background: var(--accent); color: #fff; transform: scale(1.015); }
  .lb { display: flex; flex-direction: column; min-width: 0; }
  .lb b { font-weight: 600; }
  .lb small { opacity: 0.7; font-size: 0.95rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 44rem; }
  .badge { flex: none; padding: 0.2rem 0.8rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.18); font-size: 0.95rem; }
  .hint { margin: 1.1rem 0 0; font-size: 0.95rem; color: var(--muted); }
  .empty { color: var(--muted); padding: 0.6rem 0.4rem; }
  .err { color: #ff8b6b; font-size: 1.05rem; }
  .spin { width: 3.4rem; height: 3.4rem; margin: 1.4rem 0; border: 0.35rem solid var(--glass-strong); border-top-color: var(--accent); border-radius: 50%; animation: s 0.8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg); } }
</style>
