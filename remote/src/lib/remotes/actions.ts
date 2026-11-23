// Correspondance bouton -> message du protocole, commune à tous les modèles.
import { send } from '../connection.svelte.ts';
import type { KeyName, MediaAction } from '../protocol/types.ts';

export const key = (k: KeyName) => () => send({ t: 'key', k });
export const media = (a: MediaAction) => () => send({ t: 'media', a });
export const home = () => send({ t: 'home' });
export const search = () => send({ t: 'key', k: 'search' });
export const menu = () => send({ t: 'key', k: 'menu' });
/** Ouvre une tuile de l'accueil du PC ; sans effet si elle n'existe pas côté PC. */
export const tile = (id: string) => () => send({ t: 'openTile', id });
