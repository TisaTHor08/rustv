<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import { play } from '../../../sounds';
  import type { SearchScreenProps } from '../../screens';

  // Recherche style tvOS : champ centré, clavier de touches rondes en grille compacte (la touche ciblée devient blanche et grossit),
  // résultats en rangée de cartes dessous. Le champ est le vrai champ de saisie : le clavier du téléphone fonctionne aussi.
  let { query, setQuery, rows, index, loading, sectionLabel, notes, placeholder, menu, bindInput, hoverRow, activateRow, setKeyboardHandler }: SearchScreenProps = $props();

  const KEYS = [...'abcdefghijklmnopqrstuvwxyz0123456789'];
  const COLS = 12;
  const ACTIONS = ['space', 'delete', 'clear'] as const;
  const RCOLS = 5;
  let zone = $state<'kb' | 'res'>('kb');
  let k = $state(0);
  let input: HTMLInputElement | undefined = $state();
  let grid: HTMLElement | undefined = $state();
  onMount(() => input && bindInput(input));

  function press(i: number) {
    if (i < KEYS.length) setQuery(query + KEYS[i]);
    else if (ACTIONS[i - KEYS.length] === 'space') setQuery(query.endsWith(' ') || !query ? query : query + ' ');
    else if (ACTIONS[i - KEYS.length] === 'delete') setQuery(query.slice(0, -1));
    else setQuery('');
    play('select');
  }
  function moveKb(dir: string): boolean {
    const rowsN = KEYS.length / COLS;
    if (k >= KEYS.length) {
      const a = k - KEYS.length;
      if (dir === 'ArrowLeft') k = a > 0 ? k - 1 : k;
      else if (dir === 'ArrowRight') k = a < ACTIONS.length - 1 ? k + 1 : k;
      else if (dir === 'ArrowUp') k = (rowsN - 1) * COLS + a * 4 + 2;
      else if (dir === 'ArrowDown' && rows.length) zone = 'res';
      return true;
    }
    const r = Math.floor(k / COLS);
    const c = k % COLS;
    if (dir === 'ArrowLeft') k = c > 0 ? k - 1 : k;
    else if (dir === 'ArrowRight') k = c < COLS - 1 ? k + 1 : k;
    else if (dir === 'ArrowUp') k = r > 0 ? k - COLS : k;
    else if (dir === 'ArrowDown') k = r < rowsN - 1 ? k + COLS : KEYS.length + Math.floor(c / 4);
    return true;
  }
  function moveRes(dir: string): boolean {
    const n = rows.length;
    if (!n) return (zone = 'kb'), true;
    let i = index;
    if (dir === 'ArrowLeft') i = i % RCOLS > 0 ? i - 1 : i;
    else if (dir === 'ArrowRight') i = i % RCOLS < RCOLS - 1 && i + 1 < n ? i + 1 : i;
    else if (dir === 'ArrowUp') {
      if (i < RCOLS) return (zone = 'kb'), true;
      i -= RCOLS;
    } else if (dir === 'ArrowDown') i = i + RCOLS < n ? i + RCOLS : i;
    if (i !== index) (hoverRow(i), play('move'));
    return true;
  }
  const handler = (e: KeyboardEvent): boolean => {
    if (menu) return false;
    if (e.key.startsWith('Arrow')) {
      const before = k;
      const r = zone === 'kb' ? moveKb(e.key) : moveRes(e.key);
      if (zone === 'kb' && k !== before) play('move');
      return r;
    }
    if (e.key === 'Enter') {
      if (zone === 'kb') press(k);
      else if (rows.length) activateRow(index);
      return true;
    }
    return false;
  };
  $effect(() => {
    setKeyboardHandler(handler);
    return () => setKeyboardHandler(null);
  });
  $effect(() => {
    void index;
    zone;
    grid?.querySelector('.card.on')?.scrollIntoView({ block: 'nearest' });
  });
  const label = (a: (typeof ACTIONS)[number]) => t(`tvos.kb.${a}`);
</script>

<div class="tsr" transition:fade={{ duration: 200 }}>
  <div class="field">
    <svg viewBox="0 0 24 24"><path d="M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z" /></svg>
    <input bind:this={input} value={query} oninput={(e) => setQuery(e.currentTarget.value)} {placeholder} autocomplete="off" spellcheck="false" />
  </div>
  <div class="kb">
    {#each KEYS as key, i}
      <button class="key" class:on={zone === 'kb' && k === i} onmouseenter={() => ((zone = 'kb'), (k = i))} onclick={() => press(i)}>{key}</button>
    {/each}
    {#each ACTIONS as a, j}
      <button class="key act" class:on={zone === 'kb' && k === KEYS.length + j} onmouseenter={() => ((zone = 'kb'), (k = KEYS.length + j))} onclick={() => press(KEYS.length + j)}>{label(a)}</button>
    {/each}
  </div>
  <main bind:this={grid}>
    {#if loading}<p class="note">{t('usearch.loading')}</p>{/if}
    {#if !rows.length && !loading}<p class="note">{query.trim() ? t('search.none') : t('tvos.search.start')}</p>{/if}
    <div class="grid">
      {#each rows as r, i}
        <button class="card" class:on={zone === 'res' && i === index} onmouseenter={() => ((zone = 'res'), hoverRow(i))} onclick={() => activateRow(i)}>
          <span class="art" class:poster={r.poster} style:--a={r.color ?? '#3a3a4a'}>
            {#if r.image}<img src={r.image} alt="" />{:else}<b>{r.title[0]}</b>{/if}
            {#if r.badge}<em>{r.badge}</em>{/if}
          </span>
          <span class="nm">{r.title}</span>
          <span class="sb">{r.sub ?? sectionLabel(r.section)}</span>
        </button>
      {/each}
    </div>
    {#each notes as n}<p class="note">{n}</p>{/each}
  </main>
  {#if menu}
    <div class="menuwrap"><div class="menu"><h4>{menu.title}</h4>{#each menu.items as it, i}<div class="mi" class:on={i === menu.index}>{it}</div>{/each}</div></div>
  {/if}
</div>

<style>
  .tsr { position: absolute; inset: 0; z-index: 30; color: #fff; font-family: var(--font); background: radial-gradient(90rem 50rem at 50% 0%, #2b2d3f, transparent 70%), #08080a; }
  .field { position: absolute; left: 50%; top: 3rem; transform: translateX(-50%); width: 56rem; height: 5rem; display: flex; align-items: center; gap: 1.2rem; padding: 0 2rem; box-sizing: border-box; border-radius: 99rem; background: rgba(118, 118, 128, 0.3); backdrop-filter: blur(2rem); }
  .field svg { width: 2.2rem; height: 2.2rem; flex: none; fill: var(--muted); }
  input { flex: 1; min-width: 0; border: 0; background: transparent; color: #fff; font: inherit; font-size: 1.9rem; font-weight: 500; outline: 0; }
  input::placeholder { color: var(--muted); }
  .kb { position: absolute; left: 50%; top: 10rem; transform: translateX(-50%); display: grid; grid-template-columns: repeat(12, 4.4rem); gap: 0.6rem; }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; }
  .key { height: 4.4rem; border-radius: 99rem; font-size: 1.8rem; font-weight: 500; text-transform: uppercase; background: rgba(118, 118, 128, 0.24); transition: transform 0.15s var(--ease), background 0.15s; }
  .key.act { grid-column: span 4; font-size: 1.3rem; text-transform: none; letter-spacing: 0.02em; }
  .key.on { background: rgba(255, 255, 255, 0.95); color: #111; transform: scale(1.14); position: relative; z-index: 2; box-shadow: 0 0.8rem 1.8rem rgba(0, 0, 0, 0.45); }
  main { position: absolute; left: 8rem; right: 8rem; top: 31.5rem; bottom: 1.5rem; overflow: hidden auto; scrollbar-width: none; padding: 1.4rem; }
  .grid { display: grid; grid-template-columns: repeat(5, 1fr); gap: 2rem 1.8rem; }
  .card { text-align: left; padding: 0; transition: transform 0.25s var(--ease); }
  .art { position: relative; display: grid; place-items: center; aspect-ratio: 16 / 10; border-radius: var(--r); overflow: hidden; background: linear-gradient(145deg, var(--a), #15151c); box-shadow: 0 0.6rem 1.6rem rgba(0, 0, 0, 0.45); }
  .art b { font-size: 3.4rem; font-weight: 700; }
  .art img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .art.poster img { object-fit: contain; background: #0e0e12; }
  .art em { position: absolute; left: 0.6rem; bottom: 0.6rem; padding: 0.1rem 0.7rem; border-radius: 99rem; background: rgba(0, 0, 0, 0.55); font-size: 0.85rem; font-style: normal; font-weight: 600; }
  .nm { display: block; margin-top: 0.7rem; font-size: 1.2rem; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sb { display: block; font-size: 0.95rem; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card.on { transform: translateY(-0.4rem) scale(1.1); position: relative; z-index: 2; }
  .card.on .art { box-shadow: 0 2rem 3rem -0.4rem rgba(0, 0, 0, 0.7), 0 0 0 0.1rem rgba(255, 255, 255, 0.35); }
  .note { margin: 0.6rem 0; text-align: center; font-size: 1.3rem; color: var(--muted); }
  .menuwrap { position: absolute; inset: 0; display: grid; place-items: center; background: rgba(0, 0, 0, 0.55); backdrop-filter: blur(1.6rem); }
  .menu { min-width: 38rem; padding: 1.6rem; border-radius: 2.4rem; background: rgba(44, 44, 46, 0.92); }
  .menu h4 { margin: 0 0 0.8rem 0.6rem; font-size: 1.4rem; font-weight: 600; }
  .mi { padding: 0.9rem 1.4rem; border-radius: 1.4rem; font-size: 1.4rem; }
  .mi.on { background: rgba(255, 255, 255, 0.95); color: #111; font-weight: 600; }
</style>
