<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import type { SearchScreenProps } from '../../screens';

  // Recherche style Google TV : grand champ en haut, résultats en lignes larges groupées par section.
  let { query, setQuery, rows, index, chipIndex, loading, sectionLabel, notes, placeholder, hint, menu, bindInput, hoverRow, activateRow, activateChip }: SearchScreenProps = $props();
  let input: HTMLInputElement | undefined = $state();
  onMount(() => input && bindInput(input));
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    void index;
    list?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="gsr" transition:fade={{ duration: 160 }}>
  <label class="field">
    <svg viewBox="0 0 24 24"><path d="M15.5 14h-.8l-.3-.3A6.5 6.5 0 1 0 14 15.5l.3.3v.8l5 5 1.5-1.5-5-5Zm-6 0a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z" /></svg>
    <input bind:this={input} value={query} oninput={(e) => setQuery(e.currentTarget.value)} {placeholder} autocomplete="off" spellcheck="false" />
  </label>
  <div class="list" bind:this={list}>
    {#if loading}<p class="note">…</p>{/if}
    {#each rows as r, i}
      {#if i === 0 || rows[i - 1].section !== r.section}<h3>{sectionLabel(r.section)}</h3>{/if}
      <div class="row" class:on={i === index} onmouseenter={() => hoverRow(i)} role="presentation">
        <button class="hit" onclick={() => activateRow(i)}>
          <span class="ic" class:poster={r.poster} style:background={r.color ?? 'var(--glass-strong)'}>
            {#if r.image}<img src={r.image} alt="" />{:else}{r.title[0]}{/if}
          </span>
          <span class="tx"><b>{r.title}</b>{#if r.sub}<small>{r.sub}</small>{/if}</span>
          {#if r.badge}<span class="badge">{r.badge}</span>{/if}
        </button>
        {#if r.chips}
          <div class="chips">
            {#each r.chips as c, k}<button class="chip" class:off={!c.enabled} class:sel={i === index && k === chipIndex} onclick={() => activateChip(i, k)}>{c.label}{#if c.sub}<small>{c.sub}</small>{/if}</button>{/each}
          </div>
        {/if}
      </div>
    {/each}
    {#each notes as n}<p class="note">{n}</p>{/each}
  </div>
  <p class="hint">{hint}</p>
  {#if menu}
    <div class="menuwrap"><div class="menu"><h4>{menu.title}</h4>{#each menu.items as it, i}<div class="mi" class:on={i === menu.index}>{it}</div>{/each}</div></div>
  {/if}
</div>

<style>
  .gsr { position: absolute; inset: 0; z-index: 30; padding: 3.6rem 6rem 2rem; box-sizing: border-box; background: radial-gradient(60rem 40rem at 90% -10%, rgba(80, 90, 170, 0.22), transparent 70%), var(--bg); color: var(--text); font-family: var(--font); }
  .field { display: flex; align-items: center; gap: 1.2rem; height: 5.6rem; padding: 0 2rem; border-radius: 99rem; background: var(--glass); box-shadow: inset 0 0 0 0.15rem var(--glass-strong); }
  .field svg { width: 2.4rem; height: 2.4rem; fill: var(--muted); flex: none; }
  input { flex: 1; min-width: 0; border: 0; outline: 0; background: transparent; color: var(--text); font: inherit; font-size: 2.1rem; }
  input::placeholder { color: var(--muted); }
  .list { margin-top: 1.6rem; height: calc(100% - 11rem); overflow: hidden auto; scrollbar-width: none; }
  h3 { margin: 1.6rem 0 0.5rem 1.2rem; font-size: 1.15rem; font-weight: 500; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); }
  .row { border-radius: 1.4rem; transition: background 0.15s; }
  .row.on { background: #e8eaed; color: #1f1f1f; }
  button { font-family: inherit; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  .hit { display: flex; align-items: center; gap: 1.4rem; width: 100%; padding: 0.8rem 1.2rem; }
  .ic { flex: none; width: 4.4rem; height: 4.4rem; border-radius: 0.9rem; display: grid; place-items: center; font-size: 1.8rem; font-weight: 600; color: #fff; overflow: hidden; }
  .ic.poster { width: 4.2rem; height: 6.2rem; border-radius: 0.6rem; }
  .ic img { width: 100%; height: 100%; object-fit: cover; }
  .tx { display: flex; flex-direction: column; min-width: 0; flex: 1; }
  .tx b { font-size: 1.6rem; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tx small { font-size: 1.1rem; opacity: 0.7; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .badge { font-size: 0.95rem; padding: 0.2rem 0.8rem; border-radius: 99rem; background: rgba(128, 128, 128, 0.25); }
  .chips { display: flex; flex-wrap: wrap; gap: 0.6rem; padding: 0 1.2rem 0.9rem 7.2rem; }
  .chip { padding: 0.45rem 1.1rem; border-radius: 99rem; font-size: 1.15rem; background: rgba(128, 128, 128, 0.22); }
  .chip small { display: block; font-size: 0.8rem; opacity: 0.7; }
  .chip.off { opacity: 0.45; }
  .chip.sel { background: #1f1f1f; color: #fff; box-shadow: 0 0 0 0.15rem #8ab4f8; }
  .note { margin: 0.8rem 1.2rem; font-size: 1.1rem; color: var(--muted); }
  .hint { position: absolute; left: 6rem; bottom: 1.2rem; margin: 0; font-size: 1rem; color: var(--muted); }
  .menuwrap { position: absolute; inset: 0; display: grid; place-items: center; background: rgba(0, 0, 0, 0.6); }
  .menu { min-width: 36rem; padding: 1.4rem; border-radius: 2rem; background: #2d2f33; }
  .menu h4 { margin: 0 0 0.8rem 0.6rem; font-size: 1.4rem; font-weight: 500; }
  .mi { padding: 0.9rem 1.4rem; border-radius: 99rem; font-size: 1.35rem; }
  .mi.on { background: #e8eaed; color: #1f1f1f; }
</style>
