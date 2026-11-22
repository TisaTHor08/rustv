import { mount } from 'svelte';
import './app.css';
import Home from './lib/Home.svelte';
import Picker from './lib/Picker.svelte';
import Pairing from './lib/Pairing.svelte';
import { loadPrefs } from './lib/prefs.svelte';
import { loadMods } from './lib/mods.svelte';

const view = new URLSearchParams(location.search).get('view') ?? 'home';
document.documentElement.dataset.view = view;

const views: Record<string, typeof Home> = {
  home: Home,
  picker: Picker as unknown as typeof Home,
  pairing: Pairing as unknown as typeof Home,
};

const target = document.getElementById('app')!;
// « black » : simple écran noir pour les écrans secondaires.
if (view !== 'black') {
  await loadPrefs(); // langue, thème et accent disponibles avant le premier rendu
  await loadMods(); // thèmes, tuiles et widgets des mods activés
  mount(views[view] ?? Home, { target });
}
