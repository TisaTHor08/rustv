<script lang="ts">
  import { app, send } from './connection.svelte.ts';
  import { tick } from './haptics.ts';
  import { label, modName, panelsOf, runActions } from './mods.ts';
  import type { RemoteButton } from './protocol/types.ts';
  import Glyph from './Glyph.svelte';

  const panels = $derived(panelsOf(app.remoteMods));
  let sel = $state('');
  const current = $derived(panels.find((p) => `${p.mod.id}/${p.panel.id}` === sel) ?? panels[0]);
  let running = $state(false);
  let pressed = $state(-1);

  async function press(b: RemoteButton, i: number) {
    if (running) return;
    running = true;
    pressed = i;
    tick(true);
    setTimeout(() => (pressed = -1), 160);
    try {
      await runActions(b.actions ?? [], send);
    } finally {
      running = false;
    }
  }
</script>

{#if current}
  {#if panels.length > 1}
    <div class="chips">
      {#each panels as p (p.mod.id + '/' + p.panel.id)}
        <button class:sel={p === current} onclick={() => (sel = `${p.mod.id}/${p.panel.id}`)}>
          {label(p.panel.name)}<small>{modName(p.mod)}</small>
        </button>
      {/each}
    </div>
  {/if}
  <div class="grid">
    {#each current.panel.buttons as b, i}
      <button class="big" class:pressed={pressed === i} onpointerdown={(e) => { e.preventDefault(); void press(b, i); }}>
        <Glyph name={b.glyph} size={28} />
        <span>{label(b.label)}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .chips {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    flex: none;
  }
  .chips button {
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 6px 14px;
    border-radius: 14px;
    font-size: 0.85rem;
    background: var(--panel);
  }
  .chips .sel {
    background: var(--accent);
  }
  .chips small {
    font-size: 0.65rem;
    color: var(--muted);
  }
  .chips .sel small {
    color: #ffffffcc;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    align-content: start;
    overflow-y: auto;
  }
  .big {
    aspect-ratio: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 6px;
    border-radius: 18px;
    background: var(--panel);
    font-size: 0.8rem;
    text-align: center;
    transition: transform 0.12s, background 0.12s;
  }
  .big span {
    overflow-wrap: anywhere;
  }
  .big.pressed {
    transform: scale(0.94);
    background: var(--accent);
  }
</style>
