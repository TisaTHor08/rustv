// Fenêtres modales réutilisables : saisie de texte, sélecteur de fichier, sélecteur d'application.
import type { WinApp } from './api';

export interface PromptState {
  title: string;
  value: string;
  hint: string;
  resolve: (v: string | null) => void;
}
export interface FilePickState {
  title: string;
  filter: 'image' | 'video' | 'folder' | 'media';
  resolve: (v: string | null) => void;
}
export interface AppPickState {
  resolve: (v: WinApp | null) => void;
}

export const overlay = $state<{ prompt: PromptState | null; file: FilePickState | null; app: AppPickState | null }>({
  prompt: null,
  file: null,
  app: null,
});

export const anyOverlay = () => !!(overlay.prompt || overlay.file || overlay.app);

export function askText(title: string, initial = '', hint = ''): Promise<string | null> {
  return new Promise((resolve) => {
    overlay.prompt = { title, value: initial, hint, resolve };
  });
}

export function pickFile(title: string, filter: FilePickState['filter']): Promise<string | null> {
  return new Promise((resolve) => {
    overlay.file = { title, filter, resolve };
  });
}

export function pickApp(): Promise<WinApp | null> {
  return new Promise((resolve) => {
    overlay.app = { resolve };
  });
}
