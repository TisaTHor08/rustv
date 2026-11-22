<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import { t } from './i18n';
  import { askText, pickFile } from './overlays.svelte';
  import { TILE_COLORS, prefs, savePrefs } from './prefs.svelte';
  import { deleteCustom, hideTile, iconSrc, resetTile, type DTile } from './tiles.svelte';

  let {
    tile,
    onclose,
    onopen,
    onmove,
  }: { tile: DTile; onclose: () => void; onopen: () => void; onmove: (dir: -1 | 1) => void } = $props();

  interface Item {
    label: string;
    value?: string;
    swatch?: string;
    left?: () => void;
    right?: () => void;
    enter?: () => void | Promise<void>;
  }
  let idx = $state(0);

  function colorStep(d: number) {
    const cur = prefs.tiles.colors[tile.id]?.[0] ?? tile.color;
    const i = TILE_COLORS.findIndex((c) => c[0] === cur);
    const n = TILE_COLORS[(i + d + TILE_COLORS.length) % TILE_COLORS.length];
    prefs.tiles.colors[tile.id] = n;
    savePrefs();
  }

  const items = $derived.by<Item[]>(() => {
    const cur = prefs.tiles.colors[tile.id]?.[0] ?? tile.color;
    const out: Item[] = [
      { label: t('tilemenu.open'), enter: () => (onclose(), onopen()) },
      {
        label: t('tilemenu.rename'),
        enter: async () => {
          const n = await askText(t('tilemenu.rename'), tile.name);
          if (n) {
            prefs.tiles.names[tile.id] = n;
            savePrefs();
          }
        },
      },
      { label: t('tilemenu.color'), swatch: cur, left: () => colorStep(-1), right: () => colorStep(1), enter: () => colorStep(1) },
      {
        label: t('tilemenu.icon'),
        enter: async () => {
          const p = await pickFile(t('picker.title.image'), 'image');
          if (p) {
            prefs.tiles.icons[tile.id] = p;
            savePrefs();
          }
        },
      },
    ];
    if (tile.icon) {
      out.push({
        label: t('tilemenu.noicon'),
        enter: () => {
          delete prefs.tiles.icons[tile.id];
          // une icône intégrée à la tuile (application Windows) est aussi retirée
          const c = prefs.tiles.custom.find((x) => x.id === tile.id);
          if (c) c.icon = undefined;
          savePrefs();
        },
      });
    }
    out.push({ label: t('tilemenu.move'), value: '◀ ▶', left: () => onmove(-1), right: () => onmove(1) });
    if (tile.custom) out.push({ label: t('tilemenu.delete'), enter: () => (deleteCustom(tile.id), onclose()) });
    else out.push({ label: t('tilemenu.hide'), enter: () => (hideTile(tile.id), onclose()) });
    if (tile.customized) out.push({ label: t('tilemenu.reset'), enter: () => resetTile(tile.id) });
    return out;
  });

  function onKey(e: KeyboardEvent) {
    const it = items[idx];
    if (e.key === 'ArrowDown') idx = Math.min(items.length - 1, idx + 1);
    else if (e.key === 'ArrowUp') idx = Math.max(0, idx - 1);
    else if (e.key === 'ArrowLeft') it?.left?.();
    else if (e.key === 'ArrowRight') it?.right?.();
    else if (e.key === 'Enter') it?.enter?.();
    else if (e.key === 'Escape' || e.key === 'Backspace' || e.key === 'ContextMenu') onclose();
    else return;
    if (idx >= items.length) idx = items.length - 1;
    e.preventDefault();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="sheet" in:fly={{ y: 30, duration: 320, easing: cubicOut }}>
  <div class="head">
    <span class="mini" style:background="linear-gradient(135deg, {tile.color}, {tile.color2})">
      {#if tile.icon}<img src={iconSrc(tile.icon)} alt="" />{:else}{tile.name[0]}{/if}
    </span>
    <h3>{tile.name}</h3>
  </div>
  {#each items as it, i}
    <button class="item" class:sel={i === idx} onmouseenter={() => (idx = i)} onclick={() => it.enter?.()}>
      <span>{it.label}</span>
      <span class="val">{#if it.swatch}<i style:background={it.swatch}></i>{/if}{#if it.left}<b>◀</b>{/if}{it.value ?? ''}{#if it.right}<b>▶</b>{/if}</span>
    </button>
  {/each}
  <p class="hint">{t('tilemenu.hint')}</p>
</div>

<style>
  .sheet { width: 38rem; padding: 2rem 2.2rem; border-radius: 1.8rem; background: var(--sheet); box-shadow: 0 3rem 6rem rgba(0, 0, 0, 0.55), inset 0 0 0 0.08rem var(--glass-strong); color: var(--text); }
  .head { display: flex; align-items: center; gap: 1rem; margin-bottom: 0.9rem; }
  h3 { margin: 0; font-size: 1.8rem; }
  .mini { width: 3.6rem; height: 3.6rem; border-radius: 0.9rem; display: grid; place-items: center; font-weight: 800; font-size: 1.6rem; color: #fff; overflow: hidden; }
  .mini img { width: 70%; height: 70%; object-fit: contain; }
  .item { display: flex; justify-content: space-between; align-items: center; width: 100%; text-align: left; margin: 0.2rem 0; padding: 0.85rem 1.1rem; border: 0; border-radius: 0.9rem; background: transparent; color: var(--text); font-family: inherit; font-size: 1.25rem; transition: background 0.2s, transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .item.sel { background: var(--accent); color: #fff; font-weight: 600; transform: scale(1.02); }
  .val { display: flex; align-items: center; gap: 0.6rem; font-size: 1.15rem; }
  .val b { font-size: 0.8rem; opacity: 0.6; }
  .val i { width: 1.4rem; height: 1.4rem; border-radius: 50%; box-shadow: 0 0 0 0.12rem rgba(255, 255, 255, 0.7); }
  .hint { margin: 1rem 0 0; font-size: 0.92rem; color: var(--muted); }
</style>
