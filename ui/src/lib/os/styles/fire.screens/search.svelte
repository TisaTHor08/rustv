<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import { play } from '../../../sounds';
  import type { SearchScreenProps } from '../../screens';

  // Recherche style Fire TV : champ en pilule claire en haut, clavier à l'écran à gauche (touches sombres), résultats en cartes 16:9 à droite.
  // Le champ est le vrai champ de saisie : le clavier du téléphone fonctionne aussi.
  let { query, setQuery, rows, index, loading, sectionLabel, notes, placeholder, menu, bindInput, hoverRow, activateRow, setKeyboardHandler }: SearchScreenProps = $props();

  const KEYS = [...'abcdefghijklmnopqrstuvwxyz0123456789'];
  const COLS = 6;
  const ACTIONS = ['space', 'delete', 'clear'] as const;
  const RCOLS = 3;
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
      else if (dir === 'ArrowRight') {
        if (a < ACTIONS.length - 1) k++;
        else if (rows.length) zone = 'res';
      } else if (dir === 'ArrowUp') k = (rowsN - 1) * COLS + a * 2;
      return true;
    }
    const r = Math.floor(k / COLS);
    const c = k % COLS;
    if (dir === 'ArrowLeft') k = c > 0 ? k - 1 : k;
    else if (dir === 'ArrowRight') {
      if (c < COLS - 1) k++;
      else if (rows.length) zone = 'res';
    } else if (dir === 'ArrowUp') k = r > 0 ? k - COLS : k;
    else if (dir === 'ArrowDown') k = r < rowsN - 1 ? k + COLS : KEYS.length + Math.floor(c / 2);
    return true;
  }
  function moveRes(dir: string): boolean {
    const n = rows.length;
    if (!n) return (zone = 'kb'), true;
    let i = index;
    if (dir === 'ArrowLeft') {
      if (i % RCOLS === 0) return (zone = 'kb'), true;
      i--;
    } else if (dir === 'ArrowRight') i = i % RCOLS < RCOLS - 1 && i + 1 < n ? i + 1 : i;
    else if (dir === 'ArrowUp') i = i >= RCOLS ? i - RCOLS : i;
    else if (dir === 'ArrowDown') i = i + RCOLS < n ? i + RCOLS : i;
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
  const label = (a: (typeof ACTIONS)[number]) => t(`fire.kb.${a}`);
</script>

<div class="fsr" transition:fade={{ duration: 160 }}>
  <div class="field">
    <svg viewBox="0 0 24 24"><path d="M10.5 3.5a7 7 0 1 0 0 14 7 7 0 0 0 0-14Zm5.2 12.2L21 21" /></svg>
    <input bind:this={input} value={query} oninput={(e) => setQuery(e.currentTarget.value)} {placeholder} autocomplete="off" spellcheck="false" />
  </div>
  <aside>
    <div class="kb">
      {#each KEYS as key, i}
        <button class="key" class:on={zone === 'kb' && k === i} onmouseenter={() => ((zone = 'kb'), (k = i))} onclick={() => press(i)}>{key}</button>
      {/each}
      {#each ACTIONS as a, j}
        <button class="key act" class:on={zone === 'kb' && k === KEYS.length + j} onmouseenter={() => ((zone = 'kb'), (k = KEYS.length + j))} onclick={() => press(KEYS.length + j)}>{label(a)}</button>
      {/each}
    </div>
  </aside>
  <main bind:this={grid}>
    {#if loading}<p class="note">{t('usearch.loading')}</p>{/if}
    {#if !rows.length && !loading}<p class="note">{query.trim() ? t('search.none') : t('fire.search.start')}</p>{/if}
    <div class="grid">
      {#each rows as r, i}
        <button class="card" class:on={zone === 'res' && i === index} onmouseenter={() => ((zone = 'res'), hoverRow(i))} onclick={() => activateRow(i)}>
          <span class="art" class:poster={r.poster} style:--a={r.color ?? '#3a3a3a'}>
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
  .fsr { position: absolute; inset: 0; z-index: 30; color: #fff; font-family: var(--font); background: #171717; }
  .field { position: absolute; left: 5.2rem; top: 3rem; width: 42rem; height: 4rem; display: flex; align-items: center; gap: 1rem; padding: 0 1.4rem; box-sizing: border-box; border-radius: 2rem; background: #e6e6e6; color: #1a1a1a; }
  .field svg { width: 2rem; height: 2rem; flex: none; fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; }
  input { flex: 1; min-width: 0; border: 0; background: transparent; color: inherit; font: inherit; font-size: 1.6rem; outline: 0; }
  input::placeholder { color: #6b6b6b; }
  aside { position: absolute; left: 5.2rem; top: 9rem; width: 42rem; }
  .kb { display: grid; grid-template-columns: repeat(6, 1fr); gap: 0.4rem; }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; }
  .key { height: 4.6rem; border-radius: var(--r); font-size: 1.9rem; font-weight: 500; text-transform: uppercase; background: #2c2c2c; transition: transform 0.12s; }
  .key.act { grid-column: span 2; font-size: 1.2rem; text-transform: none; }
  .key.on { background: #3a3a3a; box-shadow: 0 0 0 0.2rem #fff; transform: scale(1.06); position: relative; z-index: 2; }
  main { position: absolute; left: 52rem; right: 5rem; top: 3rem; bottom: 3rem; overflow: hidden auto; scrollbar-width: none; padding: 0.8rem; }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1.4rem 0.6rem; }
  .card { text-align: left; padding: 0; transition: transform 0.18s var(--ease); }
  .art { position: relative; display: grid; place-items: center; aspect-ratio: 16 / 9; border-radius: var(--r); overflow: hidden; background: linear-gradient(135deg, var(--a), #1d1d1d); }
  .art b { font-size: 3.4rem; font-weight: 700; }
  .art img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .art.poster img { object-fit: contain; background: #101010; }
  .art em { position: absolute; left: 0.5rem; bottom: 0.5rem; padding: 0.1rem 0.6rem; border-radius: 0.2rem; background: rgba(0, 0, 0, 0.65); font-size: 0.85rem; font-style: normal; font-weight: 700; }
  .nm { display: block; margin-top: 0.5rem; font-size: 1.2rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sb { display: block; font-size: 0.95rem; color: rgba(255, 255, 255, 0.6); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card.on { transform: scale(1.05); position: relative; z-index: 2; }
  .card.on .art { box-shadow: 0 0 0 0.2rem #fff, 0 0.8rem 2rem rgba(0, 0, 0, 0.6); }
  .note { margin: 0.6rem 0; font-size: 1.25rem; color: rgba(255, 255, 255, 0.65); }
  .menuwrap { position: absolute; inset: 0; display: grid; place-items: center; background: rgba(0, 0, 0, 0.65); }
  .menu { min-width: 36rem; padding: 1.4rem; border-radius: var(--r); background: #262626; }
  .menu h4 { margin: 0 0 0.8rem 0.4rem; font-size: 1.3rem; font-weight: 500; }
  .mi { padding: 0.8rem 1.2rem; border-radius: var(--r); font-size: 1.3rem; }
  .mi.on { background: #3a3a3a; box-shadow: 0 0 0 0.18rem #fff; }
</style>
