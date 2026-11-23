<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import { play } from '../../../sounds';
  import type { SearchScreenProps } from '../../screens';

  // Recherche style VIDAA : champ en haut, clavier à l'écran 10 colonnes à gauche (cadre blanc sur la touche), résultats en liste
  // verticale à droite. Le champ est le vrai champ de saisie (le clavier du téléphone fonctionne aussi).
  let { query, setQuery, rows, index, loading, sectionLabel, notes, placeholder, menu, bindInput, hoverRow, activateRow, setKeyboardHandler }: SearchScreenProps = $props();

  const KEYS = [...'1234567890abcdefghijklmnopqrstuvwxyz'];
  const COLS = 10;
  const ACTIONS = ['space', 'delete', 'clear'] as const;
  let zone = $state<'kb' | 'res'>('kb');
  let k = $state(10); // 0..35 touches, 36..38 actions
  let input: HTMLInputElement | undefined = $state();
  let list: HTMLElement | undefined = $state();

  onMount(() => input && bindInput(input));

  function press(i: number) {
    if (i < KEYS.length) setQuery(query + KEYS[i]);
    else if (ACTIONS[i - KEYS.length] === 'space') setQuery(query.endsWith(' ') || !query ? query : query + ' ');
    else if (ACTIONS[i - KEYS.length] === 'delete') setQuery(query.slice(0, -1));
    else setQuery('');
    play('select');
  }

  function moveKb(dir: string): boolean {
    const rowsN = Math.ceil(KEYS.length / COLS); // 4 rangées (la dernière est partielle), puis les actions
    if (k >= KEYS.length) {
      const a = k - KEYS.length;
      if (dir === 'ArrowLeft') k = a > 0 ? k - 1 : k;
      else if (dir === 'ArrowRight') {
        if (a < ACTIONS.length - 1) k++;
        else if (rows.length) zone = 'res';
      } else if (dir === 'ArrowUp') k = (rowsN - 1) * COLS + a * 3;
      return true;
    }
    const r = Math.floor(k / COLS);
    const c = k % COLS;
    if (dir === 'ArrowLeft') k = c > 0 ? k - 1 : k;
    else if (dir === 'ArrowRight') {
      if (c < COLS - 1 && k + 1 < KEYS.length) k++;
      else if (rows.length) zone = 'res';
    } else if (dir === 'ArrowUp') k = r > 0 ? k - COLS : k;
    else if (dir === 'ArrowDown') k = (r + 1) * COLS + c < KEYS.length ? k + COLS : KEYS.length + Math.min(2, Math.floor(c / 3));
    return true;
  }

  function moveRes(dir: string): boolean {
    if (!rows.length) return (zone = 'kb'), true;
    let i = index;
    if (dir === 'ArrowLeft') return (zone = 'kb'), true;
    if (dir === 'ArrowUp') i = Math.max(0, i - 1);
    else if (dir === 'ArrowDown') i = Math.min(rows.length - 1, i + 1);
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
    list?.querySelector('.res.on')?.scrollIntoView({ block: 'nearest' });
  });
  const label = (a: (typeof ACTIONS)[number]) => t(`vidaa.kb.${a}`);
</script>

<div class="vsr" transition:fade={{ duration: 160 }}>
  <div class="top">
    <svg viewBox="0 0 24 24"><path d="M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13Zm5-1.5L21 21" /></svg>
    <input bind:this={input} value={query} oninput={(e) => setQuery(e.currentTarget.value)} {placeholder} autocomplete="off" spellcheck="false" />
  </div>
  <aside>
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
  </aside>
  <main bind:this={list}>
    {#if loading}<p class="note">{t('usearch.loading')}</p>{/if}
    {#if !rows.length && !loading}<p class="note">{query.trim() ? t('search.none') : t('vidaa.search.start')}</p>{/if}
    {#each rows as r, i}
      <button class="res" class:on={zone === 'res' && i === index} onmouseenter={() => ((zone = 'res'), hoverRow(i))} onclick={() => activateRow(i)}>
        <span class="art" class:poster={r.poster} style:--a={r.color ?? '#2c6aa8'}>
          {#if r.image}<img src={r.image} alt="" />{:else}<b>{r.title[0]}</b>{/if}
        </span>
        <span class="tx"><span class="nm">{r.title}</span><span class="sb">{r.sub ?? sectionLabel(r.section)}</span></span>
        {#if r.badge}<em>{r.badge}</em>{/if}
      </button>
    {/each}
    {#each notes as n}<p class="note">{n}</p>{/each}
  </main>
  {#if menu}
    <div class="menuwrap"><div class="menu"><h4>{menu.title}</h4>{#each menu.items as it, i}<div class="mi" class:on={i === menu.index}>{it}</div>{/each}</div></div>
  {/if}
</div>

<style>
  .vsr { position: absolute; inset: 0; z-index: 30; color: #fff; font-family: var(--font); background: #10121a; }
  .top { position: absolute; left: 5rem; right: 4.5rem; top: 2.6rem; display: flex; align-items: center; gap: 1.2rem; padding: 0 1.4rem; height: 4.8rem; border-bottom: 0.15rem solid rgba(255, 255, 255, 0.5); }
  .top svg { flex: none; width: 2.2rem; height: 2.2rem; fill: none; stroke: currentColor; stroke-width: 1.7; stroke-linecap: round; }
  input { flex: 1; min-width: 0; border: 0; background: transparent; color: #fff; font: inherit; font-size: 2rem; font-weight: 300; outline: 0; }
  input::placeholder { color: rgba(255, 255, 255, 0.45); }
  aside { position: absolute; left: 5rem; top: 10.4rem; width: 49rem; }
  .kb { display: grid; grid-template-columns: repeat(10, 1fr); gap: 0.4rem; }
  .acts { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.4rem; margin-top: 0.4rem; }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; }
  .key { height: 4.4rem; border: 0.2rem solid transparent; border-radius: 0.3rem; background: rgba(255, 255, 255, 0.08); font-size: 1.8rem; text-transform: uppercase; }
  .key.act { font-size: 1.2rem; text-transform: none; }
  .key.on { border-color: #fff; background: rgba(255, 255, 255, 0.2); }
  main { position: absolute; left: 58rem; right: 4.5rem; top: 10.4rem; bottom: 2.4rem; overflow: hidden auto; scrollbar-width: none; display: flex; flex-direction: column; gap: 0.4rem; padding: 0.3rem; }
  .res { display: flex; align-items: center; gap: 1.4rem; padding: 0.6rem 1rem; border: 0.2rem solid transparent; border-radius: 0.3rem; text-align: left; flex: none; }
  .res.on { border-color: #fff; background: rgba(255, 255, 255, 0.1); }
  .art { position: relative; flex: none; display: grid; place-items: center; width: 10rem; aspect-ratio: 16 / 9; overflow: hidden; border-radius: 0.3rem; background: linear-gradient(135deg, var(--a), #10121a); }
  .art.poster { width: 4.2rem; aspect-ratio: 2 / 3; }
  .art b { font-size: 2.2rem; }
  .art img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .tx { flex: 1; min-width: 0; display: grid; }
  .nm { font-size: 1.45rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sb { font-size: 1.05rem; color: rgba(255, 255, 255, 0.6); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  em { flex: none; padding: 0.1rem 0.7rem; border-radius: 0.3rem; background: rgba(47, 181, 200, 0.25); color: #7fe3f0; font-size: 0.95rem; font-style: normal; }
  .note { margin: 0.6rem 0.4rem; font-size: 1.2rem; color: rgba(255, 255, 255, 0.65); }
  .menuwrap { position: absolute; inset: 0; display: grid; place-items: center; background: rgba(5, 6, 10, 0.7); }
  .menu { min-width: 36rem; padding: 1.4rem; border-radius: 0.5rem; background: #1b1e2b; }
  .menu h4 { margin: 0 0 0.8rem 0.4rem; font-size: 1.3rem; font-weight: 400; }
  .mi { padding: 0.8rem 1.2rem; border: 0.2rem solid transparent; border-radius: 0.3rem; font-size: 1.3rem; }
  .mi.on { border-color: #fff; background: rgba(255, 255, 255, 0.1); }
</style>
