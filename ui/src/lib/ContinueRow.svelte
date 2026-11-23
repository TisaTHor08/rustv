<script lang="ts">
  import { t } from './i18n';
  import { iconSrc } from './tiles.svelte';
  import type { ContinueEntry } from './player.svelte';

  let { items, focus = -1, tilt = '', onopen, onfocus }: { items: ContinueEntry[]; focus?: number; tilt?: string; onopen: (i: number) => void; onfocus: (i: number) => void } = $props();

  const pct = (e: ContinueEntry) => (e.duration > 0 ? Math.min(100, Math.round((e.position / e.duration) * 100)) : 0);
  function left(e: ContinueEntry): string {
    if (e.duration <= 0) return '';
    const s = Math.max(0, Math.round(e.duration - e.position));
    const h = Math.floor(s / 3600);
    const m = Math.max(1, Math.floor((s % 3600) / 60));
    return t('continue.left', { t: h > 0 ? `${h} h ${String(m).padStart(2, '0')}` : `${m} min` });
  }
</script>

{#each items as e, i (e.url)}
  <button class="card" class:focus={i === focus} data-tilt={i === focus ? tilt : ''} onmouseenter={() => onfocus(i)} onclick={() => onopen(i)}>
    {#if e.poster}<img class="art" src={iconSrc(e.poster)} alt="" />{:else}<span class="mark">{(e.title || '?')[0]}</span>{/if}
    <span class="shade"></span>
    <span class="label">{e.title}</span>
    <span class="sub">{left(e)}</span>
    <span class="bar"><i style:width="{pct(e)}%"></i></span>
  </button>
{/each}

<style>
  .card { position: relative; flex: none; width: 17rem; height: 9.6rem; border: 0; padding: 0; border-radius: 1.2rem; overflow: hidden; color: #fff; font-family: inherit; cursor: pointer; background: linear-gradient(135deg, var(--glass-strong), var(--glass)); box-shadow: 0 0.6rem 1.4rem -0.6rem rgba(0, 0, 0, 0.6); transition: transform 0.4s var(--ease), box-shadow 0.4s var(--ease); }
  .art { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .mark { position: absolute; right: -0.6rem; bottom: -2.6rem; font-size: 10rem; font-weight: 800; opacity: 0.14; line-height: 1; }
  .shade { position: absolute; inset: 0; background: linear-gradient(to top, rgba(0, 0, 0, 0.85), transparent 65%); }
  .label { position: absolute; left: 1rem; right: 1rem; bottom: 2.3rem; font-size: 1.3rem; font-weight: 700; text-align: left; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sub { position: absolute; left: 1rem; right: 1rem; bottom: 0.85rem; font-size: 0.95rem; opacity: 0.8; text-align: left; }
  .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 0.35rem; background: rgba(255, 255, 255, 0.25); }
  .bar i { display: block; height: 100%; background: var(--accent); }
  .card.focus { transform: perspective(60rem) translateY(-0.3rem) scale(1.16); box-shadow: 0 0 0 0.22rem var(--ring), 0 1rem 2rem rgba(0, 0, 0, 0.5); z-index: 2; }
</style>
