// Médiathèque locale : analyse des dossiers choisis (en arrière-plan côté Rust) et cache partagé
// entre la page Médiathèque et la recherche.
import { api } from './api';
import { prefs } from './prefs.svelte';

export const lib = $state<{ data: any | null; scanning: boolean; key: string }>({ data: null, scanning: false, key: '' });

export async function scanLibrary(force = false) {
  const L = $state.snapshot(prefs.library);
  const key = JSON.stringify(L);
  if (!force && lib.data && lib.key === key) return lib.data;
  if (!(L.movies.length || L.series.length || L.photos.length || L.music.length)) {
    lib.data = { movies: [], series: [], photos: [], music: [] };
    lib.key = key;
    return lib.data;
  }
  lib.scanning = true;
  try {
    lib.data = await api.libraryScan(L.movies, L.series, L.photos, L.music);
    lib.key = key;
  } finally {
    lib.scanning = false;
  }
  return lib.data;
}
