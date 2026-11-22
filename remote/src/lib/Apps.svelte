<script lang="ts">
  import { app, send } from './connection.svelte.ts';
  import { tick } from './haptics.ts';

  let filter = $state('');
  const shown = $derived(app.apps.filter((a) => a.name.toLowerCase().includes(filter.toLowerCase())));
  const initial = (n: string) => (n.trim()[0] ?? '?').toUpperCase();

  function launch(id: string) {
    tick(true);
    send({ t: 'launch', appId: id });
  }
</script>

<input placeholder="Rechercher…" bind:value={filter} />
<div class="grid">
  {#each shown as a (a.id)}
    <button onclick={() => launch(a.id)}>
      {#if a.icon}
        <img src={`data:image/png;base64,${a.icon}`} alt="" />
      {:else}
        <span class="ico">{initial(a.name)}</span>
      {/if}
      <span class="name">{a.name}</span>
    </button>
  {:else}
    <p>Aucune application.</p>
  {/each}
</div>

<style>
  input {
    background: var(--panel);
    color: var(--text);
    border: 0;
    border-radius: 12px;
    padding: 12px 14px;
    font: inherit;
  }
  .grid {
    flex: 1;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    align-content: start;
  }
  button {
    touch-action: pan-y;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 14px 6px;
    min-width: 0;
  }
  .ico,
  img {
    width: 44px;
    height: 44px;
    border-radius: 12px;
  }
  .ico {
    display: grid;
    place-items: center;
    background: var(--accent);
    font-weight: 700;
    font-size: 1.3rem;
  }
  .name {
    font-size: 0.78rem;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  p {
    color: var(--muted);
    grid-column: 1 / -1;
    text-align: center;
  }
</style>
