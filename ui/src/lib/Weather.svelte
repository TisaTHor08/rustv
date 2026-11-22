<script lang="ts">
  import { prefs } from './prefs.svelte';
  import { currentWeather, type Current } from './weather';

  let cur = $state<Current | null>(null);

  $effect(() => {
    const w = prefs.weather;
    cur = null;
    if (!w.enabled || w.lat == null || w.lon == null) return;
    const load = () => currentWeather(w.lat!, w.lon!, w.unit).then((c) => (cur = c)).catch(() => {});
    load();
    const id = setInterval(load, 30 * 60 * 1000);
    return () => clearInterval(id);
  });
</script>

{#if cur}
  <div class="w">
    <svg viewBox="0 0 32 32" aria-hidden="true">
      {#if cur.kind === 'sun'}
        <circle cx="16" cy="16" r="6" fill="#ffc83d" />
        {#each Array(8) as _, i}<rect x="15" y="2" width="2" height="5" rx="1" fill="#ffc83d" transform="rotate({i * 45} 16 16)" />{/each}
      {:else if cur.kind === 'partly'}
        <circle cx="12" cy="12" r="5" fill="#ffc83d" />
        <path d="M10 25h13a5 5 0 0 0 .6-10 7 7 0 0 0-13.4 2A4.3 4.3 0 0 0 10 25Z" fill="#e6ebf5" />
      {:else}
        <path d="M9 22h14a5 5 0 0 0 .6-10 7.5 7.5 0 0 0-14.2 2A4.3 4.3 0 0 0 9 22Z" fill={cur.kind === 'storm' ? '#9aa3b5' : '#d8deea'} />
        {#if cur.kind === 'rain'}
          {#each [11, 16, 21] as x}<rect x={x} y="24" width="2" height="5" rx="1" fill="#5aa9ff" transform="rotate(18 {x} 24)" />{/each}
        {:else if cur.kind === 'snow'}
          {#each [11, 16, 21] as x}<circle cx={x} cy="27" r="1.6" fill="#fff" />{/each}
        {:else if cur.kind === 'storm'}
          <path d="M16 22l-3 6h3l-1 4 5-7h-3l2-3Z" fill="#ffc83d" />
        {:else if cur.kind === 'fog'}
          <rect x="7" y="25" width="18" height="2" rx="1" fill="#b7c0d1" />
        {/if}
      {/if}
    </svg>
    <strong>{cur.temp}°</strong>
  </div>
{/if}

<style>
  .w { display: flex; align-items: center; gap: 0.5rem; padding: 0.3rem 1.1rem 0.3rem 0.7rem; border-radius: 99rem; background: var(--glass); backdrop-filter: blur(1.2rem); }
  svg { width: 2.2rem; height: 2.2rem; }
  strong { font-size: 1.35rem; font-weight: 600; }
</style>
