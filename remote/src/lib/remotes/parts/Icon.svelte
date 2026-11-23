<script lang="ts">
  // Pictogrammes génériques (viewBox 24x24) : aucun logo de marque.
  const ICONS: Record<string, { d: string[]; fill?: boolean }> = {
    back: { d: ['M15 6l-6 6 6 6'] },
    backArrow: { d: ['M9 5L4 10l5 5', 'M4 10h9a6 6 0 016 6v2'] },
    home: { d: ['M4 11l8-7 8 7v8a1 1 0 01-1 1h-4v-6H9v6H5a1 1 0 01-1-1z'] },
    tv: { d: ['M3 6h18v12H3z', 'M8 21h8'] },
    mic: { d: ['M12 3a3 3 0 00-3 3v6a3 3 0 006 0V6a3 3 0 00-3-3z', 'M6 11a6 6 0 0012 0', 'M12 17v4'] },
    menu: { d: ['M5 7h14M5 12h14M5 17h14'] },
    gear: { d: ['M12 9a3 3 0 100 6 3 3 0 000-6z', 'M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M5.6 18.4l2.1-2.1M16.3 7.7l2.1-2.1'] },
    play: { d: ['M8 5l11 7-11 7z'], fill: true },
    playpause: { d: ['M4 5l8 7-8 7z', 'M14 5h2.6v14H14zM19 5h2.6v14H19z'], fill: true },
    rew: { d: ['M11 7l-7 5 7 5z', 'M20 7l-7 5 7 5z'], fill: true },
    ff: { d: ['M13 7l7 5-7 5z', 'M4 7l7 5-7 5z'], fill: true },
    mute: { d: ['M4 9v6h4l5 4V5L8 9z', 'M16 9l5 6M21 9l-5 6'] },
    vol: { d: ['M4 9v6h4l5 4V5L8 9z', 'M16 8.5a5 5 0 010 7', 'M18.5 6a8.5 8.5 0 010 12'] },
    plus: { d: ['M12 5v14M5 12h14'] },
    minus: { d: ['M5 12h14'] },
    up: { d: ['M7 15l5-5 5 5'] },
    down: { d: ['M7 9l5 5 5-5'] },
    left: { d: ['M15 7l-5 5 5 5'] },
    right: { d: ['M9 7l5 5-5 5'] },
    replay: { d: ['M5 12a7 7 0 107-7H6', 'M9 2L5 5l4 3'] },
    star: { d: ['M12 4v16M5 8l14 8M19 8L5 16'] },
    ambilight: { d: ['M6 7h12v10H6z', 'M3 9v6M21 9v6M9 4h6M9 20h6'] },
    pointer: { d: ['M5 3l14 8-6 1.5L10 19z'] },
    input: { d: ['M4 6h16v12H4z', 'M8 12h7M12 9l3 3-3 3'] },
    search: { d: ['M11 5a6 6 0 100 12 6 6 0 000-12z', 'M16 16l4 4'] },
    kbd: { d: ['M3 7h18v10H3z', 'M7 11h.01M11 11h.01M15 11h.01M8 14h8'] },
    key: { d: ['M8 11a4 4 0 100-8 4 4 0 000 8z', 'M11 9l9 9M16 14l2 2M13 17l2 2'] },
    tabs: { d: ['M4 6h12v10H4z', 'M8 19h12V9'] },
  };
  let { name, size = 24, sw = 2 }: { name: string; size?: number | string; sw?: number } = $props();
  const ic = $derived(ICONS[name]);
</script>

{#if ic}
  <svg viewBox="0 0 24 24" width={size} height={size} fill={ic.fill ? 'currentColor' : 'none'} stroke="currentColor" stroke-width={sw} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    {#each ic.d as d}<path {d} />{/each}
  </svg>
{/if}
