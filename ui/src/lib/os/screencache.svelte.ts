// Cache réactif des composants d'écrans de style déjà chargés (clé « style/écran »).
import type { Component } from 'svelte';

export const screenCache = $state<{ map: Record<string, Component<any>>; failed: Record<string, boolean> }>({ map: {}, failed: {} });
