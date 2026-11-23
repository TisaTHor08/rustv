<script lang="ts">
  import { tile } from '../actions.ts';
  import Btn from './Btn.svelte';
  import Icon from './Icon.svelte';

  // Raccourci streaming : libellé neutre « Stream n » (jamais de logo) ; ouvre la tuile du même identifiant
  // sur l'accueil du PC, sans effet si elle n'existe pas.
  let { id, n, label, cls = '' }: { id: string; n: number; label: string; cls?: string } = $props();
</script>

<Btn {cls} {label} onTap={tile(id)}>
  <span class="lbl"><Icon name="play" size={14} /><b>Stream {n}</b></span>
</Btn>

<style>
  .lbl {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    line-height: 1;
  }
  b {
    font-size: 0.66rem;
    font-weight: 600;
    letter-spacing: 0.02em;
  }
</style>
