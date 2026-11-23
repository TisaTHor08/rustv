<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import { api } from './api';
  import { t } from './i18n';
  import { lib, scanLibrary } from './library.svelte';
  import { enableEpg, guideTarget, hideBroken, loadEpg, safeImg, whenLabel, type EpgHit } from './epg.svelte';
  import { addReminder, dvrError, live, loadLive, recordNow, scheduleProg, startReminders } from './live.svelte';
  import { playMedia } from './player.svelte';
  import { prefs } from './prefs.svelte';
  import './settings/usearch.strings';
  import { play } from './sounds';
  import { iconSrc, type DTile } from './tiles.svelte';
  import { usearch, type UFile, type ULib, type UPartial, type UProvider, type UResult, type UService } from './usearch';

  let {
    tiles,
    onclose,
    onopen,
    onmedia,
    note,
  }: { tiles: DTile[]; onclose: () => void; onopen: (t: DTile) => void; onmedia: (tab: 'movies' | 'series' | 'photos' | 'music') => void; note: (m: string) => void } = $props();

  type Section = 'where' | 'libs' | 'tv' | 'apps' | 'channels' | 'inapp';
  interface Chip {
    label: string;
    sub?: string;
    enabled: boolean;
    run: () => void;
  }
  interface Row {
    section: Section;
    title: string;
    sub?: string;
    badge?: string;
    color?: string;
    icon?: string;
    poster?: string;
    img?: string;
    chips?: Chip[];
    run?: () => void;
  }

  let q = $state('');
  let ri = $state(0);
  let ci = $state(0);
  let input = $state<HTMLInputElement | null>(null);
  let res = $state<UResult | null>(null);
  let partial = $state<UPartial | null>(null);
  let loading = $state(false);
  let seq = 0; // numéro de la dernière recherche envoyée (non réactif : sert à ignorer les réponses périmées)
  let timer: ReturnType<typeof setTimeout> | undefined;
  let unlisten: (() => void) | undefined;
  let bump = $state(0); // incremente pour relancer la recherche (guide TV active)
  let menu = $state<{ title: string; items: { label: string; run: () => void | Promise<void> }[]; i: number } | null>(null);
  let libOn = $state(true); // source « médiathèque locale » activée dans les réglages

  onMount(() => {
    tick().then(() => input?.focus());
    // Les chaînes et la médiathèque sont chargées en arrière-plan pour pouvoir être cherchées.
    if (prefs.live.sources.length) loadLive().catch(() => {});
    scanLibrary().catch(() => {});
    loadEpg().catch(() => {});
    usearch.getConfig().then((c) => (libOn = c.sources.library)).catch(() => {});
    usearch.scan(false).catch(() => {}); // met à jour l'index des dossiers si besoin (au plus une fois par heure)
    usearch.onPartial((p) => {
      if (p.seq === seq) partial = p;
    }).then((f) => (unlisten = f)).catch(() => {});
  });
  onDestroy(() => {
    clearTimeout(timer);
    seq++;
    unlisten?.();
  });

  // Saisie → recherche distante après 350 ms sans frappe ; une nouvelle saisie rend la précédente caduque.
  $effect(() => {
    const s = q.trim();
    void bump;
    clearTimeout(timer);
    ri = 0;
    ci = 0;
    res = null;
    partial = null;
    seq++;
    if (s.length < 2) {
      loading = false;
      return;
    }
    loading = true;
    const mine = seq;
    timer = setTimeout(() => {
      usearch
        .query(s, { seq: mine, lang: prefs.lang === 'en' ? 'en' : 'fr' })
        .then((r) => {
          if (mine === seq) {
            res = r;
            loading = false;
          }
        })
        .catch((e) => {
          if (mine === seq) {
            loading = false;
            if (String(e) !== 'cancelled') res = null;
          }
        });
    }, 350);
  });

  // ---------- texte : insensible à la casse, aux accents et à la ponctuation ----------
  const fold = (s: string) =>
    s
      .normalize('NFD')
      .replace(/\p{M}/gu, '')
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]+/gu, ' ')
      .trim();

  // ---------- lancement ----------
  const tileOf = (id?: string) => (id ? tiles.find((x) => x.id === id) : undefined);

  async function launch(name: string, tileId: string | undefined, url?: string | null) {
    const tile = tileOf(tileId);
    if (!tile || !tileId) {
      play('error');
      note(t('usearch.err.hidden', { name }));
      return;
    }
    try {
      const webTile = tile.source === 'catalog' || tile.source === 'builtin';
      const r = await usearch.launch(tile.id, url ?? undefined, webTile ? undefined : $state.snapshot(tile.raw));
      if (r === 'ok') {
        play('select');
        onclose();
      } else if (r === 'needs_url') {
        play('error');
        note(t('usearch.err.needs_url', { name }));
      }
    } catch (e) {
      play('error');
      note(t('usearch.err.launch', { name }));
    }
  }

  function provChip(p: UProvider | UService, kindLabel?: string): Chip {
    const tile = tileOf(p.tileId);
    const mapped = 'mapped' in p ? p.mapped : true;
    return { label: p.name, sub: !mapped || !tile ? t('usearch.notin') : kindLabel, enabled: !!tile && mapped, run: () => launch(p.name, p.tileId, p.url) };
  }

  function openFile(f: UFile) {
    onclose();
    if (f.kind === 'photo') api.openPath(f.path).catch(() => note(t('usearch.err.play')));
    else playMedia(f.kind === 'audio' ? 'audio' : 'video', [{ name: f.title, url: f.path }], 0, note);
  }

  const kindName = (k: string) => {
    const key = `usearch.kind.${k}`;
    const v = t(key);
    return v === key ? '' : v;
  };

  // ---------- lignes ----------
  const view = $derived<{ titles: UResult['titles']; library: ULib[]; files: UFile[]; services: UService[]; programmes: EpgHit[]; epg: UResult['epg'] } | null>(
    res ? res : partial ? { titles: [], library: partial.library, files: partial.files, services: partial.services, programmes: partial.programmes ?? [], epg: partial.epg ?? 'off' } : null,
  );

  const rows = $derived.by<Row[]>(() => {
    const s = fold(q);
    if (!s) return [];
    const out: Row[] = [];

    // 1) Où regarder (TMDB / JustWatch)
    for (const ti of view?.titles ?? []) {
      const chips = ti.providers.map((p) => provChip(p, t(`usearch.kind.${p.kind}`))).sort((a, b) => Number(b.enabled) - Number(a.enabled));
      out.push({
        section: 'where',
        title: ti.title,
        sub: [ti.year ?? '', kindName(ti.kind)].filter(Boolean).join(' · ') + (chips.length ? '' : ` · ${t('usearch.noprov', { cc: res?.country ?? '' })}`),
        poster: ti.poster ?? undefined,
        chips: chips.length ? chips : undefined,
        run: undefined,
      });
    }

    // 2) Vos médiathèques : serveurs, fichiers des dossiers, médiathèque locale
    const libPaths = new Set<string>();
    const d = lib.data;
        const libs: Row[] = [];
    // La ligne « Ouvrir UGREEN Theater » n'est pas un vrai résultat : elle passe après les autres groupes.
    const ugOpen: Row[] = [];
    for (const l of view?.library ?? []) {
      (l.source === 'ugreen' ? ugOpen : libs).push({
        section: 'libs',
        title: l.title,
        sub: l.source === 'ugreen' ? t('usearch.ugreen', { q: q.trim() }) : [l.parent, l.year, kindName(l.kind)].filter(Boolean).join(' · '),
        badge: l.source === 'plex' ? 'Plex' : l.source === 'jellyfin' ? 'Jellyfin' : 'UGREEN',
        run: () => launch(l.title, l.tileId, l.url),
      });
    }
    if (d && libOn) {
      for (const m of d.movies.filter((m: any) => fold(m.title).includes(s)).slice(0, 10)) {
        libPaths.add(m.path);
        libs.push({ section: 'libs', title: m.title, sub: t('media.movies'), badge: t('search.library'), icon: m.poster, run: () => (onclose(), playMedia('video', [{ name: m.title, url: m.path }], 0, note)) });
      }
      for (const sh of d.series.filter((x: any) => fold(x.show).includes(s)).slice(0, 6)) {
        for (const se of sh.seasons) for (const ep of se.episodes) libPaths.add(ep.path);
        libs.push({ section: 'libs', title: sh.show, sub: t('media.series'), badge: t('search.library'), icon: sh.poster, run: () => (onclose(), onmedia('series')) });
      }
      for (const a of d.music.filter((x: any) => fold(x.artist).includes(s) || x.albums.some((al: any) => fold(al.album).includes(s))).slice(0, 6)) {
        for (const al of a.albums) for (const tr of al.tracks) libPaths.add(tr.path);
        libs.push({ section: 'libs', title: a.artist, sub: t('media.music'), badge: t('search.library'), run: () => (onclose(), onmedia('music')) });
      }
      for (const p of d.photos.filter((x: any) => fold(x.album).includes(s)).slice(0, 6)) {
        libs.push({ section: 'libs', title: p.album, sub: t('media.photos'), badge: t('search.library'), icon: p.cover, run: () => (onclose(), onmedia('photos')) });
      }
    }
    for (const f of (view?.files ?? []).filter((f) => !libPaths.has(f.path)).slice(0, 12)) {
      libs.push({ section: 'libs', title: f.title, sub: [f.year, kindName(f.kind)].filter(Boolean).join(' · ') + (f.sub ? `  ${f.sub}` : ''), badge: t('usearch.src.files').split(' ')[0], run: () => openFile(f) });
    }
    out.push(...libs);

    // 2b) A la TV : programmes du guide (cache local), ou invitation a activer le guide
    for (const h of view?.programmes ?? []) {
      out.push({
        section: 'tv',
        title: h.title,
        sub: [whenLabel(h.start, h.stop), h.category].filter(Boolean).join(' \u00b7 '),
        badge: h.channelName,
        img: safeImg(h.channelIcon) || undefined,
        run: () => (h.now && h.playable ? playHit(h) : openHitMenu(h)),
      });
    }
    if (view && view.epg === 'off' && q.trim().length >= 2) {
      out.push({ section: 'tv', title: t('epg.cta.title'), sub: t('epg.cta.sub'), run: () => enableEpg(note).then((ok) => ok && bump++) });
    }

    // 3) Applications
    for (const x of tiles) {
      if (fold(x.name).includes(s)) out.push({ section: 'apps', title: x.name, sub: x.desc, color: x.color, icon: x.icon, run: () => (onclose(), onopen(x)) });
    }

    // 4) Chaînes
    for (const c of live.channels.filter((c) => fold(c.name).includes(s)).slice(0, 20)) {
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

    // 5) Chercher dans une application (services dont la tuile est dans RusTV)
    const chips = (view?.services ?? []).filter((x) => tileOf(x.tileId)).map((x) => provChip(x));
    out.push(...ugOpen);
    if (q.trim().length >= 2 && chips.length) out.push({ section: 'inapp', title: t('usearch.inapp'), sub: `« ${q.trim()} »`, chips });
    return out;
  });

  // Position valide même quand la liste change sous le curseur.
  const row = $derived(rows[Math.min(ri, Math.max(0, rows.length - 1))]);
  const sel = $derived(Math.min(ri, Math.max(0, rows.length - 1)));
  const chipIdx = (r: Row | undefined) => {
    const cs = r?.chips ?? [];
    if (cs[ci]?.enabled) return ci;
    const first = cs.findIndex((c) => c.enabled);
    return first === -1 ? 0 : first;
  };
  const selChip = $derived(chipIdx(row));

  const label = (s: Section) => t(s === 'where' ? 'usearch.where' : s === 'libs' ? 'usearch.libs' : s === 'tv' ? 'epg.search.group' : s === 'apps' ? 'search.apps' : s === 'channels' ? 'search.channels' : 'usearch.inapp');

  const hasTitles = $derived(!!res && res.titles.length > 0);
  const nokey = $derived(res?.tmdb === 'nokey' && q.trim().length >= 2);
  const errLine = $derived.by(() => {
    const e = res?.errors;
    if (!e) return '';
    const k = Object.values(e)[0];
    return k ? t(`usearch.err.${k}`) : '';
  });

  // ---------- programmes TV ----------
  function playHit(h: EpgHit) {
    const list = live.channels;
    const i = list.findIndex((x) => x.id === h.liveChannelId);
    if (i < 0) return note(t('epg.noplay'));
    onclose();
    playMedia('live', list.map((x) => ({ name: x.name, url: x.url })), i, note);
  }

  async function recordHit(h: EpgHit) {
    const c = live.channels.find((x) => x.id === h.liveChannelId);
    if (!c) return note(t('epg.noplay'));
    try {
      if (h.now) await recordNow(c, Math.max(1, Math.min(720, Math.ceil((h.stop - Date.now() / 1000) / 60) + 1)));
      else await scheduleProg(c, h);
      note(t('lv.rec.ok'));
    } catch (e) {
      note(dvrError(e));
    }
  }

  function showInGuide(h: EpgHit) {
    const tile = tiles.find((x) => x.id === 'live');
    if (!tile) return note(t('epg.noguidetile'));
    guideTarget.v = { channelId: h.channelId, liveChannelId: h.liveChannelId, start: h.start };
    onclose();
    onopen(tile);
  }

  function openHitMenu(h: EpgHit) {
    const items: { label: string; run: () => void | Promise<void> }[] = [];
    const nowTs = Date.now() / 1000;
    if (h.playable) items.push({ label: t('epg.menu.watch'), run: () => playHit(h) });
    if (h.start > nowTs) items.push({ label: t('epg.menu.remind'), run: () => (startReminders(note), addReminder({ channelName: h.channelName, title: h.title, start: h.start }), note(t('lv.remind.ok'))) });
    if (h.playable && h.stop > nowTs) items.push({ label: t('epg.menu.record'), run: () => recordHit(h) });
    items.push({ label: t('epg.menu.guide'), run: () => showInGuide(h) });
    items.push({ label: t('epg.menu.close'), run: () => {} });
    menu = { title: `${h.title} \u00b7 ${h.channelName}`, items, i: 0 };
  }

  function moveChip(dir: -1 | 1) {
    const cs = row?.chips;
    if (!cs) return false;
    let i = selChip;
    do i += dir;
    while (i >= 0 && i < cs.length && !cs[i].enabled);
    if (i >= 0 && i < cs.length) {
      ci = i;
      play('move');
    }
    return true;
  }

  function onKey(e: KeyboardEvent) {
    const stop = () => (e.preventDefault(), e.stopImmediatePropagation());
    if (menu) {
      stop();
      if (e.key === 'ArrowDown') menu.i = Math.min(menu.items.length - 1, menu.i + 1);
      else if (e.key === 'ArrowUp') menu.i = Math.max(0, menu.i - 1);
      else if (e.key === 'Enter') {
        const it = menu.items[menu.i];
        menu = null;
        it?.run();
      } else if (e.key === 'Escape' || e.key === 'Backspace' || e.key === 'ContextMenu') menu = null;
      return;
    }
    if (e.key === 'Escape') {
      stop();
      play('back');
      onclose();
    } else if (e.key === 'ArrowDown') {
      stop();
      if (sel < rows.length - 1) {
        ri = sel + 1;
        ci = 0;
        play('move');
      }
    } else if (e.key === 'ArrowUp') {
      stop();
      if (sel > 0) {
        ri = sel - 1;
        ci = 0;
        play('move');
      }
    } else if ((e.key === 'ArrowLeft' || e.key === 'ArrowRight') && row?.chips) {
      stop();
      moveChip(e.key === 'ArrowLeft' ? -1 : 1);
    } else if (e.key === 'Enter') {
      stop();
      const r = row;
      if (!r) return;
      if (r.chips) {
        const c = r.chips[selChip];
        if (c?.enabled) c.run();
        else {
          play('error');
          note(t('usearch.err.hidden', { name: c?.label ?? r.title }));
        }
      } else {
        play('select');
        r.run?.();
      }
    } else e.stopImmediatePropagation(); // le reste va au champ de saisie
    tick().then(() => document.querySelector('.row.sel')?.scrollIntoView({ block: 'nearest' }));
  }
</script>

<svelte:window onkeydowncapture={onKey} />

<div class="scrim" transition:fade={{ duration: 160 }}>
  <div class="sheet" in:fly={{ y: 30, duration: 320, easing: cubicOut }}>
    <input bind:this={input} bind:value={q} placeholder={t('search.placeholder')} autocomplete="off" spellcheck="false" />
    <div class="list">
      {#if loading}<p class="note">{t('usearch.loading')}</p>{/if}
      {#if q.trim() && !rows.length && !loading}<p class="none">{t('search.none')}</p>{/if}
      {#each rows as h, i}
        {#if i === 0 || rows[i - 1].section !== h.section}<h4>{label(h.section)}</h4>{/if}
        <div class="row" class:sel={i === sel} class:wide={!!h.chips} onmouseenter={() => (ri = i)} role="presentation">
          <button class="hit" onclick={() => (h.chips ? h.chips.find((c) => c.enabled)?.run() : h.run?.())}>
            <span class="ic" class:poster={!!h.poster} style:background={h.color ?? 'var(--glass-strong)'}>
              {#if h.poster}<img src={h.poster} alt="" />{:else if h.img}<img src={h.img} alt="" onerror={hideBroken} />{:else if h.icon}<img src={iconSrc(h.icon)} alt="" />{:else}{h.title[0]}{/if}
            </span>
            <span class="tx"><b>{h.title}</b>{#if h.sub}<small>{h.sub}</small>{/if}</span>
            {#if h.badge}<span class="badge">{h.badge}</span>{/if}
          </button>
          {#if h.chips}
            <div class="chips">
              {#each h.chips as c, k}
                <button class="chip" class:off={!c.enabled} class:csel={i === sel && k === selChip} onclick={() => (c.enabled ? c.run() : note(t('usearch.err.hidden', { name: c.label })))}>
                  <span>{c.label}</span>{#if c.sub}<small>{c.sub}</small>{/if}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
      {#if nokey}<p class="note">{t('usearch.nokey')}</p>{/if}
      {#if errLine}<p class="note">{errLine}</p>{/if}
      {#if hasTitles}<p class="attr"><b>TMDB</b> · <b>JustWatch</b> — {t('usearch.attr')}</p>{/if}
    </div>
    <p class="hint">{t('search.hint')}</p>
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
</div>

<style>
  .scrim { position: absolute; inset: 0; background: rgba(4, 6, 10, 0.8); backdrop-filter: blur(1.6rem); display: grid; place-items: start center; padding-top: 5rem; z-index: 30; }
  .sheet { width: 72rem; max-width: 94%; padding: 2rem 2.4rem; border-radius: 1.8rem; background: var(--sheet); box-shadow: 0 3rem 6rem rgba(0, 0, 0, 0.6), inset 0 0 0 0.08rem var(--glass-strong); color: var(--text); }
  input { width: 100%; padding: 1.1rem 1.4rem; border-radius: 1rem; border: 0.12rem solid var(--glass-strong); background: var(--field); color: var(--text); font-size: 1.7rem; outline: none; box-sizing: border-box; }
  input:focus { border-color: var(--accent); }
  .list { margin-top: 1rem; max-height: 40rem; overflow: hidden auto; scrollbar-width: none; }
  h4 { margin: 1rem 0 0.3rem; font-size: 0.95rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.12em; }
  .row { border-radius: 0.9rem; transition: background 0.15s; }
  .row.wide.sel { background: var(--glass); }
  .hit { display: flex; align-items: center; gap: 1.1rem; width: 100%; padding: 0.6rem 0.9rem; border: 0; border-radius: 0.9rem; background: transparent; color: var(--text); font: inherit; text-align: left; transition: background 0.15s, transform 0.2s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .row.sel:not(.wide) .hit { background: var(--accent); color: #fff; transform: scale(1.015); }
  .ic { flex: none; width: 3.2rem; height: 3.2rem; border-radius: 0.8rem; display: grid; place-items: center; font-weight: 800; font-size: 1.3rem; color: #fff; overflow: hidden; }
  .ic.poster { width: 4rem; height: 5.8rem; border-radius: 0.5rem; }
  .ic img { width: 100%; height: 100%; object-fit: cover; }
  .tx { display: flex; flex-direction: column; min-width: 0; flex: 1; }
  .tx b { font-size: 1.3rem; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tx small { font-size: 0.95rem; opacity: 0.7; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .badge { flex: none; font-size: 0.8rem; font-weight: 700; padding: 0.15rem 0.6rem; border-radius: 0.6rem; background: var(--glass-strong); letter-spacing: 0.05em; text-transform: uppercase; }
  .row.sel:not(.wide) .badge { background: rgba(255, 255, 255, 0.25); }
  .chips { display: flex; flex-wrap: wrap; gap: 0.5rem; padding: 0 0.9rem 0.7rem 5.3rem; }
  .chip { display: inline-flex; flex-direction: column; align-items: flex-start; padding: 0.35rem 0.9rem; border-radius: 0.8rem; border: 0.12rem solid var(--glass-strong); background: var(--field); color: var(--text); font: inherit; font-size: 1.05rem; line-height: 1.15; transition: background 0.15s, transform 0.2s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .chip small { font-size: 0.78rem; opacity: 0.7; }
  .chip.off { opacity: 0.4; }
  .chip.csel { background: var(--accent); border-color: var(--accent); color: #fff; transform: scale(1.06); }
  .none { color: var(--muted); font-size: 1.3rem; padding: 1rem 0.4rem; }
  .note { color: var(--muted); font-size: 0.95rem; padding: 0.5rem 0.4rem 0; margin: 0; }
  .attr { color: var(--muted); font-size: 0.78rem; padding: 0.8rem 0.4rem 0; margin: 0; opacity: 0.8; }
  .sheet { position: relative; }
  .menuwrap { position: absolute; inset: 0; z-index: 5; display: grid; place-items: center; border-radius: 1.8rem; background: rgba(0, 0, 0, 0.55); }
  .menu { min-width: 34rem; max-width: 90%; padding: 1.4rem; border-radius: 1.4rem; background: var(--bg); border: 0.1rem solid var(--glass-strong); }
  .menu h3 { margin: 0 0 0.9rem; font-size: 1.4rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .menu button { display: block; width: 100%; padding: 0.8rem 1.1rem; margin: 0.15rem 0; border: 0; border-radius: 0.8rem; background: transparent; color: var(--text); font: inherit; font-size: 1.25rem; text-align: left; }
  .menu button.sel { background: var(--accent); color: #fff; font-weight: 600; }
  .hint { margin: 1rem 0 0; font-size: 0.95rem; color: var(--muted); }
</style>
