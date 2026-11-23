// Fenêtres modales réutilisables : saisie de texte, sélecteur de fichier, sélecteur d'application,
// confirmation et indicateur de progression.
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
export interface ConfirmState {
  title: string;
  text: string;
  ok: string;
  cancel: string;
  resolve: (v: boolean) => void;
}
export interface QrState {
  title: string;
  /** SVG du QR code (fourni par le shell). */
  svg: string;
  text: string;
  close: string;
  resolve: () => void;
}
export interface BusyState {
  title: string;
  text: string;
  pct: number | null;
}

export const overlay = $state<{
  prompt: PromptState | null;
  file: FilePickState | null;
  app: AppPickState | null;
  confirm: ConfirmState | null;
  qr: QrState | null;
  busy: BusyState | null;
}>({ prompt: null, file: null, app: null, confirm: null, qr: null, busy: null });

export const anyOverlay = () => !!(overlay.prompt || overlay.file || overlay.app || overlay.confirm || overlay.qr || overlay.busy);

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

export function askConfirm(title: string, text: string, ok: string, cancel: string): Promise<boolean> {
  return new Promise((resolve) => {
    overlay.confirm = { title, text, ok, cancel, resolve };
  });
}

/** Affiche un QR code (SVG) et un texte ; se résout à la fermeture (Entrée, Échap ou clic). */
export function showQr(title: string, svg: string, text: string, close: string): Promise<void> {
  return new Promise((resolve) => {
    overlay.qr = { title, svg, text, close, resolve };
  });
}
