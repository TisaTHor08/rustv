<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import { play } from '../../../sounds';
  import type { SearchScreenProps } from '../../screens';
  import { KB_ACTIONS, KB_ROWS, kbApply, kbMove, keyAt, type KbPos } from '../kbd';

  // Recherche style Tizen : champ en pilule en haut, résultats en cartes arrondies, clavier flottant en bas (flèches + OK).
  // Le champ est le vrai champ de saisie : le clavier du téléphone fonctionne aussi.
  let { query, setQuery, rows, index, loading, sectionLabel, notes, placeholder, menu, bindInput, hoverRow, activateRow, setKeyboardHandler }: SearchScreenProps = $props();

  const COLS = 5;
  let zone = $state<'kb' | 'res'>('kb');
  let pos = $state<KbPos>({ r: 1, c: 0 });
  let input: HTMLInputElement | undefined = $state();
  let grid: HTMLElement | undefined = $state();
  onMount(() => input && bindInput(input));

  function press(p: KbPos) {
    setQuery(kbApply(query, p));
    play('select');
  }
  function moveRes(k: string) {
    const n = rows.length;
    let i = index;
    if (k === 'ArrowLeft') i = i % COLS > 0 ? i - 1 : i;
    else if (k === 'ArrowRight') i = i % COLS < COLS - 1 && i + 1 < n ? i + 1 : i;
    else if (k === 'ArrowUp') i = i >= COLS ? i - COLS : i;
    else if (k === 'ArrowDown') {
      if (i + COLS < n) i += COLS;
      else return void (zone = 'kb');
    }
    if (i !== index) (hoverRow(i), play('move'));
  }
  const handler = (e: KeyboardEvent): boolean => {
    if (menu) return false;
    if (e.key.startsWith('Arrow')) {
      if (zone === 'res') moveRes(e.key);
      else {
        const m = kbMove(pos, e.key as any);
        if (m.edge === 'up') {
          if (rows.length) zone = 'res';
        } else if (m.pos.r !== pos.r || m.pos.c !== pos.c) (pos = m.pos, play('move'));
      }
      return true;
    }
    if (e.key === 'Enter') {
      if (zone === 'kb') press(pos);
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
    void zone;
    grid?.querySelector('.card.on')?.scrollIntoView({ block: 'nearest' });
  });
  const act = (a: string) => t(`tizen.kb.${a}`);
</script>

<div class="tsr" transition:fade={{ duration: 160 }}>
  <div class="field">
    <svg viewBox="0 0 24 24"><path d="M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z" /></svg>
    <input bind:this={input} value={query} oninput={(e) => setQuery(e.currentTarget.value)} {placeholder} autocomplete="off" spellcheck="false" />
  </div>
  <main bind:this={grid}>
    {#if loading}<p class="note">{t('usearch.loading')}</p>{/if}
    {#if !rows.length && !loading}<p class="note">{query.trim() ? t('search.none') : t('tizen.search.start')}</p>{/if}
    <div class="grid">
      {#each rows as r, i}
        <button class="card" class:on={zone === 'res' && i === index} onmouseenter={() => ((zone = 'res'), hoverRow(i))} onclick={() => activateRow(i)}>
          <span class="art" class:poster={r.poster} style:--a={r.color ?? '#3d6bd9'}>
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
  <div class="kb">
    {#each KB_ROWS as row, r}
      <div class="kr">
        {#each [...row] as ch, c}
          <button class="key" class:on={zone === 'kb' && pos.r === r && pos.c === c} onmouseenter={() => ((zone = 'kb'), (pos = { r, c }))} onclick={() => press({ r, c })}>{ch}</button>
        {/each}
      </div>
    {/each}
    <div class="kr">
      {#each KB_ACTIONS as a, c}
        <button class="key act" class:on={zone === 'kb' && pos.r === KB_ROWS.length && pos.c === c} onmouseenter={() => ((zone = 'kb'), (pos = { r: KB_ROWS.length, c }))} onclick={() => press({ r: KB_ROWS.length, c })}>{act(a)}</button>
      {/each}
    </div>
  </div>
  {#if menu}
    <div class="menuwrap"><div class="menu"><h4>{menu.title}</h4>{#each menu.items as it, i}<div class="mi" class:on={i === menu.index}>{it}</div>{/each}</div></div>
  {/if}
</div>

<style>
  .tsr { position: absolute; inset: 0; z-index: 30; color: #f4f5fa; font-family: var(--font); background: radial-gradient(80rem 50rem at 50% 0%, #26285a 0%, transparent 70%), linear-gradient(180deg, #14162f 0%, #080915 100%); }
  .field { position: absolute; left: 50%; top: 2.8rem; transform: translateX(-50%); width: 62rem; display: flex; align-items: center; gap: 1.2rem; padding: 0.9rem 2rem; border-radius: 99rem; background: rgba(255, 255, 255, 0.14); box-shadow: inset 0 0 0 0.1rem rgba(255, 255, 255, 0.3); box-sizing: border-box; }
  .field svg { width: 2.4rem; height: 2.4rem; fill: currentColor; opacity: 0.8; }
  input { flex: 1; min-width: 0; border: 0; background: transparent; color: inherit; font: inherit; font-size: 1.9rem; outline: 0; }
  input::placeholder { color: rgba(255, 255, 255, 0.5); }
  main { position: absolute; left: 6rem; right: 6rem; top: 8.6rem; bottom: 22rem; overflow: hidden auto; scrollbar-width: none; padding: 0.8rem 1rem; }
  .grid { display: grid; grid-template-columns: repeat(5, 1fr); gap: 1.4rem; }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; }
  .card { text-align: left; padding: 0; transition: transform 0.25s cubic-bezier(0.3, 1.3, 0.5, 1); }
  .art { position: relative; display: grid; place-items: center; aspect-ratio: 16 / 9; border-radius: 1.4rem; overflow: hidden; background: linear-gradient(135deg, var(--a), #151735); box-shadow: 0 0.4rem 1rem rgba(0, 0, 0, 0.4); }
  .art b { font-size: 3.2rem; font-weight: 700; opacity: 0.6; }
  .art img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .art.poster img { object-fit: contain; background: #0c0d1c; }
  .art em { position: absolute; left: 0.6rem; bottom: 0.6rem; padding: 0.1rem 0.7rem; border-radius: 99rem; background: rgba(0, 0, 0, 0.65); font-size: 0.85rem; font-style: normal; font-weight: 600; }
  .nm { display: block; margin-top: 0.5rem; font-size: 1.2rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sb { display: block; font-size: 0.95rem; color: rgba(255, 255, 255, 0.6); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card.on { transform: scale(1.05); position: relative; z-index: 2; }
  .card.on .art { box-shadow: 0 0 0 0.22rem #fff, 0 1rem 2rem rgba(0, 0, 0, 0.55); }
  .note { margin: 0.6rem 0; font-size: 1.25rem; color: rgba(255, 255, 255, 0.7); text-align: center; }
  .kb { position: absolute; left: 50%; bottom: 2rem; transform: translateX(-50%); width: 68rem; padding: 1.2rem; box-sizing: border-box; border-radius: 2.4rem; background: linear-gradient(160deg, rgba(60, 62, 110, 0.6), rgba(24, 25, 46, 0.92)); box-shadow: inset 0 0 0 0.1rem rgba(255, 255, 255, 0.28), 0 1.6rem 4rem rgba(0, 0, 0, 0.5); display: grid; gap: 0.5rem; }
  .kr { display: flex; justify-content: center; gap: 0.5rem; }
  .key { width: 5.6rem; height: 3.4rem; border-radius: 1rem; font-size: 1.6rem; font-weight: 500; text-transform: uppercase; background: rgba(255, 255, 255, 0.1); transition: background 0.12s; }
  .key.act { width: 14rem; font-size: 1.15rem; text-transform: none; }
  .key.on { background: #fff; color: #10131c; }
  .menuwrap { position: absolute; inset: 0; display: grid; place-items: center; background: rgba(0, 0, 0, 0.6); }
  .menu { min-width: 36rem; padding: 1.4rem; border-radius: 2rem; background: #1c1d38; box-shadow: inset 0 0 0 0.1rem rgba(255, 255, 255, 0.28); }
  .menu h4 { margin: 0 0 0.8rem 0.6rem; font-size: 1.3rem; }
  .mi { padding: 0.8rem 1.2rem; border-radius: 1.2rem; font-size: 1.3rem; }
  .mi.on { background: #fff; color: #10131c; }
</style>
