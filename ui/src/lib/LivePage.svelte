<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t } from './i18n';
  import { activeSource, live, loadLive, refreshGuide } from './live.svelte';
  import { askText } from './overlays.svelte';
  import { playMedia } from './player.svelte';
  import { prefs, savePrefs } from './prefs.svelte';

  let { onclose, note }: { onclose: () => void; note: (m: string) => void } = $props();

  const VISIBLE = 11;
  const ROW_H = 5; // rem

  let zone = $state<'groups' | 'channels'>('channels');
  let gi = $state(0);
  let ci = $state(0);
  let now = $state(Date.now() / 1000);

  interface GRow {
    label: string;
    sub?: string;
    run: () => void | Promise<void>;
  }

  const groupRows = $derived.by<GRow[]>(() => {
    const rows: GRow[] = [{ label: t('live.groups.all'), sub: t('live.count', { n: live.channels.length }), run: () => select(0) }];
    live.groups.forEach((g, i) => rows.push({ label: g, run: () => select(i + 1) }));
    return rows;
  });
  const actionRows = $derived.by<GRow[]>(() => {
    const rows: GRow[] = [];
    const srcs = prefs.live.sources;
    if (srcs.length > 1) {
      const s = activeSource();
      rows.push({ label: t('live.source', { name: s?.name ?? '' }), run: cycleSource });
    }
    rows.push({ label: t('live.add'), run: addSource });
    if (srcs.length) rows.push({ label: t('live.remove.named', { name: activeSource()?.name ?? '' }), run: removeSource });
    return rows;
  });
  const allRows = $derived([...groupRows, ...actionRows]);

  const list = $derived(gi === 0 ? live.channels : live.channels.filter((c) => c.group === live.groups[gi - 1]));
  const start = $derived(Math.max(0, Math.min(ci - 4, Math.max(0, list.length - VISIBLE))));
  const shown = $derived(list.slice(start, start + VISIBLE));

  function select(i: number) {
    gi = i;
    ci = 0;
    zone = 'channels';
  }

  async function addSource() {
    const name = await askText(t('live.name'), '');
    if (!name) return;
    const m3u = await askText(t('live.m3u'), 'https://');
    if (!m3u) return;
    const xmltv = (await askText(t('live.xmltv'), '', t('live.xmltv'))) ?? '';
    const id = `src${Date.now().toString(36)}`;
    prefs.live.sources = [...prefs.live.sources, { id, name, m3u, xmltv }];
    prefs.live.active = id;
    savePrefs();
    gi = 0;
    ci = 0;
    await loadLive(true);
  }

  async function removeSource() {
    const s = activeSource();
    if (!s) return;
    prefs.live.sources = prefs.live.sources.filter((x) => x.id !== s.id);
    prefs.live.active = prefs.live.sources[0]?.id ?? '';
    savePrefs();
    live.channels = [];
    live.groups = [];
    live.sourceId = '';
    gi = 0;
    await loadLive(true);
  }

  async function cycleSource() {
    const srcs = prefs.live.sources;
    const i = srcs.findIndex((s) => s.id === activeSource()?.id);
    prefs.live.active = srcs[(i + 1) % srcs.length].id;
    savePrefs();
    gi = 0;
    ci = 0;
    await loadLive(true);
  }

  async function play() {
    const c = list[ci];
    if (!c) return;
    await playMedia('live', list.map((x) => ({ name: x.name, url: x.url })), ci, note);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' || e.key === 'Backspace') return void (onclose(), e.preventDefault());
    if (zone === 'groups') {
      if (e.key === 'ArrowDown') gi = Math.min(allRows.length - 1, gi + 1);
      else if (e.key === 'ArrowUp') gi = Math.max(0, gi - 1);
      else if (e.key === 'ArrowRight') zone = 'channels';
      else if (e.key === 'Enter') allRows[gi]?.run();
      else return;
    } else {
      if (e.key === 'ArrowDown') ci = Math.min(list.length - 1, ci + 1);
      else if (e.key === 'ArrowUp') ci = Math.max(0, ci - 1);
      else if (e.key === 'ArrowLeft') zone = 'groups';
      else if (e.key === 'Enter') play();
      else return;
    }
    e.preventDefault();
  }

  const prog = (id: string) => {
    const n = live.nn[id]?.now;
    return n ? Math.max(0, Math.min(100, ((now - n.start) / (n.stop - n.start)) * 100)) : 0;
  };
  const hm = (ts: number) => new Date(ts * 1000).toLocaleTimeString(prefs.lang === 'en' ? 'en-US' : 'fr-FR', { hour: '2-digit', minute: '2-digit' });

  onMount(() => {
    loadLive();
    const a = setInterval(() => (now = Date.now() / 1000), 20000);
    const b = setInterval(refreshGuide, 60000);
    return () => {
      clearInterval(a);
      clearInterval(b);
    };
  });
</script>

<svelte:window onkeydown={onKey} />

<div class="page" transition:fade={{ duration: 180 }}>
  <header>
    <h1>{t('live.title')}</h1>
    <span class="meta">{activeSource() ? activeSource()?.name : ''}{live.hasEpg ? '' : ''}</span>
  </header>

  <div class="cols">
    <aside class:active={zone === 'groups'}>
      {#each allRows as r, i}
        <button class="g" class:sel={i === gi} class:act={i >= groupRows.length} onmouseenter={() => ((gi = i), (zone = 'groups'))} onclick={() => r.run()}>
          <span class="n">{r.label}</span>{#if r.sub}<small>{r.sub}</small>{/if}
        </button>
      {/each}
    </aside>

    <section class:active={zone === 'channels'}>
      {#if live.loading}
        <p class="msg">{t('live.loading')}</p>
      {:else if !prefs.live.sources.length}
        <p class="msg">{t('live.empty')}</p>
      {:else if live.error && !live.channels.length}
        <p class="msg err">{live.error}</p>
      {:else}
        <div class="rows" style:--off="{-start * ROW_H}rem">
          {#each shown as c, i (c.id)}
            {@const idx = start + i}
            {@const n = live.nn[c.id]}
            <button class="ch" class:sel={idx === ci && zone === 'channels'} class:cur={idx === ci} onmouseenter={() => ((ci = idx), (zone = 'channels'))} onclick={() => ((ci = idx), play())} style:top="{idx * ROW_H}rem">
              <span class="logo">{#if c.logo}<img src={c.logo} alt="" loading="lazy" />{:else}{c.name[0]}{/if}</span>
              <span class="info">
                <b>{c.name}</b>
                {#if n?.now}
                  <span class="now">{hm(n.now.start)} · {n.now.title}</span>
                  <i class="pb"><u style:width="{prog(c.id)}%"></u></i>
                {:else}<span class="now dim">{c.group}</span>{/if}
              </span>
              {#if n?.next}<span class="next">{hm(n.next.start)} {n.next.title}</span>{/if}
            </button>
          {/each}
        </div>
        {#if live.error}<p class="foot err">{live.error}</p>{/if}
      {/if}
    </section>
  </div>
  <p class="hint">{t('live.hint')} · {t('live.zap')}</p>
</div>

<style>
  .page { position: absolute; inset: 0; z-index: 25; background: var(--bg); padding: 3.4rem 5rem 2rem; display: flex; flex-direction: column; color: var(--text); }
  header { display: flex; align-items: baseline; gap: 1.4rem; margin-bottom: 1.4rem; }
  h1 { margin: 0; font-size: 3rem; font-weight: 800; letter-spacing: -0.02em; }
  .meta { color: var(--muted); font-size: 1.3rem; }
  .cols { flex: 1; display: flex; gap: 2.4rem; min-height: 0; }
  aside { width: 26rem; flex: none; overflow: hidden auto; scrollbar-width: none; }
  .g { display: flex; flex-direction: column; align-items: flex-start; width: 100%; margin: 0.15rem 0; padding: 0.7rem 1.1rem; border: 0; border-radius: 0.9rem; background: transparent; color: var(--muted); font: inherit; font-size: 1.25rem; text-align: left; transition: background 0.2s, transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .g small { font-size: 0.9rem; opacity: 0.7; }
  .g.act { font-style: italic; }
  .g.sel { background: var(--glass-strong); color: var(--text); }
  aside.active .g.sel { background: var(--accent); color: #fff; font-weight: 600; transform: scale(1.02); }
  section { flex: 1; position: relative; overflow: hidden; }
  .rows { position: absolute; left: 0; right: 0; top: 0; transform: translateY(var(--off)); transition: transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .ch { position: absolute; left: 0; right: 0; height: 4.6rem; display: flex; align-items: center; gap: 1.2rem; padding: 0 1.2rem; border: 0; border-radius: 1rem; background: var(--glass); color: var(--text); font: inherit; text-align: left; transition: background 0.2s, transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .ch.cur { background: var(--glass-strong); }
  .ch.sel { background: var(--accent); color: #fff; transform: scale(1.012); }
  .logo { flex: none; width: 3.4rem; height: 3.4rem; border-radius: 0.7rem; display: grid; place-items: center; background: rgba(255, 255, 255, 0.92); color: #333; font-weight: 800; overflow: hidden; }
  .logo img { width: 100%; height: 100%; object-fit: contain; }
  .info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0.1rem; }
  .info b { font-size: 1.35rem; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .now { font-size: 1.05rem; opacity: 0.85; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .dim { opacity: 0.55; }
  .pb { display: block; height: 0.28rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.25); overflow: hidden; }
  .pb u { display: block; height: 100%; background: currentColor; opacity: 0.9; text-decoration: none; }
  .next { flex: none; max-width: 24rem; font-size: 1rem; opacity: 0.65; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .msg { color: var(--muted); font-size: 1.5rem; max-width: 50rem; line-height: 1.5; padding: 2rem 0; }
  .err { color: #ff8b6b; }
  .foot { position: absolute; bottom: 0; margin: 0; font-size: 0.95rem; }
  .hint { margin: 1rem 0 0; font-size: 1rem; color: var(--muted); }
</style>
