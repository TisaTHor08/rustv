<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t } from '../../../i18n';
  import { play } from '../../../sounds';
  import type { SearchScreenProps } from '../../screens';

  // Recherche style Roku : clavier à l'écran à gauche (flèches + OK), résultats en grille à droite.
  // La saisie au clavier du téléphone fonctionne aussi : le champ ci-dessous est le vrai champ de saisie.
  let { query, setQuery, rows, index, loading, sectionLabel, notes, placeholder, menu, bindInput, hoverRow, activateRow, setKeyboardHandler }: SearchScreenProps = $props();

  const KEYS = [...'abcdefghijklmnopqrstuvwxyz0123456789'];
  const COLS = 6;
  const ACTIONS = ['space', 'delete', 'clear'] as const;
  const RCOLS = 3;
  let zone = $state<'kb' | 'res'>('kb');
  let k = $state(0); // 0..35 touches, 36..38 actions
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
    const rowsN = KEYS.length / COLS; // 6 rangées de touches, puis la rangée d'actions
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

  // Touches fléchées et OK : traitées ici ; le reste (lettres du téléphone, Retour) continue vers le champ ou Recherche.
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
  const label = (a: (typeof ACTIONS)[number]) => t(`roku.kb.${a}`);
</script>

<div class="rsr" transition:fade={{ duration: 160 }}>
  <aside>
    <h1>{t('roku.search')}</h1>
    <input bind:this={input} value={query} oninput={(e) => setQuery(e.currentTarget.value)} {placeholder} autocomplete="off" spellcheck="false" />
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
    {#if !rows.length && !loading}<p class="note">{query.trim() ? t('search.none') : t('roku.search.start')}</p>{/if}
    <div class="grid">
      {#each rows as r, i}
        <button class="card" class:on={zone === 'res' && i === index} onmouseenter={() => ((zone = 'res'), hoverRow(i))} onclick={() => activateRow(i)}>
          <span class="art" class:poster={r.poster} style:--a={r.color ?? '#5b2bff'}>
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
  .rsr { position: absolute; inset: 0; z-index: 30; color: #fff; font-family: var(--font); background: radial-gradient(70rem 50rem at 10% 0%, #662d91 0%, transparent 70%), linear-gradient(160deg, #3b1259 0%, #1b0a2c 55%, #0b0412 100%); }
  aside { position: absolute; left: 5rem; top: 3rem; width: 40rem; }
  h1 { margin: 0 0 1.4rem; font-size: 2.6rem; font-weight: 700; }
  input { width: 100%; box-sizing: border-box; padding: 0.9rem 1.2rem; border: 0; border-radius: 0.4rem; background: rgba(255, 255, 255, 0.95); color: #2b0f45; font: inherit; font-size: 1.7rem; font-weight: 600; outline: 0; }
  input::placeholder { color: #8a7a9b; font-weight: 500; }
  .kb { display: grid; grid-template-columns: repeat(6, 1fr); gap: 0.4rem; margin-top: 1.4rem; }
  button { font-family: inherit; border: 0; cursor: pointer; color: inherit; background: transparent; }
  .key { height: 4.8rem; border-radius: 0.4rem; font-size: 2rem; font-weight: 600; text-transform: uppercase; background: rgba(255, 255, 255, 0.1); transition: background 0.12s; }
  .key.act { grid-column: span 2; font-size: 1.2rem; text-transform: none; letter-spacing: 0.02em; }
  .key.on { background: #fff; color: #2b0f45; box-shadow: 0 0.4rem 1.2rem rgba(0, 0, 0, 0.4); }
  main { position: absolute; left: 50rem; right: 4rem; top: 3rem; bottom: 3rem; overflow: hidden auto; scrollbar-width: none; padding: 0.8rem 1rem; }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1.6rem; }
  .card { text-align: left; padding: 0; transition: transform 0.18s var(--ease); }
  .art { position: relative; display: grid; place-items: center; aspect-ratio: 16 / 9; border-radius: 0.5rem; overflow: hidden; background: linear-gradient(135deg, var(--a), #1a0a2c); box-shadow: 0 0.4rem 1rem rgba(0, 0, 0, 0.4); }
  .art b { font-size: 3.6rem; font-weight: 800; }
  .art img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .art.poster img { object-fit: contain; background: #12071f; }
  .art em { position: absolute; left: 0.5rem; bottom: 0.5rem; padding: 0.1rem 0.6rem; border-radius: 0.3rem; background: rgba(0, 0, 0, 0.6); font-size: 0.85rem; font-style: normal; font-weight: 700; }
  .nm { display: block; margin-top: 0.5rem; font-size: 1.2rem; font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sb { display: block; font-size: 0.95rem; color: rgba(255, 255, 255, 0.7); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card.on { transform: scale(1.06); position: relative; z-index: 2; }
  .card.on .art { box-shadow: 0 0 0 0.3rem #fff, 0 1rem 2rem rgba(0, 0, 0, 0.55); }
  .note { margin: 0.6rem 0; font-size: 1.2rem; color: rgba(255, 255, 255, 0.75); }
  .menuwrap { position: absolute; inset: 0; display: grid; place-items: center; background: rgba(0, 0, 0, 0.6); }
  .menu { min-width: 36rem; padding: 1.4rem; border-radius: 0.6rem; background: #2a1044; }
  .menu h4 { margin: 0 0 0.8rem 0.4rem; font-size: 1.3rem; }
  .mi { padding: 0.8rem 1.2rem; border-radius: 0.4rem; font-size: 1.3rem; }
  .mi.on { background: #fff; color: #2b0f45; font-weight: 700; }
</style>
