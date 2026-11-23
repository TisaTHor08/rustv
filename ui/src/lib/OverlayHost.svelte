<script lang="ts">
  import { tick } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import { api, type FsEntry, type FsListing, type WinApp } from './api';
  import { overlay } from './overlays.svelte';
  import { t } from './i18n';
  import { play } from './sounds';

  // ---------- saisie de texte ----------
  let promptInput = $state<HTMLInputElement | null>(null);
  $effect(() => {
    if (overlay.prompt) tick().then(() => promptInput?.focus());
  });

  // ---------- sélecteur de fichier ----------
  interface Item { label: string; path?: string; action?: 'choose' | 'up' | 'open'; dir?: boolean; entry?: FsEntry }
  let roots = $state<{ name: string; path: string }[]>([]);
  let listing = $state<FsListing | null>(null);
  let idx = $state(0);
  let error = $state('');
  let preview = $state('');
  let listEl = $state<HTMLDivElement | null>(null);

  const items = $derived.by<Item[]>(() => {
    if (!overlay.file) return [];
    if (!listing) return roots.map((r) => ({ label: r.name, path: r.path, action: 'open', dir: true }));
    const out: Item[] = [];
    if (overlay.file.filter === 'folder') out.push({ label: t('picker.choosehere'), action: 'choose', path: listing.path });
    out.push({ label: t('picker.up'), action: 'up' });
    for (const e of listing.entries) out.push({ label: e.name, path: e.path, dir: e.dir, entry: e });
    return out;
  });

  $effect(() => {
    if (overlay.file) {
      listing = null;
      idx = 0;
      error = '';
      preview = '';
      api.fsRoots().then((r) => (roots = r));
    }
  });

  const isImage = (p: string) => /\.(jpe?g|png|webp|gif|bmp|avif|svg)$/i.test(p);

  $effect(() => {
    const it = items[idx];
    preview = '';
    if (overlay.file && it && !it.dir && it.path && isImage(it.path)) {
      const p = it.path;
      api.mediaUrl(p).then((u) => {
        if (items[idx]?.path === p) preview = u;
      });
    }
  });

  async function open(path: string) {
    try {
      listing = await api.fsList(path, overlay.file!.filter);
      idx = 0;
      error = '';
    } catch (e) {
      error = String(e);
    }
  }

  async function goUp() {
    if (!listing) return finishFile(null);
    if (listing.parent) await open(listing.parent);
    else {
      listing = null;
      idx = 0;
    }
  }

  function finishFile(v: string | null) {
    const o = overlay.file;
    overlay.file = null;
    o?.resolve(v);
  }

  async function activateFile() {
    const it = items[idx];
    if (!it) return;
    if (it.action === 'choose') return finishFile(it.path!);
    if (it.action === 'up') return goUp();
    if (it.dir) return open(it.path!);
    finishFile(it.path!);
  }

  // ---------- sélecteur d'application ----------
  let apps = $state<WinApp[]>([]);
  let filter = $state('');
  let aidx = $state(0);
  let appLoading = $state(false);
  let appIcon = $state('');
  let appInput = $state<HTMLInputElement | null>(null);
  const shown = $derived(apps.filter((a) => a.name.toLowerCase().includes(filter.trim().toLowerCase())).slice(0, 200));

  $effect(() => {
    if (overlay.app) {
      apps = [];
      filter = '';
      aidx = 0;
      appLoading = true;
      api.windowsApps().then((a) => {
        apps = a;
        appLoading = false;
      });
      tick().then(() => appInput?.focus());
    }
  });
  $effect(() => {
    const a = shown[aidx];
    appIcon = '';
    if (overlay.app && a) api.appIcon(a.target).then((i) => { if (shown[aidx]?.id === a.id) appIcon = i ?? ''; });
  });
  $effect(() => {
    filter;
    aidx = 0;
  });

  function finishApp(v: WinApp | null) {
    const o = overlay.app;
    overlay.app = null;
    o?.resolve(v);
  }

  // ---------- confirmation ----------
  let cidx = $state(0);
  $effect(() => {
    if (overlay.confirm) cidx = 0;
  });
  function finishConfirm(v: boolean) {
    const c = overlay.confirm;
    overlay.confirm = null;
    c?.resolve(v);
  }

  function cue(key: string) {
    if (key === 'Enter') play('select');
    else if (key === 'Escape' || (key === 'Backspace' && !overlay.prompt)) play('back');
    else if (key.startsWith('Arrow') && (overlay.confirm || overlay.file || overlay.app)) play('move');
  }

  // ---------- clavier (phase de capture : prioritaire sur les autres écrans) ----------
  function onKey(e: KeyboardEvent) {
    const stop = () => {
      e.preventDefault();
      e.stopImmediatePropagation();
    };
    if (e.type === 'keydown' && !overlay.busy) cue(e.key);
    if (overlay.busy) return stop(); // opération en cours : on ignore les touches
    if (overlay.confirm) {
      stop();
      if (e.key === 'ArrowDown' || e.key === 'ArrowRight') cidx = 1;
      else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') cidx = 0;
      else if (e.key === 'Enter') finishConfirm(cidx === 0);
      else if (e.key === 'Escape' || e.key === 'Backspace') finishConfirm(false);
      return;
    }
    if (overlay.prompt) {
      if (e.key === 'Enter') {
        stop();
        const p = overlay.prompt;
        overlay.prompt = null;
        p.resolve(p.value.trim() || null);
      } else if (e.key === 'Escape') {
        stop();
        const p = overlay.prompt;
        overlay.prompt = null;
        p.resolve(null);
      } else e.stopImmediatePropagation(); // les autres touches vont au champ de saisie
      return;
    }
    if (overlay.file) {
      stop();
      if (e.key === 'ArrowDown') idx = Math.min(items.length - 1, idx + 1);
      else if (e.key === 'ArrowUp') idx = Math.max(0, idx - 1);
      else if (e.key === 'Enter') activateFile();
      else if (e.key === 'Escape' || e.key === 'Backspace') goUp();
      tick().then(() => listEl?.querySelector('.sel')?.scrollIntoView({ block: 'nearest' }));
      return;
    }
    if (overlay.app) {
      if (e.key === 'ArrowDown') (stop(), (aidx = Math.min(shown.length - 1, aidx + 1)));
      else if (e.key === 'ArrowUp') (stop(), (aidx = Math.max(0, aidx - 1)));
      else if (e.key === 'Enter') (stop(), shown[aidx] && finishApp(shown[aidx]));
      else if (e.key === 'Escape') (stop(), finishApp(null));
      else e.stopImmediatePropagation();
      tick().then(() => document.querySelector('.apps .sel')?.scrollIntoView({ block: 'nearest' }));
    }
  }
</script>

<svelte:window onkeydowncapture={onKey} />

{#if overlay.confirm}
  <div class="scrim" transition:fade={{ duration: 160 }}>
    <div class="sheet" in:fly={{ y: 30, duration: 320, easing: cubicOut }}>
      <h3>{overlay.confirm.title}</h3>
      <p class="body">{overlay.confirm.text}</p>
      <div class="btns">
        <button class:sel={cidx === 0} onmouseenter={() => (cidx = 0)} onclick={() => finishConfirm(true)}>{overlay.confirm.ok}</button>
        <button class:sel={cidx === 1} onmouseenter={() => (cidx = 1)} onclick={() => finishConfirm(false)}>{overlay.confirm.cancel}</button>
      </div>
    </div>
  </div>
{/if}

{#if overlay.busy}
  <div class="scrim" transition:fade={{ duration: 160 }}>
    <div class="sheet">
      <h3>{overlay.busy.title}</h3>
      <p class="body">{overlay.busy.text}</p>
      {#if overlay.busy.pct !== null}
        <div class="bar"><i style:width="{overlay.busy.pct}%"></i></div>
      {:else}<div class="spin"></div>{/if}
    </div>
  </div>
{/if}

{#if overlay.prompt}
  <div class="scrim" transition:fade={{ duration: 160 }}>
    <div class="sheet" in:fly={{ y: 30, duration: 320, easing: cubicOut }}>
      <h3>{overlay.prompt.title}</h3>
      <input bind:this={promptInput} bind:value={overlay.prompt.value} autocomplete="off" spellcheck="false" />
      <p class="hint">{overlay.prompt.hint || t('prompt.hint')}</p>
    </div>
  </div>
{/if}

{#if overlay.file}
  <div class="scrim" transition:fade={{ duration: 160 }}>
    <div class="sheet big" in:fly={{ y: 30, duration: 320, easing: cubicOut }}>
      <h3>{overlay.file.title}</h3>
      <div class="split">
        <div class="list" bind:this={listEl}>
          {#each items as it, i (i + (it.path ?? it.label))}
            <div class="row" class:sel={i === idx}>
              <span class="ico">{it.dir ? '📁' : it.action === 'choose' ? '' : it.action === 'up' ? '' : '🖼'}</span>
              <span class="nm">{it.label}</span>
            </div>
          {:else}
            <p class="empty">{t('picker.empty')}</p>
          {/each}
        </div>
        {#if overlay.file.filter === 'image' || overlay.file.filter === 'media'}
          <div class="prev">{#if preview}<img src={preview} alt="" />{/if}</div>
        {/if}
      </div>
      {#if error}<p class="err">{error}</p>{/if}
      <p class="hint">{t('picker.hint')}</p>
    </div>
  </div>
{/if}

{#if overlay.app}
  <div class="scrim" transition:fade={{ duration: 160 }}>
    <div class="sheet big" in:fly={{ y: 30, duration: 320, easing: cubicOut }}>
      <h3>{t('apps.title')}</h3>
      <input bind:this={appInput} bind:value={filter} placeholder={t('apps.filter')} autocomplete="off" spellcheck="false" />
      <div class="split">
        <div class="list apps">
          {#if appLoading}<p class="empty">{t('apps.loading')}</p>{/if}
          {#each shown as a, i (a.id)}
            <div class="row" class:sel={i === aidx}><span class="nm">{a.name}</span></div>
          {/each}
        </div>
        <div class="prev">{#if appIcon}<img class="appico" src={`data:image/png;base64,${appIcon}`} alt="" />{/if}</div>
      </div>
      <p class="hint">{t('picker.hint')}</p>
    </div>
  </div>
{/if}

<style>
  .scrim { position: absolute; inset: 0; background: rgba(4, 6, 10, 0.78); backdrop-filter: blur(1.6rem); display: grid; place-items: center; z-index: 30; }
  .sheet { width: 36rem; padding: 2.2rem 2.4rem; border-radius: 1.8rem; background: var(--sheet); box-shadow: 0 3rem 6rem rgba(0, 0, 0, 0.6), inset 0 0 0 0.08rem var(--glass-strong); color: var(--text); }
  .sheet.big { width: 64rem; }
  h3 { margin: 0 0 1.2rem; font-size: 1.9rem; }
  input { width: 100%; padding: 1rem 1.2rem; margin-bottom: 1rem; border-radius: 0.9rem; border: 0.12rem solid var(--glass-strong); background: var(--field); color: var(--text); font-size: 1.4rem; outline: none; }
  input:focus { border-color: var(--accent); }
  .hint { margin: 1.2rem 0 0; font-size: 0.95rem; color: var(--muted); }
  .err { color: #ff8b6b; margin: 0.8rem 0 0; font-size: 1rem; }
  .split { display: flex; gap: 1.6rem; }
  .list { flex: 1; height: 26rem; overflow: hidden auto; scrollbar-width: none; }
  .row { display: flex; align-items: center; gap: 0.9rem; padding: 0.7rem 1rem; border-radius: 0.8rem; font-size: 1.25rem; }
  .row.sel { background: var(--accent); color: #fff; font-weight: 600; }
  .ico { width: 1.8rem; text-align: center; }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .prev { flex: none; width: 18rem; height: 26rem; display: grid; place-items: center; }
  .prev img { max-width: 100%; max-height: 100%; border-radius: 1rem; object-fit: contain; }
  .prev img.appico { width: 8rem; height: 8rem; }
  .empty { color: var(--muted); padding: 1rem; }
  .body { margin: 0 0 1.4rem; font-size: 1.25rem; line-height: 1.5; color: var(--muted); }
  .btns { display: flex; flex-direction: column; gap: 0.5rem; }
  .btns button { padding: 0.95rem 1.2rem; border: 0; border-radius: 0.9rem; background: transparent; color: var(--text); font: inherit; font-size: 1.3rem; text-align: left; transition: background 0.2s, transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .btns button.sel { background: var(--accent); color: #fff; font-weight: 600; transform: scale(1.02); }
  .bar { height: 0.8rem; border-radius: 99rem; background: var(--glass-strong); overflow: hidden; }
  .bar i { display: block; height: 100%; background: var(--accent); transition: width 0.3s; }
  .spin { width: 3.4rem; height: 3.4rem; margin: 0.6rem 0; border: 0.35rem solid var(--glass-strong); border-top-color: var(--accent); border-radius: 50%; animation: s 0.8s linear infinite; }
  @keyframes s { to { transform: rotate(360deg); } }
</style>
