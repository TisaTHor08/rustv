<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import { play } from '../../../sounds';
  import type { SearchScreenProps } from '../../screens';

  // Recherche style Titan : champ en pastille en haut, clavier à l'écran en bandeau (12 colonnes, centré), résultats en rail de
  // cartes 16:9 en dessous (Bas depuis le clavier, Haut pour y revenir). Le champ est le vrai champ de saisie.
  let { query, setQuery, rows, index, loading, sectionLabel, notes, placeholder, menu, bindInput, hoverRow, activateRow, setKeyboardHandler }: SearchScreenProps = $props();

  const KEYS = [...'abcdefghijklmnopqrstuvwxyz0123456789'];
  const COLS = 12;
  const ACTIONS = ['space', 'delete', 'clear'] as const;
  const RCOLS = 4;
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
    const rowsN = KEYS.length / COLS; // 3 rangées, puis les actions
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
  const label = (a: (typeof ACTIONS)[number]) => t(`titan.kb.${a}`);
</script>

<div class="tsr" transition:fade={{ duration: 160 }}>
  <div class="field">
    <svg viewBox="0 0 24 24"><path d="M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z" /></svg>
    <input bind:this={input} value={query} oninput={(e) => setQuery(e.currentTarget.value)} {placeholder} autocomplete="off" spellcheck="false" />
  </div>
  <div class="kbwrap">
    <div class="kb">
      {#each KEYS as key, i}
        <button class="key" class:on={zone === 'kb' && k === i} onmouseenter={() => ((zone = 'kb'), (k = i))} onclick={() => press(i)}>{key}</button>
      {/each}
    </div>
    <div class="acts">
      {#each ACTIONS as a, j}
        <button class="key act" class:on={zone === 'kb' && k === KEYS.length + j} onmouseenter={() => ((zone = 'kb'), (k = KEYS.length + j))} onclick={() => press(KEYS.length + j)}>{label(a)}</button>
      {/each}
    </div>
  </div>
  <main bind:this={grid}>
    {#if loading}<p class="note">{t('usearch.loading')}</p>{/if}
    {#if !rows.length && !loading}<p class="note">{query.trim() ? t('search.none') : t('titan.search.start')}</p>{/if}
    <div class="grid">
      {#each rows as r, i}
        <button class="card" class:on={zone === 'res' && i === index} onmouseenter={() => ((zone = 'res'), hoverRow(i))} onclick={() => activateRow(i)}>
          <span class="art" class:poster={r.poster} style:--a={r.color ?? '#3a3f66'}>
            {#if r.image}<img src={r.image} alt="" />{:else}<b>{r.title[0]}</b>{/if}
            {#if r.badge}<em>{r.badge}</em>{/if}
            <span class="cap">{r.title}</span>
          </span>
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
  .tsr { position: absolute; inset: 0; z-index: 30; color: #fff; font-family: var(--font); background: radial-gradient(60rem 40rem at 80% 0%, rgba(70, 80, 190, 0.35), transparent 70%), #080818; }
  .field { position: absolute; left: 50%; top: 2.4rem; transform: translateX(-50%); width: 56rem; display: flex; align-items: center; gap: 1.2rem; height: 4.8rem; padding: 0 2rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.12); }
  .field svg { flex: none; width: 2.2rem; height: 2.2rem; fill: currentColor; opacity: 0.8; }
  input { flex: 1; min-width: 0; border: 0; background: transparent; color: #fff; font: inherit; font-size: 1.9rem; outline: 0; }
  input::placeholder { color: rgba(255, 255, 255, 0.5); }
  .kbwrap { position: absolute; left: 50%; top: 9.2rem; transform: translateX(-50%); width: 66rem; }
  .kb { display: grid; grid-template-columns: repeat(12, 1fr); gap: 0.4rem; }
  .acts { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.4rem; margin-top: 0.4rem; }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; }
  .key { height: 4rem; border-radius: 0.5rem; background: rgba(255, 255, 255, 0.1); font-size: 1.7rem; text-transform: uppercase; }
  .key.act { font-size: 1.15rem; text-transform: none; }
  .key.on { background: #1d7be5; box-shadow: 0 0 1.4rem rgba(29, 123, 229, 0.7); }
  main { position: absolute; left: 5rem; right: 5rem; top: 26.4rem; bottom: 1.6rem; overflow: hidden auto; scrollbar-width: none; padding: 0.8rem; }
  .grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 1.4rem; }
  .card { text-align: left; padding: 0; transition: transform 0.18s var(--ease); }
  .art { position: relative; display: grid; place-items: center; aspect-ratio: 16 / 9; border-radius: 0.8rem; overflow: hidden; background: linear-gradient(135deg, var(--a), #12142a); }
  .art b { font-size: 3rem; font-weight: 700; }
  .art img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .art.poster img { object-fit: contain; background: #0c0d1f; }
  .art em { position: absolute; right: 0.5rem; top: 0.5rem; padding: 0.1rem 0.6rem; border-radius: 99rem; background: rgba(0, 0, 0, 0.6); font-size: 0.85rem; font-style: normal; }
  .cap { position: absolute; left: 0; right: 0; bottom: 0; padding: 1.4rem 0.8rem 0.5rem; background: linear-gradient(transparent, rgba(0, 0, 0, 0.75)); font-size: 1.05rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sb { display: block; margin-top: 0.3rem; font-size: 0.95rem; opacity: 0.65; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card.on { transform: scale(1.06); position: relative; z-index: 2; }
  .card.on .art { box-shadow: 0 0 0 0.2rem #dfe7ff, 0 0 2rem rgba(120, 140, 255, 0.5); }
  .note { margin: 0.6rem 0; font-size: 1.2rem; opacity: 0.7; }
  .menuwrap { position: absolute; inset: 0; display: grid; place-items: center; background: rgba(2, 3, 10, 0.72); }
  .menu { min-width: 36rem; padding: 1.4rem; border-radius: 0.8rem; background: #141c36; }
  .menu h4 { margin: 0 0 0.8rem 0.4rem; font-size: 1.3rem; }
  .mi { padding: 0.8rem 1.2rem; border-radius: 0.3rem; font-size: 1.3rem; }
  .mi.on { background: #1d7be5; }
</style>
