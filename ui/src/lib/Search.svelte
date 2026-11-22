<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import { t } from './i18n';
  import { lib, scanLibrary } from './library.svelte';
  import { live, loadLive } from './live.svelte';
  import { playMedia } from './player.svelte';
  import { prefs } from './prefs.svelte';
  import { iconSrc, type DTile } from './tiles.svelte';

  let {
    tiles,
    onclose,
    onopen,
    onmedia,
    note,
  }: { tiles: DTile[]; onclose: () => void; onopen: (t: DTile) => void; onmedia: (tab: 'movies' | 'series' | 'photos' | 'music') => void; note: (m: string) => void } = $props();

  let q = $state('');
  let idx = $state(0);
  let input = $state<HTMLInputElement | null>(null);

  interface Hit {
    section: 'apps' | 'channels' | 'library';
    title: string;
    sub?: string;
    color?: string;
    icon?: string;
    run: () => void;
  }

  onMount(() => {
    tick().then(() => input?.focus());
    // Les chaînes et la médiathèque sont chargées en arrière-plan pour pouvoir être cherchées.
    if (prefs.live.sources.length) loadLive().catch(() => {});
    scanLibrary().catch(() => {});
  });

  const hits = $derived.by<Hit[]>(() => {
    const s = q.trim().toLowerCase();
    if (!s) return [];
    const out: Hit[] = [];
    for (const x of tiles) {
      if (x.name.toLowerCase().includes(s)) out.push({ section: 'apps', title: x.name, sub: x.desc, color: x.color, icon: x.icon, run: () => (onclose(), onopen(x)) });
    }
    const chans = live.channels.filter((c) => c.name.toLowerCase().includes(s)).slice(0, 20);
    for (const c of chans) {
      out.push({
        section: 'channels',
        title: c.name,
        sub: c.group,
        run: () => {
          onclose();
          const list = live.channels;
          playMedia('live', list.map((x) => ({ name: x.name, url: x.url })), list.findIndex((x) => x.id === c.id), note);
        },
      });
    }
    const d = lib.data;
    if (d) {
      for (const m of d.movies.filter((m: any) => m.title.toLowerCase().includes(s)).slice(0, 10)) {
        out.push({ section: 'library', title: m.title, sub: t('media.movies'), icon: m.poster, run: () => (onclose(), playMedia('video', [{ name: m.title, url: m.path }], 0, note)) });
      }
      for (const sh of d.series.filter((x: any) => x.show.toLowerCase().includes(s)).slice(0, 6)) {
        out.push({ section: 'library', title: sh.show, sub: t('media.series'), icon: sh.poster, run: () => (onclose(), onmedia('series')) });
      }
      for (const a of d.music.filter((x: any) => x.artist.toLowerCase().includes(s) || x.albums.some((al: any) => al.album.toLowerCase().includes(s))).slice(0, 6)) {
        out.push({ section: 'library', title: a.artist, sub: t('media.music'), run: () => (onclose(), onmedia('music')) });
      }
      for (const p of d.photos.filter((x: any) => x.album.toLowerCase().includes(s)).slice(0, 6)) {
        out.push({ section: 'library', title: p.album, sub: t('media.photos'), icon: p.cover, run: () => (onclose(), onmedia('photos')) });
      }
    }
    return out;
  });

  $effect(() => {
    q;
    idx = 0;
  });

  const label = (s: Hit['section']) => t(s === 'apps' ? 'search.apps' : s === 'channels' ? 'search.channels' : 'search.library');

  function onKey(e: KeyboardEvent) {
    const stop = () => (e.preventDefault(), e.stopImmediatePropagation());
    if (e.key === 'Escape') {
      stop();
      onclose();
    } else if (e.key === 'ArrowDown') {
      stop();
      idx = Math.min(hits.length - 1, idx + 1);
    } else if (e.key === 'ArrowUp') {
      stop();
      idx = Math.max(0, idx - 1);
    } else if (e.key === 'Enter') {
      stop();
      hits[idx]?.run();
    } else e.stopImmediatePropagation(); // le reste va au champ de saisie
    tick().then(() => document.querySelector('.hit.sel')?.scrollIntoView({ block: 'nearest' }));
  }
</script>

<svelte:window onkeydowncapture={onKey} />

<div class="scrim" transition:fade={{ duration: 160 }}>
  <div class="sheet" in:fly={{ y: 30, duration: 320, easing: cubicOut }}>
    <input bind:this={input} bind:value={q} placeholder={t('search.placeholder')} autocomplete="off" spellcheck="false" />
    <div class="list">
      {#if q.trim() && !hits.length}<p class="none">{t('search.none')}</p>{/if}
      {#each hits as h, i}
        {#if i === 0 || hits[i - 1].section !== h.section}<h4>{label(h.section)}</h4>{/if}
        <button class="hit" class:sel={i === idx} onmouseenter={() => (idx = i)} onclick={() => h.run()}>
          <span class="ic" style:background={h.color ?? 'var(--glass-strong)'}>{#if h.icon}<img src={iconSrc(h.icon)} alt="" />{:else}{h.title[0]}{/if}</span>
          <span class="tx"><b>{h.title}</b>{#if h.sub}<small>{h.sub}</small>{/if}</span>
        </button>
      {/each}
    </div>
    <p class="hint">{t('search.hint')}</p>
  </div>
</div>

<style>
  .scrim { position: absolute; inset: 0; background: rgba(4, 6, 10, 0.8); backdrop-filter: blur(1.6rem); display: grid; place-items: start center; padding-top: 6rem; z-index: 30; }
  .sheet { width: 62rem; padding: 2rem 2.4rem; border-radius: 1.8rem; background: var(--sheet); box-shadow: 0 3rem 6rem rgba(0, 0, 0, 0.6), inset 0 0 0 0.08rem var(--glass-strong); color: var(--text); }
  input { width: 100%; padding: 1.1rem 1.4rem; border-radius: 1rem; border: 0.12rem solid var(--glass-strong); background: var(--field); color: var(--text); font-size: 1.7rem; outline: none; }
  input:focus { border-color: var(--accent); }
  .list { margin-top: 1rem; max-height: 34rem; overflow: hidden auto; scrollbar-width: none; }
  h4 { margin: 1rem 0 0.3rem; font-size: 0.95rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.12em; }
  .hit { display: flex; align-items: center; gap: 1.1rem; width: 100%; padding: 0.6rem 0.9rem; border: 0; border-radius: 0.9rem; background: transparent; color: var(--text); font: inherit; text-align: left; transition: background 0.15s, transform 0.2s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .hit.sel { background: var(--accent); color: #fff; transform: scale(1.015); }
  .ic { flex: none; width: 3.2rem; height: 3.2rem; border-radius: 0.8rem; display: grid; place-items: center; font-weight: 800; font-size: 1.3rem; color: #fff; overflow: hidden; }
  .ic img { width: 100%; height: 100%; object-fit: cover; }
  .tx { display: flex; flex-direction: column; min-width: 0; }
  .tx b { font-size: 1.3rem; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tx small { font-size: 0.95rem; opacity: 0.7; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .none { color: var(--muted); font-size: 1.3rem; padding: 1rem 0.4rem; }
  .hint { margin: 1rem 0 0; font-size: 0.95rem; color: var(--muted); }
</style>
