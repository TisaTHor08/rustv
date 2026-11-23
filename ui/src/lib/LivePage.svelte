<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { fade } from 'svelte/transition';
  import type { LiveChannel } from './api';
  import { t } from './i18n';
  import {
    activeSource, addReminder, addSeries, blockPos, dvr, dvrError, favs, fetchEpg, isFav, isScheduled, live, loadDvr, loadLive, recordNow, refreshGuide, scheduleProg, startReminders, toggleFav,
    type EpgProg, type Recording,
  } from './live.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { eg, enableEpg, fetchBuiltinGuide, guideTarget, hideBroken, loadEpg, safeImg, tonightTs } from './epg.svelte';
  import { askConfirm, askText } from './overlays.svelte';
  import { playMedia } from './player.svelte';
  import { prefs, savePrefs } from './prefs.svelte';

  let { onclose, note }: { onclose: () => void; note: (m: string) => void } = $props();

  const VISIBLE = 11;
  const ROW_H = 5; // rem
  const G_VIS = 8; // chaînes visibles dans le guide
  const SPAN = 3 * 3600; // fenêtre du guide : 3 h
  const STEP = 1800;

  type Mode = 'list' | 'guide' | 'rec';
  let mode = $state<Mode>('list');
  let zone = $state<'groups' | 'channels'>('channels');
  let gi = $state(0);
  let ci = $state(0);
  let now = $state(Date.now() / 1000);

  // guide
  let wStart = $state(Math.floor(Date.now() / 1000 / STEP) * STEP - STEP);
  let selT = $state(Date.now() / 1000);
  let gprogs = $state<Record<string, EpgProg[]>>({});
  let reqId = 0;

  // enregistrements
  let ri = $state(0);

  interface MenuItem {
    label: string;
    run: () => void | Promise<void>;
  }
  let menu = $state<{ title: string; items: MenuItem[]; i: number } | null>(null);

  interface GRow {
    label: string;
    sub?: string;
    run: () => void | Promise<void>;
  }

  const FAV_ROW = 1;
  const groupRows = $derived.by<GRow[]>(() => {
    const rows: GRow[] = [
      { label: t('live.groups.all'), sub: t('live.count', { n: live.channels.length }), run: () => select(0) },
      { label: t('lv.fav'), sub: String(live.channels.filter((c) => favs.names.includes(c.name)).length), run: () => select(FAV_ROW) },
    ];
    live.groups.forEach((g, i) => rows.push({ label: g, run: () => select(i + 2) }));
    return rows;
  });
  const viewLabel = $derived(t(`lv.view.${mode}`));
  const actionRows = $derived.by<GRow[]>(() => {
    const rows: GRow[] = [{ label: t('lv.view', { v: viewLabel }), run: cycleView }];
    if (mode === 'guide' && guideKind !== 'off') {
      rows.push({ label: t('epg.tonight'), run: goTonight });
      rows.push({ label: t('epg.day.prev'), run: () => jump(selT - 86400) });
      rows.push({ label: t('epg.day.next'), run: () => jump(selT + 86400) });
    }
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

  const list = $derived.by<LiveChannel[]>(() => {
    if (gi === 0 || gi >= groupRows.length) return live.channels;
    if (gi === FAV_ROW) return live.channels.filter((c) => favs.names.includes(c.name));
    return live.channels.filter((c) => c.group === live.groups[gi - 2]);
  });
  const start = $derived(Math.max(0, Math.min(ci - 4, Math.max(0, list.length - VISIBLE))));
  const shown = $derived(list.slice(start, start + VISIBLE));

  // Guide : le guide de la source de l'utilisateur (XMLTV) prime ; sinon le guide TV automatique (epg.rs).
  interface GCh {
    id: string;
    name: string;
    logo: string;
    live: LiveChannel | null;
  }
  const guideKind = $derived<'own' | 'builtin' | 'off'>(live.hasEpg ? 'own' : eg.enabled && eg.loaded ? 'builtin' : 'off');
  const glist = $derived.by<GCh[]>(() => {
    if (guideKind === 'builtin') {
      const byId = new Map(live.channels.map((c) => [c.id, c]));
      let ch = eg.channels;
      if (gi === FAV_ROW) ch = ch.filter((c) => favs.names.includes(c.name));
      return ch.map((c) => ({ id: c.id, name: c.name, logo: safeImg(c.icon) || (c.liveChannelId ? (byId.get(c.liveChannelId)?.logo ?? '') : ''), live: c.liveChannelId ? (byId.get(c.liveChannelId) ?? null) : null }));
    }
    return list.map((c) => ({ id: c.id, name: c.name, logo: c.logo, live: c }));
  });
  const gc = $derived(glist[ci]);
  const lo = $derived(guideKind === 'builtin' && eg.from ? eg.from : now - 6 * 3600);
  const hi = $derived(guideKind === 'builtin' && eg.to ? eg.to : now + 47 * 3600);
  const gStart = $derived(Math.max(0, Math.min(ci - 3, Math.max(0, glist.length - G_VIS))));
  const gShown = $derived(glist.slice(gStart, gStart + G_VIS));

  const progsOf = (c?: { id: string }) => (c ? (gprogs[c.id] ?? []) : []);
  const curProg = $derived(progsOf(gc).find((p) => p.start <= selT && selT < p.stop) ?? null);
  const ticks = $derived(Array.from({ length: SPAN / STEP }, (_, i) => ({ ts: wStart + i * STEP, left: (i * STEP * 100) / SPAN })));
  const nowLeft = $derived(((now - wStart) / SPAN) * 100);
  const recs = $derived([...dvr.recs].sort((a, b) => b.start - a.start));

  function select(i: number) {
    gi = i;
    ci = 0;
    zone = 'channels';
  }

  function cycleView() {
    mode = mode === 'list' ? 'guide' : mode === 'guide' ? 'rec' : 'list';
    zone = 'channels';
    if (mode === 'guide') {
      selT = Math.max(selT, now);
      wStart = Math.floor(now / STEP) * STEP - STEP;
    }
    if (mode === 'rec') loadDvr();
  }

  async function addSource() {
    const name = await askText(t('live.name'), '');
    if (!name) return;
    const m3u = await askText(t('live.m3u'), 'https://');
    if (!m3u) return;
    const xmltv = (await askText(t('live.xmltv'), '', t('live.xmltv'))) ?? '';
    const id = `src${Date.now().toString(36)}`;
    prefs.live.sources = [...prefs.live.sources, { id, name, m3u, xmltv, kind: 'm3u' }];
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

  /** Regarde la chaîne du guide si elle correspond à une chaîne en direct de l'utilisateur. */
  async function playG(g?: GCh) {
    const c = g?.live;
    if (!c) return note(t('epg.noplay'));
    await playMedia('live', live.channels.map((x) => ({ name: x.name, url: x.url })), live.channels.findIndex((x) => x.id === c.id), note);
  }

  /** Place le guide sur l'instant `ts` (borné à la plage connue). */
  function jump(ts: number) {
    selT = Math.max(lo, Math.min(hi - 60, ts));
    wStart = Math.floor((selT - 3600) / STEP) * STEP;
  }
  async function goTonight() {
    try {
      jump(await tonightTs());
    } catch {}
    zone = 'channels';
  }

  const run = async (fn: () => Promise<unknown>, ok = '') => {
    try {
      await fn();
      if (ok) note(ok);
    } catch (e) {
      note(dvrError(e));
    }
  };

  // ---------- menus d'actions ----------

  function openChannelMenu() {
    const c = list[ci];
    if (!c) return;
    menu = {
      title: c.name,
      i: 0,
      items: [
        { label: t('lv.menu.play'), run: play },
        { label: t('lv.menu.recordnow'), run: () => recNow(c) },
        { label: isFav(c.name) ? t('lv.fav.del') : t('lv.fav.add'), run: () => toggleFav(c.name) },
        { label: t('lv.menu.close'), run: () => {} },
      ],
    };
  }

  async function recNow(c: LiveChannel) {
    const v = await askText(t('lv.minutes'), '60');
    const m = Math.round(Number(v));
    if (!v || !(m >= 1 && m <= 720)) return;
    await run(() => recordNow(c, m), t('lv.rec.ok'));
  }

  function openProgMenu() {
    const g = gc;
    if (!g) return;
    const c = g.live;
    const p = curProg;
    const items: MenuItem[] = [];
    if (c) items.push({ label: p && p.start <= now && now < p.stop ? t('lv.menu.play') : t('lv.menu.playprog'), run: () => playG(g) });
    if (c && p && p.stop > now) {
      if (!isScheduled(c.name, p.start)) items.push({ label: t('lv.menu.record'), run: () => run(async () => note(await scheduleProg(c, p))) });
      if (p.title) items.push({ label: t('lv.menu.series'), run: () => run(async () => (await addSeries(c, p.title), await scheduleProg(c, p).catch(() => ''), note(t('lv.rec.series.ok')))) });
    }
    if (p && p.start > now) items.push({ label: t('lv.menu.remind'), run: () => (addReminder({ channelName: g.name, title: p.title, start: p.start }), note(t('lv.remind.ok'))) });
    if (c) items.push({ label: t('lv.menu.recordnow'), run: () => recNow(c) });
    items.push({ label: isFav(g.name) ? t('lv.fav.del') : t('lv.fav.add'), run: () => toggleFav(g.name) });
    items.push({ label: t('lv.menu.close'), run: () => {} });
    menu = { title: p?.title || g.name, i: 0, items };
  }

  function openRecMenu() {
    const r = recs[ri];
    if (!r) return;
    const items: MenuItem[] = [];
    if (r.status === 'done' || (r.status === 'failed' && r.file)) items.push({ label: t('lv.rec.play'), run: () => playRec(r) });
    if (r.status === 'scheduled' || r.status === 'recording') items.push({ label: t('lv.rec.cancel'), run: () => cancelRec(r) });
    else {
      items.push({ label: t('lv.rec.delete'), run: () => delRec(r, false) });
      if (r.file) items.push({ label: t('lv.rec.deletefile'), run: () => delRec(r, true) });
    }
    items.push({ label: t('lv.menu.close'), run: () => {} });
    menu = { title: r.title, i: 0, items };
  }

  async function playRec(r: Recording) {
    if (!r.file) return note(t('lv.rec.nofile'));
    // « video » : le lecteur applique la reprise de lecture.
    await playMedia('video', [{ name: r.title, url: r.file }], 0, note);
  }
  async function cancelRec(r: Recording) {
    if (!(await askConfirm(t('lv.rec.confirm.cancel', { t: r.title }), '', t('lv.rec.cancel'), t('common.cancel')))) return;
    await run(() => invoke('dvr_cancel', { id: r.id }));
    await loadDvr();
  }
  async function delRec(r: Recording, withFile: boolean) {
    if (!(await askConfirm(t('lv.rec.confirm', { t: r.title }), t('lv.rec.confirm.text'), withFile ? t('lv.rec.deletefile') : t('lv.rec.delete'), t('common.cancel')))) return;
    await run(() => invoke('dvr_delete', { id: r.id, deleteFile: withFile }));
    await loadDvr();
    ri = Math.max(0, Math.min(ri, dvr.recs.length - 1));
  }

  // ---------- navigation ----------

  function ensureVisible() {
    if (selT < wStart) wStart = Math.floor(selT / STEP) * STEP;
    else if (selT >= wStart + SPAN - STEP) wStart = Math.floor((selT - 3600) / STEP) * STEP;
  }
  function shiftWindow(dir: number) {
    selT += dir * STEP;
    wStart += dir * STEP;
  }

  function guideKey(e: KeyboardEvent): boolean {
    if (guideKind === 'off') {
      if (e.key === 'Enter') enableEpg(note).then((ok) => ok && (jump(now), (zone = 'channels')));
      else if (e.key === 'ArrowLeft') zone = 'groups';
      else return false;
      return true;
    }
    if (e.key === 'PageDown') jump(selT + 86400);
    else if (e.key === 'PageUp') jump(selT - 86400);
    else if (e.key === 'ArrowDown') ci = Math.min(glist.length - 1, ci + 1);
    else if (e.key === 'ArrowUp') ci = Math.max(0, ci - 1);
    else if (e.key === 'ArrowLeft') {
      const p = curProg;
      const row = progsOf(gc);
      const prev = p ? [...row].reverse().find((x) => x.stop <= p.start) : undefined;
      if (prev) {
        selT = prev.start;
        ensureVisible();
      } else if (!p) shiftWindow(-1);
      else zone = 'groups';
    } else if (e.key === 'ArrowRight') {
      const p = curProg;
      const next = progsOf(gc).find((x) => x.start >= (p ? p.stop : selT) && x.start > selT);
      if (next) {
        selT = next.start;
        ensureVisible();
      } else if (selT < hi - STEP) shiftWindow(1);
    } else if (e.key === 'Enter') {
      const p = curProg;
      if (!p ? !!gc?.live : p.start <= now && now < p.stop && !!gc?.live) playG(gc);
      else if (p) openProgMenu();
      else if (gc) note(t('epg.noplay'));
    } else if (e.key === 'ContextMenu') openProgMenu();
    else return false;
    return true;
  }

  function onKey(e: KeyboardEvent) {
    if (menu) {
      if (e.key === 'ArrowDown') menu.i = Math.min(menu.items.length - 1, menu.i + 1);
      else if (e.key === 'ArrowUp') menu.i = Math.max(0, menu.i - 1);
      else if (e.key === 'Enter') {
        const it = menu.items[menu.i];
        menu = null;
        it?.run();
      } else if (e.key === 'Escape' || e.key === 'Backspace' || e.key === 'ContextMenu') menu = null;
      else return;
      return e.preventDefault();
    }
    if (e.key === 'Escape' || e.key === 'Backspace') return void (onclose(), e.preventDefault());
    if (zone === 'groups') {
      if (e.key === 'ArrowDown') gi = Math.min(allRows.length - 1, gi + 1);
      else if (e.key === 'ArrowUp') gi = Math.max(0, gi - 1);
      else if (e.key === 'ArrowRight') zone = 'channels';
      else if (e.key === 'Enter') allRows[gi]?.run();
      else return;
    } else if (mode === 'guide') {
      if (!guideKey(e)) return;
    } else if (mode === 'rec') {
      if (e.key === 'ArrowDown') ri = Math.min(recs.length - 1, ri + 1);
      else if (e.key === 'ArrowUp') ri = Math.max(0, ri - 1);
      else if (e.key === 'ArrowLeft') zone = 'groups';
      else if (e.key === 'Enter' || e.key === 'ContextMenu') openRecMenu();
      else return;
    } else {
      if (e.key === 'ArrowDown') ci = Math.min(list.length - 1, ci + 1);
      else if (e.key === 'ArrowUp') ci = Math.max(0, ci - 1);
      else if (e.key === 'ArrowLeft') zone = 'groups';
      else if (e.key === 'Enter') play();
      else if (e.key === 'ContextMenu') openChannelMenu();
      else return;
    }
    e.preventDefault();
  }

  // Charge les programmes de la fenêtre pour les chaînes visibles (avec une marge pour naviguer).
  $effect(() => {
    if (mode !== 'guide' || guideKind === 'off') return;
    const ids = glist.slice(Math.max(0, gStart - 1), gStart + G_VIS + 1).map((c) => c.id);
    const from = wStart - 3600;
    const to = wStart + SPAN + 4 * 3600;
    const my = ++reqId;
    (guideKind === 'builtin' ? fetchBuiltinGuide : fetchEpg)(from, to, ids)
      .then((r) => {
        if (my === reqId) gprogs = { ...gprogs, ...r };
      })
      .catch(() => {});
  });
  // « Voir dans le guide » depuis la recherche : se place sur la chaîne et l'heure du programme.
  $effect(() => {
    const tg = guideTarget.v;
    if (!tg || mode !== 'guide' || guideKind === 'off' || !glist.length) return;
    const i = glist.findIndex((g) => g.id === tg.channelId || (!!tg.liveChannelId && g.live?.id === tg.liveChannelId));
    untrack(() => {
      if (i >= 0) ci = i;
      jump(tg.start);
      zone = 'channels';
      guideTarget.v = null;
    });
  });
  $effect(() => {
    void gi;
    const n = mode === 'guide' ? glist.length : list.length;
    if (ci >= n) ci = Math.max(0, n - 1);
  });

  const prog = (id: string) => {
    const n = live.nn[id]?.now;
    return n ? Math.max(0, Math.min(100, ((now - n.start) / (n.stop - n.start)) * 100)) : 0;
  };
  const loc = () => (prefs.lang === 'en' ? 'en-US' : 'fr-FR');
  const hm = (ts: number) => new Date(ts * 1000).toLocaleTimeString(loc(), { hour: '2-digit', minute: '2-digit' });
  const dayhm = (ts: number) => new Date(ts * 1000).toLocaleString(loc(), { weekday: 'short', day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });
  const gb = (n: number) => (n / 2 ** 30 >= 10 ? (n / 2 ** 30).toFixed(0) : (n / 2 ** 30).toFixed(1));
  const statusText = (r: Recording) => t(`lv.rec.${r.status}`) + (r.status === 'failed' && r.error ? ` · ${dvrError(r.error)}` : '');

  onMount(() => {
    if (guideTarget.v) mode = 'guide';
    loadLive();
    loadEpg().then(() => refreshGuide());
    loadDvr();
    startReminders(note);
    const a = setInterval(() => (now = Date.now() / 1000), 20000);
    const b = setInterval(refreshGuide, 60000);
    const c = setInterval(loadDvr, 30000);
    return () => {
      clearInterval(a);
      clearInterval(b);
      clearInterval(c);
    };
  });
</script>

<svelte:window onkeydown={onKey} />

<div class="page" transition:fade={{ duration: 180 }}>
  <header>
    <h1>{t('live.title')}</h1>
    <span class="meta">{activeSource() ? activeSource()?.name : ''}</span>
    <span class="meta mode">{viewLabel}{dvr.running ? ' · ● ' + dvr.running : ''}</span>
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
      {#if mode === 'rec'}
        {#if !recs.length}
          <p class="msg">{t('lv.rec.empty')}</p>
        {:else}
          {#if dvr.freeBytes != null}<p class="free">{t('lv.rec.free', { n: gb(dvr.freeBytes) })}</p>{/if}
          <div class="reclist">
            {#each recs.slice(Math.max(0, Math.min(ri - 4, recs.length - 9)), Math.max(0, Math.min(ri - 4, recs.length - 9)) + 9) as r (r.id)}
              <button class="rec" class:sel={recs[ri]?.id === r.id && zone === 'channels'} class:cur={recs[ri]?.id === r.id} onclick={() => ((ri = recs.findIndex((x) => x.id === r.id)), openRecMenu())}>
                <i class="dot {r.status}"></i>
                <span class="info"><b>{r.title}</b><span class="now">{r.channelName} · {dayhm(r.start)}</span></span>
                <span class="next">{statusText(r)}{r.size ? ' · ' + gb(r.size) + ' GB' : ''}</span>
              </button>
            {/each}
          </div>
        {/if}
      {:else if mode === 'guide' && guideKind === 'off' && !(live.loading && prefs.live.sources.length)}
        <div class="cta">
          <h2>{t('epg.cta.title')}</h2>
          <p>{t('epg.cta.sub')}</p>
          {#if !prefs.live.sources.length}<p>{t('epg.cta.nosource')}</p>{/if}
          <p class="dim">{t('epg.cta.notice')}</p>
          {#if eg.enabled}<p class="msg">{t('epg.empty', { e: '' })}</p>{:else}<button class="ctab" class:sel={zone === 'channels'} onclick={() => enableEpg(note)}>{t('epg.cta.enter')}</button>{/if}
        </div>
      {:else if guideKind !== 'builtin' && live.loading}
        <p class="msg">{t('live.loading')}</p>
      {:else if guideKind !== 'builtin' && !prefs.live.sources.length}
        <p class="msg">{t('live.empty')}</p>
      {:else if guideKind !== 'builtin' && live.error && !live.channels.length}
        <p class="msg err">{live.error}</p>
      {:else if mode === 'guide'}
        {#if false}
          <p class="msg">{t('lv.guide.empty')}</p>
        {:else}
          <div class="guide">
            <div class="grow head">
              <div class="lab">{hm(now)}</div>
              <div class="track">
                {#each ticks as k}<span class="tick" style:left="{k.left}%">{hm(k.ts)}</span>{/each}
                {#if nowLeft >= 0 && nowLeft <= 100}<i class="nowline" style:left="{nowLeft}%"></i>{/if}
              </div>
            </div>
            {#each gShown as c, i (c.id)}
              {@const idx = gStart + i}
              <div class="grow" class:cur={idx === ci}>
                <div class="lab">
                  <span class="logo">{#if c.logo}<img src={c.logo} alt="" loading="lazy" onerror={hideBroken} />{:else}{c.name[0]}{/if}</span>
                  <b>{isFav(c.name) ? '★ ' : ''}{c.name}</b>
                </div>
                <div class="track">
                  {#each progsOf(c) as p (p.start)}
                    {@const pos = blockPos(p, wStart, SPAN)}
                    {#if pos}
                      <button
                        class="blk"
                        class:sel={idx === ci && zone === 'channels' && curProg?.start === p.start}
                        class:onair={p.start <= now && now < p.stop}
                        class:past={p.stop <= now}
                        style:left="{pos.left}%"
                        style:width="calc({pos.width}% - 0.2rem)"
                        onmouseenter={() => ((ci = idx), (selT = Math.max(p.start, wStart)), (zone = 'channels'))}
                        onclick={() => ((ci = idx), (selT = Math.max(p.start, wStart)), p.start <= now && now < p.stop && c.live ? playG(c) : openProgMenu())}
                      >
                        {#if c.live && isScheduled(c.live.name, p.start)}<i class="reddot"></i>{/if}
                        <span class="t">{p.title}</span>
                      </button>
                    {/if}
                  {:else}
                    <span class="noepg">{t('lv.guide.noepg')}</span>
                  {/each}
                  {#if nowLeft >= 0 && nowLeft <= 100}<i class="nowline" style:left="{nowLeft}%"></i>{/if}
                </div>
              </div>
            {/each}
            <div class="details">
              {#if curProg}
                {#if safeImg(curProg.icon)}<img class="pic" src={safeImg(curProg.icon)} alt="" loading="lazy" onerror={hideBroken} />{/if}
                <h2>{curProg.title}</h2>
                <p class="when">{dayhm(curProg.start)} – {hm(curProg.stop)} · {t('epg.min', { n: Math.round((curProg.stop - curProg.start) / 60) })} · {gc?.name}{curProg.category ? ' · ' + curProg.category : ''}{curProg.rating ? ' · ' + curProg.rating : ''}{gc?.live && isScheduled(gc.live.name, curProg.start) ? ' · ● ' + t('lv.rec.scheduled') : ''}</p>
                <p class="desc">{curProg.desc}</p>
                {#if gc && !gc.live}<p class="when">{t('epg.guide.readonly')}</p>{/if}
              {:else if gc}
                <h2>{gc.name}</h2>
                <p class="when">{t('lv.guide.noepg')}</p>
              {/if}
            </div>
          </div>
        {/if}
      {:else if !list.length}
        <p class="msg">{gi === FAV_ROW ? t('lv.fav.none') : t('live.empty')}</p>
      {:else}
        <div class="rows" style:--off="{-start * ROW_H}rem">
          {#each shown as c, i (c.id)}
            {@const idx = start + i}
            {@const n = live.nn[c.id]}
            <button class="ch" class:sel={idx === ci && zone === 'channels'} class:cur={idx === ci} onmouseenter={() => ((ci = idx), (zone = 'channels'))} onclick={() => ((ci = idx), play())} style:top="{idx * ROW_H}rem">
              <span class="logo">{#if c.logo}<img src={c.logo} alt="" loading="lazy" />{:else}{c.name[0]}{/if}</span>
              <span class="info">
                <b>{isFav(c.name) ? '★ ' : ''}{c.name}</b>
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
  <p class="hint">{mode === 'guide' ? (guideKind === 'off' ? t('epg.cta.enter') : t('epg.guide.hint')) : mode === 'rec' ? t('lv.rec.hint') : t('live.hint')} · {t('live.zap')}</p>

  {#if menu}
    <div class="menuwrap" transition:fade={{ duration: 120 }}>
      <div class="menu">
        <h3>{menu.title}</h3>
        {#each menu.items as it, i}
          <button class:sel={i === menu.i} onmouseenter={() => menu && (menu.i = i)} onclick={() => ((menu = null), it.run())}>{it.label}</button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .page { position: absolute; inset: 0; z-index: 25; background: var(--bg); padding: 3.4rem 5rem 2rem; display: flex; flex-direction: column; color: var(--text); }
  header { display: flex; align-items: baseline; gap: 1.4rem; margin-bottom: 1.4rem; }
  h1 { margin: 0; font-size: 3rem; font-weight: 800; letter-spacing: -0.02em; }
  .meta { color: var(--muted); font-size: 1.3rem; }
  .mode { margin-left: auto; }
  .cols { flex: 1; display: flex; gap: 2.4rem; min-height: 0; }
  aside { width: 26rem; flex: none; overflow: hidden auto; scrollbar-width: none; }
  .g { display: flex; flex-direction: column; align-items: flex-start; width: 100%; margin: 0.15rem 0; padding: 0.7rem 1.1rem; border: 0; border-radius: 0.9rem; background: transparent; color: var(--muted); font: inherit; font-size: 1.25rem; text-align: left; transition: background 0.2s, transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .g small { font-size: 0.9rem; opacity: 0.7; }
  .g.act { font-style: italic; }
  .g.sel { background: var(--glass-strong); color: var(--text); }
  aside.active .g.sel { background: var(--accent); color: #fff; font-weight: 600; transform: scale(1.02); }
  section { flex: 1; position: relative; overflow: hidden; min-width: 0; }
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

  /* guide */
  .guide { display: flex; flex-direction: column; height: 100%; }
  .grow { display: flex; height: 4.4rem; flex: none; margin-bottom: 0.2rem; }
  .grow.head { height: 2.4rem; color: var(--muted); font-size: 1rem; }
  .grow .lab { width: 15rem; flex: none; display: flex; align-items: center; gap: 0.8rem; padding-right: 1rem; min-width: 0; }
  .grow .lab b { font-size: 1.1rem; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .grow .logo { width: 2.8rem; height: 2.8rem; }
  .track { position: relative; flex: 1; min-width: 0; overflow: hidden; }
  .tick { position: absolute; top: 0.4rem; transform: translateX(0.3rem); }
  .nowline { position: absolute; top: 0; bottom: 0; width: 0.18rem; background: #ff3b5c; z-index: 3; pointer-events: none; }
  .blk { position: absolute; top: 0; bottom: 0; display: flex; align-items: center; gap: 0.4rem; padding: 0 0.8rem; overflow: hidden; border: 0; border-radius: 0.7rem; background: var(--glass); color: var(--text); font: inherit; font-size: 1.05rem; text-align: left; transition: background 0.15s; }
  .blk .t { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .blk.past { opacity: 0.55; }
  .blk.onair { background: var(--glass-strong); }
  .blk.sel { background: var(--accent); color: #fff; opacity: 1; font-weight: 600; z-index: 2; }
  .reddot { flex: none; width: 0.8rem; height: 0.8rem; border-radius: 50%; background: #ff3b3b; box-shadow: 0 0 0 0.12rem rgba(255, 255, 255, 0.7); }
  .cta { max-width: 56rem; padding: 1rem 0; }
  .cta h2 { margin: 0 0 0.8rem; font-size: 2rem; }
  .cta p { margin: 0.4rem 0; font-size: 1.3rem; line-height: 1.5; color: var(--muted); }
  .cta .dim { font-size: 1.05rem; }
  .ctab { margin-top: 1rem; padding: 0.9rem 1.6rem; border: 0; border-radius: 1rem; background: var(--glass-strong); color: var(--text); font: inherit; font-size: 1.3rem; }
  .ctab.sel { background: var(--accent); color: #fff; font-weight: 600; }
  .details .pic { float: right; max-height: 7rem; max-width: 12rem; margin-left: 1rem; border-radius: 0.6rem; object-fit: cover; }
  .noepg { position: absolute; inset: 0; display: flex; align-items: center; padding-left: 0.8rem; color: var(--muted); font-size: 1rem; border-radius: 0.7rem; background: rgba(255, 255, 255, 0.04); }
  .details { margin-top: 0.8rem; padding: 0.9rem 1.2rem; border-radius: 1rem; background: var(--glass); flex: 1; min-height: 0; overflow: hidden; }
  .details h2 { margin: 0; font-size: 1.5rem; }
  .details .when { margin: 0.2rem 0 0.4rem; color: var(--muted); font-size: 1.05rem; }
  .details .desc { margin: 0; font-size: 1.1rem; line-height: 1.4; opacity: 0.9; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }

  /* enregistrements */
  .free { margin: 0 0 0.6rem; color: var(--muted); font-size: 1rem; }
  .reclist { display: flex; flex-direction: column; gap: 0.35rem; }
  .rec { display: flex; align-items: center; gap: 1.2rem; height: 4.2rem; padding: 0 1.2rem; border: 0; border-radius: 1rem; background: var(--glass); color: var(--text); font: inherit; text-align: left; }
  .rec.cur { background: var(--glass-strong); }
  .rec.sel { background: var(--accent); color: #fff; }
  .dot { flex: none; width: 0.9rem; height: 0.9rem; border-radius: 50%; background: #888; }
  .dot.scheduled { background: #ffb020; }
  .dot.recording { background: #ff3b3b; }
  .dot.done { background: #2fc36b; }
  .dot.failed { background: #ff7a45; }

  /* menu d'actions */
  .menuwrap { position: absolute; inset: 0; z-index: 30; display: grid; place-items: center; background: rgba(0, 0, 0, 0.55); }
  .menu { min-width: 34rem; max-width: 52rem; padding: 1.4rem; border-radius: 1.4rem; background: var(--bg); border: 0.1rem solid var(--glass-strong); }
  .menu h3 { margin: 0 0 0.9rem; font-size: 1.5rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .menu button { display: block; width: 100%; padding: 0.8rem 1.1rem; margin: 0.15rem 0; border: 0; border-radius: 0.8rem; background: transparent; color: var(--text); font: inherit; font-size: 1.25rem; text-align: left; }
  .menu button.sel { background: var(--accent); color: #fff; font-weight: 600; }
</style>
