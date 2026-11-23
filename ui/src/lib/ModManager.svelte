<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import { tick } from 'svelte';
  import { api } from './api';
  import { t } from './i18n';
  import { loadMods, mods, permLabel, type ModEntry } from './mods.svelte';
  import { play } from './sounds';
  import { prefs, savePrefs } from './prefs.svelte';

  let { onclose, note }: { onclose: () => void; note: (m: string) => void } = $props();

  // ---------- données ----------

  const CATS = ['themes', 'widgets', 'pages', 'games', 'apps', 'enhancements', 'icons'] as const;
  type Filter = 'all' | 'installed' | (typeof CATS)[number];

  interface Card {
    id: string;
    name: string;
    publisher: string;
    category: string;
    desc: string;
    details: string;
    tags: string[];
    version: string;
    icon: string;
    banner: string;
    art: { from: string; to: string; glyph: string } | null;
    deps: { type: 'mod' | 'external'; id?: string; name?: string; url?: string; note?: string }[];
    perms: string[] | null;
    license: string;
    entry: any | null; // entrée du catalogue
    mod: ModEntry | null; // mod installé
    update: boolean;
    error: string | null;
  }

  type View = 'browse' | 'detail' | 'confirm' | 'busy';
  let view = $state<View>('browse');
  let area = $state<'search' | 'chips' | 'grid'>('grid');
  let filter = $state<Filter>('all');
  let q = $state('');
  let gi = $state(0);
  let ai = $state(0); // ligne d'action dans la fiche
  let catalog = $state<any[] | null>(null);
  let catError = $state('');
  let busyText = $state('');
  let staged = $state<{ entry: any; manifest: any; permissions: string[] } | null>(null);
  let currentId = $state<string | null>(null);
  let searchEl: HTMLInputElement | undefined = $state();
  let gridEl: HTMLDivElement | undefined = $state();

  const fr = $derived(prefs.lang === 'fr');

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
    if (catalog === null && !catError) loadCatalog();
  });

  const loc = (src: any, k: 'name' | 'description' | 'details', base: string) => {
    const tr = src?.i18n?.[prefs.lang];
    return (tr && tr[k]) || base || '';
  };
  const rawBase = (e: any) => `https://raw.githubusercontent.com/${e.source?.repo}/${e.source?.commit}/${e.source?.path ? e.source.path + '/' : ''}`;
  const rel = (p: string) => String(p).replace(/^\/+/, '');

  function fromEntry(e: any): Card {
    const ins = mods.list.find((m) => m.id === e.id) ?? null;
    return {
      id: e.id,
      name: loc(e, 'name', e.name),
      publisher: e.publisher ?? e.author ?? '',
      category: e.category ?? 'apps',
      desc: loc(e, 'description', e.description),
      details: loc(e, 'details', e.details ?? ''),
      tags: e.tags ?? [],
      version: e.version ?? '',
      icon: e.icon ? rawBase(e) + rel(e.icon) : '',
      banner: e.banner ? rawBase(e) + rel(e.banner) : '',
      art: e.art ?? null,
      deps: e.dependencies ?? [],
      perms: e.permissions ?? null,
      license: e.license ?? '',
      entry: e,
      mod: ins,
      update: !!ins && ins.manifest?.version !== e.version,
      error: ins?.error ?? null,
    };
  }
  function fromMod(m: ModEntry): Card {
    const mf = m.manifest ?? {};
    return {
      id: m.id,
      name: loc(mf, 'name', mf.name ?? m.id),
      publisher: mf.publisher ?? mf.author ?? '',
      category: mf.category ?? 'apps',
      desc: loc(mf, 'description', mf.description ?? ''),
      details: loc(mf, 'details', mf.details ?? ''),
      tags: mf.tags ?? [],
      version: mf.version ?? '',
      icon: m.base && mf.icon ? m.base + rel(mf.icon) : '',
      banner: m.base && mf.banner ? m.base + rel(mf.banner) : '',
      art: mf.art ?? null,
      deps: mf.dependencies ?? [],
      perms: m.permissions,
      license: mf.license ?? '',
      entry: null,
      mod: m,
      update: false,
      error: m.error,
    };
  }

  const all = $derived.by<Card[]>(() => {
    const out = (catalog ?? []).map(fromEntry);
    const inCat = new Set(out.map((c) => c.id));
    for (const m of mods.list) if (!inCat.has(m.id)) out.push(fromMod(m));
    return out.sort((a, b) => a.name.localeCompare(b.name));
  });

  const norm = (s: string) => s.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '');
  const catLabel = (c: string) => t('mods.cat.' + c);

  const shown = $derived.by<Card[]>(() => {
    const words = norm(q).split(/\s+/).filter(Boolean);
    return all.filter((c) => {
      if (filter === 'installed' && !c.mod) return false;
      if (filter !== 'all' && filter !== 'installed' && c.category !== filter) return false;
      if (!words.length) return true;
      const hay = norm([c.name, c.publisher, c.desc, c.tags.join(' '), catLabel(c.category), c.id].join(' '));
      return words.every((w) => hay.includes(w));
    });
  });

  const counts = $derived.by(() => {
    const m: Record<string, number> = { all: all.length, installed: all.filter((c) => c.mod).length };
    for (const c of CATS) m[c] = all.filter((x) => x.category === c).length;
    return m;
  });
  const chips = $derived<Filter[]>(['all', 'installed', ...CATS.filter((c) => counts[c] > 0)]);

  const current = $derived(currentId ? (all.find((c) => c.id === currentId) ?? null) : null);
  const stars = $derived(current ? (prefs.modRatings[current.id] ?? 0) : 0);

  $effect(() => {
    // La sélection reste dans la liste quand le filtre change.
    if (gi >= shown.length) gi = Math.max(0, shown.length - 1);
  });

  const depState = (d: Card['deps'][number]) => {
    if (d.type === 'mod') {
      const c = all.find((x) => x.id === d.id);
      return { label: c?.name ?? d.id ?? '', ok: !!c?.mod, card: c ?? null };
    }
    return { label: d.name ?? '', ok: null as boolean | null, card: null };
  };

  // ---------- actions ----------

  interface Row {
    label: string;
    kind?: 'primary' | 'danger' | 'rating' | 'link';
    run: () => void | Promise<void>;
  }

  async function refresh() {
    await loadMods();
  }

  function setRating(n: number) {
    if (!current) return;
    n = Math.max(0, Math.min(5, n));
    if (n === 0) delete prefs.modRatings[current.id];
    else prefs.modRatings[current.id] = n;
    savePrefs();
  }

  const actions = $derived.by<Row[]>(() => {
    const c = current;
    if (!c) return [];
    const rows: Row[] = [];
    if (c.entry) {
      rows.push({
        label: c.mod ? (c.update ? t('mods.update') : t('mods.reinstall')) : t('mods.install'),
        kind: 'primary',
        run: () => prepare(c.entry),
      });
    }
    if (c.mod && !c.error) {
      const m = c.mod;
      rows.push({
        label: m.enabled ? t('mods.disable') : t('mods.enable'),
        run: async () => {
          try {
            await api.modSetEnabled(m.id, !m.enabled);
            await refresh();
          } catch (e) {
            note(String(e));
          }
        },
      });
    }
    if (c.mod) {
      rows.push({
        label: t('mods.uninstall'),
        kind: 'danger',
        run: async () => {
          const id = c.id;
          await api.modUninstall(id);
          await refresh();
          note(t('mods.removed', { name: c.name }));
          if (!all.some((x) => x.id === id)) back();
        },
      });
    }
    rows.push({ label: t('mods.rate'), kind: 'rating', run: () => {} });
    for (const d of c.deps) {
      const s = depState(d);
      if (d.type === 'mod' && s.card) {
        rows.push({ label: t('mods.opendep', { name: s.label }), kind: 'link', run: () => openCard(s.card!.id) });
      }
    }
    rows.push({ label: t('common.back'), run: back });
    return rows;
  });

  function openCard(id: string) {
    currentId = id;
    view = 'detail';
    ai = 0;
    play('select');
  }
  function back() {
    view = 'browse';
    area = 'grid';
    play('back');
  }

  async function prepare(entry: any) {
    view = 'busy';
    busyText = t('mods.downloading');
    try {
      const r = await api.modInstallPrepare(entry);
      staged = { entry, manifest: r.manifest, permissions: r.permissions };
      view = 'confirm';
      ai = 0;
    } catch (e) {
      note(String(e));
      view = 'detail';
    }
  }

  const confirmRows = $derived<Row[]>([
    {
      label: t('mods.installenable'),
      kind: 'primary',
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
        view = 'detail';
        ai = 0;
      },
    },
    {
      label: t('common.cancel'),
      run: async () => {
        if (staged) await api.modInstallCancel(staged.manifest.id);
        staged = null;
        view = 'detail';
        ai = 0;
      },
    },
  ]);

  // ---------- clavier ----------

  const COLS = 3;

  function focusSearch() {
    area = 'search';
    void tick().then(() => searchEl?.focus());
  }
  function leaveSearch(to: 'chips' | 'grid') {
    searchEl?.blur();
    area = to;
  }

  async function scrollToFocus() {
    await tick();
    gridEl?.querySelector('.card.sel')?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }

  function onKey(e: KeyboardEvent) {
    if (view === 'busy') return void e.preventDefault();
    const k = e.key;

    if (view === 'confirm') {
      if (k === 'ArrowDown') ai = Math.min(confirmRows.length - 1, ai + 1);
      else if (k === 'ArrowUp') ai = Math.max(0, ai - 1);
      else if (k === 'Enter') confirmRows[ai].run();
      else if (k === 'Escape' || k === 'Backspace') confirmRows[1].run();
      else return;
      play(k === 'Enter' ? 'select' : 'move');
      return void e.preventDefault();
    }

    if (view === 'detail') {
      const row = actions[ai];
      if (k === 'ArrowDown') ai = Math.min(actions.length - 1, ai + 1);
      else if (k === 'ArrowUp') ai = Math.max(0, ai - 1);
      else if (k === 'ArrowLeft' && row?.kind === 'rating') setRating(stars - 1);
      else if (k === 'ArrowRight' && row?.kind === 'rating') setRating(stars + 1);
      else if (/^[1-5]$/.test(k) && row?.kind === 'rating') setRating(Number(k));
      else if (k === 'Enter') {
        if (row?.kind === 'rating') setRating(stars === 5 ? 0 : stars + 1);
        else void row?.run();
      } else if (k === 'Escape' || k === 'Backspace') back();
      else return;
      play(k === 'Enter' ? 'select' : 'move');
      return void e.preventDefault();
    }

    // navigation dans le catalogue
    if (area === 'search') {
      if (k === 'ArrowDown') leaveSearch('chips');
      else if (k === 'Enter') leaveSearch('grid');
      else if (k === 'Escape') (q ? (q = '') : leaveSearch('chips'));
      else if (k === 'Backspace' && q === '') leaveSearch('chips');
      else return; // le reste se tape dans le champ
      return void e.preventDefault();
    }
    if (area === 'chips') {
      const i = chips.indexOf(filter);
      if (k === 'ArrowRight') filter = chips[Math.min(chips.length - 1, i + 1)];
      else if (k === 'ArrowLeft') filter = chips[Math.max(0, i - 1)];
      else if (k === 'ArrowUp') focusSearch();
      else if (k === 'ArrowDown' || k === 'Enter') area = 'grid';
      else if (k === 'Escape' || k === 'Backspace') return (onclose(), e.preventDefault());
      else return;
      if (k.startsWith('Arrow')) gi = 0;
      play('move');
      return void e.preventDefault();
    }
    // grille
    if (k === 'ArrowRight') gi = Math.min(shown.length - 1, gi + 1);
    else if (k === 'ArrowLeft') gi = Math.max(0, gi - 1);
    else if (k === 'ArrowDown') gi = Math.min(shown.length - 1, gi + COLS);
    else if (k === 'ArrowUp') {
      if (gi < COLS) area = 'chips';
      else gi -= COLS;
    } else if (k === 'Enter') {
      if (shown[gi]) openCard(shown[gi].id);
      return void e.preventDefault();
    } else if (k === 'Escape' || k === 'Backspace') {
      onclose();
      return void e.preventDefault();
    } else if (k.length === 1 && !e.ctrlKey && !e.altKey && !e.metaKey) {
      // Taper une lettre ouvre la recherche (clavier du PC ou du téléphone).
      focusSearch();
      return;
    } else return;
    play('move');
    void scrollToFocus();
    e.preventDefault();
  }

  const starRow = [1, 2, 3, 4, 5];
  const grad = (c: Card) => `linear-gradient(135deg, ${c.art?.from ?? '#3b4252'}, ${c.art?.to ?? '#1b1f2a'})`;
  const initial = (c: Card) => (c.name || '?').slice(0, 1).toUpperCase();
  let bad = $state<Record<string, boolean>>({});
</script>

<svelte:window onkeydown={onKey} />

<div class="sheet" in:fly={{ y: 30, duration: 360, easing: cubicOut }}>
  {#if view === 'browse'}
    <header>
      <h2>{t('mods.title')}</h2>
      <label class="search" class:on={area === 'search'}>
        <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="6.5" /><path d="M16 16l4.5 4.5" /></svg>
        <input
          bind:this={searchEl}
          bind:value={q}
          placeholder={t('mods.search')}
          onfocus={() => (area = 'search')}
          spellcheck="false"
          autocomplete="off"
        />
        {#if q}<button class="x" onclick={() => (q = '')} aria-label={t('mods.clear')}>×</button>{/if}
      </label>
    </header>

    <nav class="chips">
      {#each chips as c}
        <button class="chip" class:on={filter === c} class:focus={area === 'chips' && filter === c} onclick={() => ((filter = c), (gi = 0), (area = 'grid'))}>
          {c === 'all' ? t('mods.cat.all') : c === 'installed' ? t('mods.tab.installed') : catLabel(c)}<i>{counts[c]}</i>
        </button>
      {/each}
    </nav>

    {#if catalog === null}<p class="empty">{t('apps.loading')}</p>{/if}
    {#if catError}<p class="err">{catError} <button class="link" onclick={loadCatalog}>{t('mods.reload')}</button></p>{/if}
    {#if catalog !== null && shown.length === 0}<p class="empty">{q ? t('mods.noresult') : filter === 'installed' ? t('mods.none') : t('mods.empty')}</p>{/if}

    <div class="grid" bind:this={gridEl}>
      {#each shown as c, i (c.id)}
        <button class="card" class:sel={area === 'grid' && i === gi} onmouseenter={() => ((gi = i), (area = 'grid'))} onclick={() => openCard(c.id)}>
          <div class="banner" style:background={grad(c)}>
            {#if c.banner && !bad[c.id + 'b']}
              <img src={c.banner} alt="" loading="lazy" onerror={() => (bad[c.id + 'b'] = true)} />
            {:else}<span class="mono">{initial(c)}</span>{/if}
            {#if c.mod}<span class="pill">{c.update ? t('mods.update') : c.mod.enabled ? t('mods.installed') : t('mods.disabled')}</span>{/if}
          </div>
          <div class="body">
            <div class="ico" style:background={grad(c)}>
              {#if c.icon && !bad[c.id + 'i']}<img src={c.icon} alt="" loading="lazy" onerror={() => (bad[c.id + 'i'] = true)} />{:else}{initial(c)}{/if}
            </div>
            <div class="tx">
              <b>{c.name}</b>
              <small>{c.publisher} · {catLabel(c.category)}</small>
            </div>
            {#if prefs.modRatings[c.id]}<span class="mini">★ {prefs.modRatings[c.id]}</span>{/if}
          </div>
          <p class="d">{c.desc}</p>
        </button>
      {/each}
    </div>
    <p class="hint">{t('mods.hint2')}</p>
  {:else if view === 'busy'}
    <h3>{busyText}</h3>
    <div class="spin"></div>
  {:else if view === 'confirm' && staged}
    <h3>{t('mods.confirm', { name: staged.manifest.name })}</h3>
    <p class="meta">{staged.manifest.publisher ?? staged.manifest.author ?? ''} · {staged.manifest.version}</p>
    <h4>{t('mods.permissions')}</h4>
    {#if staged.permissions.length}
      <ul>{#each staged.permissions as p}<li>{permLabel(p, fr)}</li>{/each}</ul>
    {:else}<p class="meta">{t('mods.noperms')}</p>{/if}
    <div class="acts">
      {#each confirmRows as r, i}
        <button class="act" class:primary={r.kind === 'primary'} class:sel={i === ai} onmouseenter={() => (ai = i)} onclick={() => r.run()}>{r.label}</button>
      {/each}
    </div>
  {:else if view === 'detail' && current}
    <div class="detail">
      <aside>
        <div class="banner big" style:background={grad(current)}>
          {#if current.banner && !bad[current.id + 'b']}
            <img src={current.banner} alt="" onerror={() => (bad[current.id + 'b'] = true)} />
          {:else}<span class="mono">{initial(current)}</span>{/if}
        </div>
        <div class="acts">
          {#each actions as r, i}
            {#if r.kind === 'rating'}
              <div class="act rate" class:sel={i === ai} onmouseenter={() => (ai = i)} role="group" aria-label={t('mods.rate')}>
                <span>{t('mods.rate')}</span>
                <span class="stars">
                  {#each starRow as n}
                    <button class="star" class:on={n <= stars} onclick={() => setRating(stars === n ? 0 : n)} aria-label="{n}">★</button>
                  {/each}
                </span>
              </div>
            {:else}
              <button class="act" class:primary={r.kind === 'primary'} class:danger={r.kind === 'danger'} class:sel={i === ai} onmouseenter={() => (ai = i)} onclick={() => r.run()}>{r.label}</button>
            {/if}
          {/each}
        </div>
      </aside>
      <section>
        <div class="head">
          <div class="ico lg" style:background={grad(current)}>
            {#if current.icon && !bad[current.id + 'i']}<img src={current.icon} alt="" onerror={() => (bad[current.id + 'i'] = true)} />{:else}{initial(current)}{/if}
          </div>
          <div>
            <h3>{current.name} <small>{current.version}</small></h3>
            <p class="meta">
              {t('mods.publisher')} : <b>{current.publisher || '—'}</b> · {catLabel(current.category)}{current.license ? ' · ' + current.license : ''}
            </p>
            {#if current.mod}<p class="state">{current.update ? t('mods.update') : current.mod.enabled ? t('mods.installed') : t('mods.disabled')}</p>{/if}
          </div>
        </div>
        <p class="desc">{current.desc}</p>
        {#if current.details}<p class="det">{current.details}</p>{/if}
        {#if current.error}<p class="err">{current.error}</p>{/if}

        <h4>{t('mods.deps')}</h4>
        {#if current.deps.length}
          <ul class="deps">
            {#each current.deps as d}
              {@const s = depState(d)}
              <li>
                <span class="dot" class:ok={s.ok === true} class:no={s.ok === false}></span>
                <b>{s.label}</b>
                <small>{d.type === 'mod' ? (s.ok ? t('mods.dep.installed') : t('mods.dep.missing')) : t('mods.dep.external')}{d.note ? ' · ' + d.note : ''}{d.url ? ' · ' + d.url : ''}</small>
              </li>
            {/each}
          </ul>
        {:else}<p class="meta">{t('mods.nodeps')}</p>{/if}

        <h4>{t('mods.permissions')}</h4>
        {#if current.perms === null}
          <p class="meta">{t('mods.permsbefore')}</p>
        {:else if current.perms.length}
          <ul>{#each current.perms as p}<li>{permLabel(p, fr)}</li>{/each}</ul>
        {:else}<p class="meta">{t('mods.noperms')}</p>{/if}
        {#if current.tags.length}<p class="tags">{#each current.tags as g}<span>#{g}</span>{/each}</p>{/if}
      </section>
    </div>
  {/if}
</div>

<style>
  .sheet { width: 90rem; max-height: 50rem; display: flex; flex-direction: column; padding: 2rem 2.4rem 1.6rem; border-radius: 1.8rem; background: var(--sheet); box-shadow: 0 3rem 6rem rgba(0, 0, 0, 0.55), inset 0 0 0 0.08rem var(--glass-strong); color: var(--text); overflow: hidden; }
  header { display: flex; align-items: center; justify-content: space-between; gap: 2rem; margin-bottom: 1.1rem; }
  h2 { margin: 0; font-size: 2.1rem; }
  h3 { margin: 0 0 0.3rem; font-size: 2rem; }
  h3 small { font-size: 1.1rem; color: var(--muted); font-weight: 500; }
  h4 { margin: 1.1rem 0 0.4rem; font-size: 0.95rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.1em; }
  .search { display: flex; align-items: center; gap: 0.7rem; width: 34rem; padding: 0.7rem 1.1rem; border-radius: 99rem; background: var(--glass); box-shadow: inset 0 0 0 0.12rem transparent; transition: box-shadow 0.2s, background 0.2s; }
  .search.on { background: var(--glass-strong); box-shadow: inset 0 0 0 0.16rem var(--accent); }
  .search svg { width: 1.5rem; height: 1.5rem; fill: none; stroke: var(--muted); stroke-width: 2; stroke-linecap: round; flex: none; }
  .search input { flex: 1; min-width: 0; border: 0; outline: 0; background: transparent; color: var(--text); font: inherit; font-size: 1.3rem; }
  .search .x { border: 0; background: transparent; color: var(--muted); font-size: 1.6rem; line-height: 1; padding: 0 0.3rem; }
  .chips { display: flex; flex-wrap: wrap; gap: 0.6rem; margin-bottom: 1rem; }
  .chip { display: inline-flex; align-items: center; gap: 0.5rem; padding: 0.45rem 1.2rem; border: 0; border-radius: 99rem; background: var(--glass); color: var(--muted); font: inherit; font-size: 1.1rem; font-weight: 600; transition: background 0.2s, transform 0.2s; }
  .chip i { font-style: normal; font-size: 0.85rem; opacity: 0.7; }
  .chip.on { background: var(--accent); color: #fff; }
  .chip.focus { transform: scale(1.08); box-shadow: 0 0 0 0.2rem var(--ring); }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1.2rem; overflow: hidden auto; scrollbar-width: none; padding: 0.4rem 0.3rem 0.8rem; min-height: 0; max-height: 33rem; }
  .card { display: flex; flex-direction: column; text-align: left; padding: 0; border: 0; border-radius: 1.2rem; overflow: hidden; background: var(--glass); color: var(--text); font: inherit; transition: transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1), box-shadow 0.25s; }
  .card.sel { transform: scale(1.03); box-shadow: 0 0 0 0.22rem var(--ring), 0 1.4rem 2.6rem rgba(0, 0, 0, 0.45); }
  .banner { position: relative; aspect-ratio: 2 / 1; display: grid; place-items: center; overflow: hidden; }
  .banner img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .banner .mono { font-size: 5rem; font-weight: 800; opacity: 0.35; }
  .pill { position: absolute; top: 0.6rem; right: 0.6rem; padding: 0.15rem 0.7rem; border-radius: 99rem; background: rgba(0, 0, 0, 0.55); font-size: 0.85rem; font-weight: 600; }
  .body { display: flex; align-items: center; gap: 0.8rem; padding: 0.7rem 0.9rem 0; }
  .ico { width: 3.2rem; height: 3.2rem; flex: none; border-radius: 0.8rem; display: grid; place-items: center; overflow: hidden; font-weight: 800; font-size: 1.4rem; color: #fff; }
  .ico img { width: 100%; height: 100%; object-fit: cover; }
  .ico.lg { width: 6.4rem; height: 6.4rem; border-radius: 1.4rem; font-size: 2.6rem; }
  .tx { display: flex; flex-direction: column; min-width: 0; flex: 1; }
  .tx b { font-size: 1.25rem; font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tx small { color: var(--muted); font-size: 0.9rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .mini { flex: none; font-size: 0.95rem; color: #f5c451; font-weight: 700; }
  .d { margin: 0.4rem 0.9rem 0.9rem; font-size: 0.98rem; line-height: 1.35; color: var(--muted); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; min-height: 2.7em; }
  .detail { display: grid; grid-template-columns: 34rem 1fr; gap: 2.2rem; min-height: 0; max-height: 44rem; overflow: hidden; }
  .banner.big { border-radius: 1.2rem; }
  aside { display: flex; flex-direction: column; gap: 1rem; }
  section { overflow: hidden auto; scrollbar-width: none; padding-right: 0.4rem; }
  .head { display: flex; gap: 1.2rem; align-items: center; margin-bottom: 0.8rem; }
  .meta { margin: 0 0 0.4rem; color: var(--muted); font-size: 1.05rem; }
  .meta b { color: var(--text); }
  .state { margin: 0; display: inline-block; padding: 0.1rem 0.8rem; border-radius: 99rem; background: var(--accent); color: #fff; font-size: 0.9rem; font-weight: 600; }
  .desc { margin: 0.4rem 0; font-size: 1.35rem; font-weight: 500; }
  .det { margin: 0.4rem 0; font-size: 1.1rem; line-height: 1.5; color: var(--muted); }
  ul { margin: 0 0 0.6rem; padding-left: 1.3rem; font-size: 1.1rem; line-height: 1.6; }
  ul.deps { list-style: none; padding: 0; }
  ul.deps li { display: flex; align-items: baseline; gap: 0.6rem; flex-wrap: wrap; }
  ul.deps small { color: var(--muted); font-size: 0.95rem; }
  .dot { width: 0.8rem; height: 0.8rem; border-radius: 50%; background: var(--muted); display: inline-block; }
  .dot.ok { background: #3ecf8e; }
  .dot.no { background: #f5a524; }
  .tags { display: flex; flex-wrap: wrap; gap: 0.5rem; margin: 0.8rem 0 0; }
  .tags span { padding: 0.15rem 0.7rem; border-radius: 99rem; background: var(--glass); color: var(--muted); font-size: 0.9rem; }
  .acts { display: flex; flex-direction: column; gap: 0.5rem; }
  .act { display: flex; align-items: center; justify-content: space-between; width: 100%; padding: 0.8rem 1.2rem; border: 0; border-radius: 0.9rem; background: var(--glass); color: var(--text); font: inherit; font-size: 1.2rem; font-weight: 600; text-align: left; transition: background 0.2s, transform 0.2s; }
  .act.primary { background: var(--accent); color: #fff; }
  .act.danger { color: #ff8b6b; }
  .act.sel { transform: scale(1.025); box-shadow: 0 0 0 0.2rem var(--ring); }
  .act.rate .stars { display: inline-flex; gap: 0.15rem; }
  .star { border: 0; padding: 0 0.1rem; background: transparent; font-size: 1.7rem; line-height: 1; color: var(--muted); opacity: 0.5; }
  .star.on { color: #f5c451; opacity: 1; }
  .hint { margin: 0.6rem 0 0; font-size: 0.95rem; color: var(--muted); }
  .empty { color: var(--muted); padding: 0.6rem 0.4rem; font-size: 1.2rem; }
  .err { color: #ff8b6b; font-size: 1.05rem; }
  .link { border: 0; background: transparent; color: var(--accent); font: inherit; text-decoration: underline; }
  .spin { width: 3.4rem; height: 3.4rem; margin: 1.4rem 0; border: 0.35rem solid var(--glass-strong); border-top-color: var(--accent); border-radius: 50%; animation: s 0.8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg); } }
</style>
