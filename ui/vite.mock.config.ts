// Harnais de développement uniquement (jamais dans le build de production) :
//   npx vite --config vite.mock.config.ts --port 5199   puis   http://localhost:5199/mock.html?os=roku&keys=ArrowDown,Enter
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  base: './',
  plugins: [svelte()],
  server: { fs: { strict: false } },
});
