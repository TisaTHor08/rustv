<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import { api, type FsEntry } from './api';
  import { t } from './i18n';
  import { lib, scanLibrary } from './library.svelte';
  import { pickFile } from './overlays.svelte';
  import { playMedia } from './player.svelte';
  import { prefs, savePrefs } from './prefs.svelte';
  import { iconSrc } from './tiles.svelte';

  type Tab = 'movies' | 'series' | 'photos' | 'music' | 'files';
  interface Card {
    id: string;
    title: string;
    sub?: string;
    art?: string;
    dir?: boolean;
    open: () => void | Promise<void>;
  }
  interface Level {
    title: string;
    kind: 'grid' | 'list';
    cards: Card[];
  }
  interface Viewer {
    photos: string[];
    i: number;
    url: string;
    slide: boolean;
  }

  let { tab: initial = 'movies', onclose, note }: { tab?: Tab; onclose: () => void; note: (m: string) => void } = $props();

  const TABS: Tab[] = ['movies', 'series', 'photos', 'music', 'files'];
  const COLS = 6;
  const GRID_ROW = 19.5; // rem
  const LIST_ROW = 4.8; // rem

  let tab = $state<Tab>('movies');
  let zone = $state<'tabs' | 'content'>('content');
  let sel = $state(0);
  let stack = $state<Level[]>([]);
  let viewer = $state<Viewer | null>(null);
  let filesRoot = $state<Card[]>([]);
  let busy = $state(false);

  const ext = (n: string) => (n.split('.').pop() ?? '').toLowerCase();
  const IMG = ['jpg', 'jpeg', 'png', 'webp', 'gif', 'bmp', 'avif', 'svg'];
  const VID = ['mp4', 'm4v', 'webm', 'mkv', 'avi', 'mov', 'ts', 'm2ts', 'wmv'];
  const AUD = ['mp3', 'flac', 'ogg', 'wav', 'm4a', 'aac', 'opus'];
  const base = (p: string) => p.split(/[\\/]/).pop() ?? p;

  // ---------- cartes ----------

  async function addFolder() {
    if (tab === 'files') return;
    const p = await pickFile(t('picker.title.folder'), 'folder');
    if (!p) return;
    if (!prefs.library[tab].includes(p)) prefs.library[tab] = [...prefs.library[tab], p];
    savePrefs();
    busy = true;
    try {
      await scanLibrary(true);
    } finally {
      busy = false;
    }
  }

  const addCard = (): Card => ({ id: '__add', title: t('media.add'), sub: '', open: addFolder });

  async function openViewer(photos: string[], i: number) {
    viewer = { photos, i, url: '', slide: false };
    await showPhoto(i);
  }
  async function showPhoto(i: number) {
    if (!viewer) return;
    const p = viewer.photos[i];
    const url = await api.mediaUrl(p);
    if (viewer) {
      viewer.i = i;
      viewer.url = url;
    }
  }

  const rootLevel = $derived.by<Level>(() => {
    const d = lib.data;
    if (tab === 'files') return { title: t('media.files'), kind: 'list', cards: filesRoot };
    if (!d) return { title: '', kind: 'grid', cards: [addCard()] };
    if (tab === 'movies') {
      const cards: Card[] = d.movies.map((m: any) => ({
        id: m.path,
        title: m.title,
        sub: m.year ? String(m.year) : '',
        art: m.poster ?? undefined,
        open: () => playMedia('video', [{ name: m.title, url: m.path }], 0, note),
      }));
      return { title: t('media.movies'), kind: 'grid', cards: [...cards, addCard()] };
    }
    if (tab === 'series') {
      const cards: Card[] = d.series.map((s: any) => {
        const eps = s.seasons.flatMap((x: any) => x.episodes.map((e: any) => ({ ...e, season: x.season })));
        return {
          id: s.show,
          title: s.show,
          sub: `${t('media.seasons', { n: s.seasons.length })} · ${t('media.episodes', { n: eps.length })}`,
          art: s.poster ?? undefined,
          open: () => {
            const items = eps.map((e: any) => ({ name: `${s.show} ${e.title}`, url: e.path }));
            const lvl: Level = {
              title: s.show,
              kind: 'list',
              cards: eps.map((e: any, i: number) => ({
                id: e.path,
                title: e.title,
                sub: t('media.season', { n: e.season }),
                open: () => playMedia('video', items, i, note),
              })),
            };
            push(lvl);
          },
        };
      });
      return { title: t('media.series'), kind: 'grid', cards: [...cards, addCard()] };
    }
    if (tab === 'photos') {
      const cards: Card[] = d.photos.map((a: any) => ({
        id: a.album + a.cover,
        title: a.album,
        sub: t('media.photosCount', { n: a.count }),
        art: a.cover ?? undefined,
        open: () => openViewer(a.photos, 0),
      }));
      return { title: t('media.photos'), kind: 'grid', cards: [...cards, addCard()] };
    }
    // musique
    const cards: Card[] = d.music.map((ar: any) => ({
      id: ar.artist,
      title: ar.artist,
      sub: t('media.albums', { n: ar.albums.length }),
      open: () =>
        push({
          title: ar.artist,
          kind: 'list',
          cards: ar.albums.map((al: any) => ({
            id: ar.artist + al.album,
            title: al.album,
            sub: t('media.tracks', { n: al.tracks.length }),
            open: () =>
              push({
                title: al.album,
                kind: 'list',
                cards: al.tracks.map((tr: any, i: number) => ({
                  id: tr.path,
                  title: tr.title,
                  open: () => playMedia('audio', al.tracks.map((x: any) => ({ name: x.title, url: x.path })), i, note),
                })),
              }),
          })),
        }),
    }));
    return { title: t('media.music'), kind: 'grid', cards: [...cards, addCard()] };
  });

  const level = $derived(stack.length ? stack[stack.length - 1] : rootLevel);
  const cols = $derived(level.kind === 'grid' ? COLS : 1);

  function push(l: Level) {
    stack = [...stack, l];
    sel = 0;
  }

  // ---------- explorateur de fichiers ----------

  async function loadRoots() {
    busy = true;
    try {
      const roots = await api.fsRoots();
      filesRoot = roots.map((r) => ({ id: r.path, title: r.name, dir: true, open: () => openDir(r.path, r.name) }));
    } finally {
      busy = false;
    }
  }

  async function openDir(path: string, title: string) {
    busy = true;
    try {
      const l = await api.fsList(path, 'any');
      const entries = l.entries;
      push({ title, kind: 'list', cards: entries.map((e) => ({ id: e.path, title: e.name, sub: e.dir ? '' : size(e.size), dir: e.dir, open: () => (e.dir ? openDir(e.path, e.name) : openFile(e, entries)) })) });
    } catch (e) {
      note(String(e));
    } finally {
      busy = false;
    }
  }

  async function openFile(e: FsEntry, siblings: FsEntry[]) {
    const x = ext(e.name);
    if (VID.includes(x)) await playMedia('video', [{ name: e.name, url: e.path }], 0, note);
    else if (AUD.includes(x)) {
      const aud = siblings.filter((s) => !s.dir && AUD.includes(ext(s.name)));
      await playMedia('audio', aud.map((s) => ({ name: s.name, url: s.path })), Math.max(0, aud.findIndex((s) => s.path === e.path)), note);
    } else if (IMG.includes(x)) {
      const imgs = siblings.filter((s) => !s.dir && IMG.includes(ext(s.name)));
      await openViewer(imgs.map((s) => s.path), Math.max(0, imgs.findIndex((s) => s.path === e.path)));
    } else {
      try {
        await api.openPath(e.path);
      } catch (err) {
        note(String(err));
      }
    }
  }

  const size = (n: number) => (n > 1e9 ? `${(n / 1e9).toFixed(1)} Go` : n > 1e6 ? `${Math.round(n / 1e6)} Mo` : `${Math.max(1, Math.round(n / 1e3))} Ko`);

  // ---------- navigation ----------

  async function setTab(i: number) {
    tab = TABS[(i + TABS.length) % TABS.length];
    stack = [];
    sel = 0;
    if (tab === 'files') await loadRoots();
    else await ensureScan();
  }

  async function ensureScan() {
    busy = true;
    try {
      await scanLibrary();
    } catch (e) {
      note(String(e));
    } finally {
      busy = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (viewer) {
      if (e.key === 'ArrowRight') showPhoto((viewer.i + 1) % viewer.photos.length);
      else if (e.key === 'ArrowLeft') showPhoto((viewer.i - 1 + viewer.photos.length) % viewer.photos.length);
      else if (e.key === 'Enter') viewer.slide = !viewer.slide;
      else if (e.key === 'Escape' || e.key === 'Backspace') viewer = null;
      else return;
      return void e.preventDefault();
    }
    if (zone === 'tabs') {
      const i = TABS.indexOf(tab);
      if (e.key === 'ArrowRight') setTab(i + 1);
      else if (e.key === 'ArrowLeft') setTab(i - 1);
      else if (e.key === 'ArrowDown') zone = 'content';
      else if (e.key === 'Escape' || e.key === 'Backspace') onclose();
      else return;
      return void e.preventDefault();
    }
    const n = level.cards.length;
    if (e.key === 'ArrowRight') sel = Math.min(n - 1, sel + 1);
    else if (e.key === 'ArrowLeft') sel = Math.max(0, sel - (cols === 1 ? 0 : 1));
    else if (e.key === 'ArrowDown') sel = Math.min(n - 1, sel + cols);
    else if (e.key === 'ArrowUp') {
      if (sel < cols) zone = 'tabs';
      else sel -= cols;
    } else if (e.key === 'Enter') level.cards[sel]?.open();
    else if (e.key === 'Escape' || e.key === 'Backspace') {
      if (stack.length) {
        stack = stack.slice(0, -1);
        sel = 0;
      } else onclose();
    } else return;
    if (sel >= n) sel = Math.max(0, n - 1);
    e.preventDefault();
  }

  // diaporama
  $effect(() => {
    if (!viewer?.slide) return;
    const id = setInterval(() => viewer && showPhoto((viewer.i + 1) % viewer.photos.length), 6000);
    return () => clearInterval(id);
  });

  onMount(() => {
    setTab(TABS.indexOf(initial));
  });

  const rowH = $derived(level.kind === 'grid' ? GRID_ROW : LIST_ROW);
  const selRow = $derived(Math.floor(sel / cols));
  const rowCount = $derived(Math.ceil(level.cards.length / cols));
  const firstRow = $derived(Math.max(0, selRow - 1));
  const lastRow = $derived(Math.min(rowCount - 1, selRow + (level.kind === 'grid' ? 2 : 9)));
  const visible = $derived.by(() => {
    const out: { card: Card; idx: number }[] = [];
    for (let i = firstRow * cols; i < Math.min(level.cards.length, (lastRow + 1) * cols); i++) out.push({ card: level.cards[i], idx: i });
    return out;
  });
  const crumb = $derived([t(`media.${tab}`), ...stack.map((s) => s.title)].join('  ›  '));
</script>

<svelte:window onkeydown={onKey} />

<div class="page" transition:fade={{ duration: 180 }}>
  <header>
    <h1>{t('media.title')}</h1>
    <nav>
      {#each TABS as x, i}
        <button class:on={tab === x} class:focus={zone === 'tabs' && tab === x} onclick={() => setTab(i)}>{t(`media.${x}`)}</button>
      {/each}
    </nav>
  </header>
  <p class="crumb">{crumb}</p>

  <div class="body">
    {#if busy || lib.scanning}
      <p class="msg">{t('media.scanning')}</p>
    {:else if !level.cards.length}
      <p class="msg">{t('media.nofiles')}</p>
    {:else}
      <div class="rows" style:--off="{-firstRow * rowH}rem">
        {#each visible as { card, idx } (card.id + idx)}
          {#if level.kind === 'grid'}
            <button class="card" class:sel={idx === sel && zone === 'content'} style:left="{(idx % cols) * 14.4}rem" style:top="{Math.floor(idx / cols) * rowH}rem" onmouseenter={() => ((sel = idx), (zone = 'content'))} onclick={() => card.open()}>
              <span class="art" class:add={card.id === '__add'}>
                {#if card.art}<img src={iconSrc(card.art)} alt="" />{:else if card.id === '__add'}＋{:else}{card.title[0]}{/if}
              </span>
              <b>{card.title}</b>
              {#if card.sub}<small>{card.sub}</small>{/if}
            </button>
          {:else}
            <button class="item" class:sel={idx === sel && zone === 'content'} style:top="{idx * rowH}rem" onmouseenter={() => ((sel = idx), (zone = 'content'))} onclick={() => card.open()}>
              <span class="ico">{card.dir ? '📁' : tab === 'files' ? '📄' : '▶'}</span>
              <span class="t">{card.title}</span>
              {#if card.sub}<small>{card.sub}</small>{/if}
            </button>
          {/if}
        {/each}
      </div>
    {/if}
  </div>
  <p class="hint">{t('media.hint')}</p>

  {#if viewer}
    <div class="viewer" transition:fade={{ duration: 160 }}>
      {#if viewer.url}<img src={viewer.url} alt="" />{/if}
      <div class="vbar">{viewer.i + 1} / {viewer.photos.length} · {base(viewer.photos[viewer.i])}{viewer.slide ? ' · ▶' : ''}</div>
    </div>
  {/if}
</div>

<style>
  .page { position: absolute; inset: 0; z-index: 25; background: var(--bg); padding: 3.4rem 5rem 2rem; display: flex; flex-direction: column; color: var(--text); }
  header { display: flex; align-items: center; justify-content: space-between; }
  h1 { margin: 0; font-size: 3rem; font-weight: 800; letter-spacing: -0.02em; }
  nav { display: flex; gap: 0.5rem; }
  nav button { padding: 0.6rem 1.5rem; border: 0; border-radius: 99rem; background: var(--glass); color: var(--muted); font: inherit; font-size: 1.25rem; font-weight: 600; transition: background 0.2s, transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1); }
  nav button.on { background: var(--glass-strong); color: var(--text); }
  nav button.focus { background: var(--accent); color: #fff; transform: scale(1.06); }
  .crumb { margin: 0.9rem 0 1.2rem; color: var(--muted); font-size: 1.15rem; min-height: 1.5rem; }
  .body { flex: 1; position: relative; overflow: hidden; margin: 0 -2rem; padding: 0 2rem; }
  .rows { position: absolute; inset: 1rem 2rem auto 2rem; transform: translateY(var(--off)); transition: transform 0.3s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .msg { color: var(--muted); font-size: 1.5rem; padding: 1.5rem 0; }
  .card { position: absolute; width: 13.4rem; height: 18.8rem; display: flex; flex-direction: column; align-items: flex-start; gap: 0.2rem; padding: 0; border: 0; background: transparent; color: var(--text); font: inherit; text-align: left; transition: transform 0.3s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .art { width: 100%; height: 13.6rem; border-radius: 1rem; overflow: hidden; display: grid; place-items: center; background: linear-gradient(135deg, var(--glass-strong), var(--glass)); font-size: 4.4rem; font-weight: 800; color: var(--muted); box-shadow: 0 0.6rem 1.4rem -0.6rem rgba(0, 0, 0, 0.6); }
  .art.add { font-size: 5rem; font-weight: 300; border: 0.15rem dashed var(--glass-strong); background: transparent; }
  .art img { width: 100%; height: 100%; object-fit: cover; }
  .card b { margin-top: 0.4rem; font-size: 1.15rem; font-weight: 600; width: 100%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card small { font-size: 0.95rem; color: var(--muted); width: 100%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card.sel { transform: scale(1.07) translateY(-0.2rem); z-index: 2; }
  .card.sel .art { box-shadow: 0 0 0 0.22rem var(--ring), 0 1.6rem 2.6rem -0.6rem rgba(0, 0, 0, 0.7); }
  .item { position: absolute; left: 0; right: 0; height: 4.2rem; display: flex; align-items: center; gap: 1.2rem; padding: 0 1.4rem; border: 0; border-radius: 0.9rem; background: var(--glass); color: var(--text); font: inherit; font-size: 1.3rem; text-align: left; transition: background 0.2s, transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .item .ico { width: 2rem; text-align: center; font-size: 1.2rem; }
  .item .t { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .item small { color: inherit; opacity: 0.7; font-size: 1.05rem; }
  .item.sel { background: var(--accent); color: #fff; transform: scale(1.012); }
  .hint { margin: 0.8rem 0 0; font-size: 1rem; color: var(--muted); }
  .viewer { position: absolute; inset: 0; z-index: 5; background: #000; display: grid; place-items: center; }
  .viewer img { max-width: 100%; max-height: 100%; object-fit: contain; }
  .vbar { position: absolute; left: 50%; bottom: 1.4rem; transform: translateX(-50%); padding: 0.5rem 1.4rem; border-radius: 99rem; background: rgba(0, 0, 0, 0.65); color: #ddd; font-size: 1.05rem; }
</style>
