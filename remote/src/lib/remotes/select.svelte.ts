// Choix du modèle : `auto` suit le style d'accueil TV annoncé par le PC, sinon un modèle fixé sur ce téléphone.
import { app } from '../connection.svelte.ts';
import { MODELS, modelById } from './registry.ts';

const STORE = 'rustv.remoteModel';

export function loadModelChoice(): string {
  try {
    const v = localStorage.getItem(STORE);
    if (v && MODELS.some((m) => m.id === v)) return v;
  } catch {}
  return 'auto';
}

export const modelPref = $state({ choice: loadModelChoice() });

export function saveModelChoice(v: string) {
  modelPref.choice = v;
  try {
    localStorage.setItem(STORE, v);
  } catch {}
}

/** Modèle effectivement affiché. */
export const activeModel = () => modelById(modelPref.choice === 'auto' ? app.osStyle : modelPref.choice);
