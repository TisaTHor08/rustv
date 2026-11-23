// Liste des modèles de télécommande. Ajouter un modèle = un fichier <id>.svelte + une ligne ici.
import type { RemoteModel } from './types.ts';
import Rustv from './rustv.svelte';
import GoogleTv from './googletv.svelte';
import Tizen from './tizen.svelte';
import Webos from './webos.svelte';
import Vidaa from './vidaa.svelte';
import Titan from './titan.svelte';
import Fire from './fire.svelte';
import Tvos from './tvos.svelte';
import Roku from './roku.svelte';

export const MODELS: RemoteModel[] = [
  { id: 'rustv', name: { fr: 'RusTV (par défaut)', en: 'RusTV (default)' }, component: Rustv, ownsUtilities: true },
  { id: 'googletv', name: { fr: 'Google TV', en: 'Google TV' }, component: GoogleTv },
  { id: 'tizen', name: { fr: 'Tizen (Samsung)', en: 'Tizen (Samsung)' }, component: Tizen },
  { id: 'webos', name: { fr: 'webOS (LG)', en: 'webOS (LG)' }, component: Webos },
  { id: 'vidaa', name: { fr: 'VIDAA (Hisense)', en: 'VIDAA (Hisense)' }, component: Vidaa },
  { id: 'titan', name: { fr: 'Titan OS (Philips)', en: 'Titan OS (Philips)' }, component: Titan },
  { id: 'fire', name: { fr: 'Fire TV (Amazon)', en: 'Fire TV (Amazon)' }, component: Fire },
  { id: 'tvos', name: { fr: 'tvOS (Apple TV)', en: 'tvOS (Apple TV)' }, component: Tvos },
  { id: 'roku', name: { fr: 'Roku', en: 'Roku' }, component: Roku },
];

/** Modèle d'un identifiant ; `rustv` si inconnu (PC plus récent que le téléphone, par exemple). */
export const modelById = (id: string): RemoteModel => MODELS.find((m) => m.id === id) ?? MODELS[0];
