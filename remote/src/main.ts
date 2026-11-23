import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { consumeHashPairing } from './lib/connection.svelte.ts'
import { isNative } from './lib/native.ts'

// Lien de jumelage scanné ou ouvert (`#p=…`) : enregistré avant l'affichage, puis effacé de la barre d'adresse.
consumeHashPairing()

// Navigateur (pas l'appli Capacitor) : service worker pour garder la coquille hors ligne. Il n'existe qu'en contexte
// sécurisé (HTTPS ou localhost) ; en HTTP sur le LAN la télécommande fonctionne sans, simplement non installable.
if (!isNative && 'serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('sw.js').catch(() => {})
  })
}

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
