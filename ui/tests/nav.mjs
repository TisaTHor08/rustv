// Tests de navigation clavier/D-pad de l'accueil, pilotés dans Chrome avec le harnais ui/mock.html.
// Prérequis : npm i --no-save playwright-core ; Chrome installé (CHROME_PATH pour un chemin autre que celui par défaut).
// Usage : cd ui && node tests/nav.mjs      (démarre lui-même le serveur vite de la maquette sur le port 5199)
import { spawn } from 'node:child_process';
import { chromium } from 'playwright-core';

const STYLES = ['rustv', 'googletv', 'tizen', 'webos', 'vidaa', 'titan', 'fire', 'tvos', 'roku'];
const CHROME = process.env.CHROME_PATH || 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const PORT = 5199;
const url = (q = '') => `http://localhost:${PORT}/mock.html?cont=1&still=1${q}`;

const srv = spawn(process.platform === 'win32' ? 'npx.cmd' : 'npx', ['vite', '--config', 'vite.mock.config.ts', '--port', String(PORT), '--strictPort'], { stdio: 'ignore', shell: process.platform === 'win32' });
const stop = () => {
  try {
    if (process.platform === 'win32') spawn('taskkill', ['/pid', String(srv.pid), '/T', '/F']);
    else srv.kill();
  } catch {}
};
process.on('exit', stop);

let fails = 0;
let checks = 0;
const ok = (cond, msg) => {
  checks++;
  if (!cond) {
    fails++;
    console.log('  ECHEC', msg);
  }
};

async function waitUp() {
  for (let i = 0; i < 60; i++) {
    try {
      if ((await fetch(url())).ok) return;
    } catch {}
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error('serveur vite injoignable');
}
await waitUp();
const browser = await chromium.launch({ executablePath: CHROME, headless: true });
const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
page.on('pageerror', (e) => {
  fails++;
  console.log('  ERREUR PAGE', e.message, String(e.stack).split('\n').slice(0, 4).join(' / '), 'dernière étape:', globalThis.__step);
});

const snap = () =>
  page.evaluate(() => {
    const m = window.__rustvHome.model;
    return { row: m.focus.row, col: m.focus.col, zone: m.zone, rows: m.rows.map((r) => r.tiles.length), style: document.documentElement.dataset.os };
  });
const press = async (k, n = 1) => {
  for (let i = 0; i < n; i++) {
    await page.keyboard.press(k);
    await page.waitForTimeout(60);
  }
};
const load = async (q) => {
  await page.goto(url(q));
  await page.waitForFunction(() => window.__rustvHome && window.__rustvHome.model.rows.length > 0 && document.documentElement.dataset.os);
  await page.waitForTimeout(500);
};
const setStyle = async (id) => {
  await page.evaluate((i) => window.__rustvHome.setStyle(i), id);
  await page.waitForFunction((i) => document.documentElement.dataset.os === i, id);
  await page.waitForTimeout(450);
};
const show = (k) => page.evaluate((x) => window.__rustvHome.show(x), k);
const zonesOf = (id) => page.evaluate(async (i) => (await import('/src/lib/os/registry.ts')).resolveStyle(i).zones ?? ['content'], id);
const winFocus = () => page.evaluate(() => window.dispatchEvent(new Event('focus')));

async function invariant(label, style) {
  const s = await snap();
  const zones = await zonesOf(style);
  ok(zones.includes(s.zone), `${label}: zone "${s.zone}" inconnue du style ${style}`);
  ok(s.row >= (style === 'rustv' ? -1 : 0) && s.row < s.rows.length, `${label}: rangée ${s.row} hors bornes (${s.rows.length} rangées)`);
  if (s.row >= 0) ok(s.col >= 0 && s.col < s.rows[s.row], `${label}: colonne ${s.col} hors bornes (${s.rows[s.row]} tuiles)`);
  return s;
}

// Gauche/Droite doivent agir : en zone contenu, sur une rangée de plus d'une tuile, Droite puis Gauche changent la colonne.
async function lrWorks(label) {
  await page.evaluate(() => {
    const m = window.__rustvHome.model;
    m.setZone('content');
    m.focusAt(0, 0, true);
  });
  await page.waitForTimeout(100);
  const a = await snap();
  if (a.rows[0] < 2) return;
  await press('ArrowRight');
  const b = await snap();
  ok(b.col === 1, `${label}: Droite sans effet (col ${a.col} -> ${b.col}, zone ${b.zone})`);
  await press('ArrowLeft');
  const c = await snap();
  ok(c.col === 0 || c.zone !== 'content', `${label}: Gauche sans effet (col ${b.col} -> ${c.col})`);
}

for (const st of STYLES) {
  console.log('style', st); globalThis.__step = st;
  await load(`&os=${st}`);
  await invariant(`${st} départ`, st);
  await lrWorks(`${st} départ`);
  for (const k of ['ArrowDown', 'ArrowRight', 'ArrowRight', 'ArrowDown', 'ArrowLeft', 'ArrowUp', 'ArrowUp', 'ArrowUp', 'ArrowLeft', 'ArrowRight', 'Escape', 'Backspace']) {
    await press(k);
    await invariant(`${st} ${k}`, st);
  }
  // Zones annexes obsolètes + retour de focus de la fenêtre
  for (const z of ['menu', 'tabs', 'nav', 'rail', 'banner', 'top', 'sys', 'ambient', 'inconnue']) {
    await page.evaluate((zz) => window.__rustvHome.model.setZone(zz), z);
    await winFocus();
    await page.waitForTimeout(80);
    const s = await snap();
    ok(s.zone === 'content', `${st}: zone "${z}" encore active après retour de focus (${s.zone})`);
    await lrWorks(`${st} après focus (${z})`);
  }
  // Rangée -1 (icône Réglages) puis retour de focus
  await page.evaluate(() => window.__rustvHome.model.focusAt(-1, 0, true));
  await page.evaluate(() => document.dispatchEvent(new Event('visibilitychange')));
  await page.waitForTimeout(80);
  await invariant(`${st} rangée -1`, st);
  // Animation de lancement : bloque les touches, le retour de focus la termine
  globalThis.__step = st + ' launch';
  await show('launch');
  await page.waitForTimeout(300);
  const before = await snap();
  await press('ArrowRight');
  const during = await snap();
  ok(before.col === during.col && before.row === during.row, `${st}: les touches passent pendant le lancement`);
  await winFocus();
  await page.waitForTimeout(300);
  await lrWorks(`${st} après lancement`);
  // Événement Tauri app-launch cancelled / timeout
  for (const phase of ['cancelled', 'timeout']) {
    await show('launch');
    await page.waitForTimeout(200);
    await page.evaluate((p) => window.__emit('app-launch', { id: 'x', phase: p }), phase);
    await page.waitForTimeout(300);
    await lrWorks(`${st} après lancement ${phase}`);
  }
  // Feuilles : ouvrir, Échap, la navigation doit rester utilisable
  for (const sh of ['settings', 'search', 'quick', 'tilemenu', 'pairing']) {
    globalThis.__step = st + ' feuille ' + sh;
    await page.evaluate(() => window.__rustvHome.model.focusAt(0, 0, true));
    await show(sh);
    await page.waitForTimeout(400);
    await press('Escape');
    await page.waitForTimeout(200);
    if (sh === 'pairing') {
      await press('Escape');
      await page.waitForTimeout(200);
    }
    await lrWorks(`${st} après feuille ${sh}`);
  }
}

// Changement de style en rafale (10 fois) avec des touches entre chaque.
console.log('rafale de changements de style');
await load('&os=rustv');
for (let i = 0; i < 10; i++) {
  const id = STYLES[(i * 4 + 1) % STYLES.length];
  await page.evaluate((x) => window.__rustvHome.setStyle(x), id);
  await page.waitForTimeout(120 + (i % 3) * 100);
  await press(['ArrowDown', 'ArrowRight', 'ArrowUp'][i % 3]);
}
await page.waitForTimeout(700);
const cur = (await snap()).style;
await invariant('après rafale', cur);
await lrWorks('après rafale');
for (const st of STYLES) {
  await setStyle(st);
  await invariant(`rafale -> ${st}`, st);
  await lrWorks(`rafale -> ${st}`);
}

// Changement de style pendant qu'une feuille est ouverte
console.log('changement de style avec feuille ouverte');
for (const sh of ['settings', 'tilemenu', 'quick']) {
  await setStyle('rustv');
  await show(sh);
  await page.waitForTimeout(300);
  await setStyle('roku');
  await page.waitForTimeout(200);
  await press('Escape');
  await page.waitForTimeout(300);
  await lrWorks(`feuille ${sh} puis style`);
}

// Zone « menu » (Roku) puis passage à un style qui ne la gère pas
console.log('zone orpheline entre styles');
await setStyle('roku');
await page.evaluate(() => window.__rustvHome.model.setZone('menu'));
await setStyle('googletv');
await invariant('roku(menu) -> googletv', 'googletv');
await lrWorks('roku(menu) -> googletv');

await browser.close();
stop();
console.log(fails ? `\n${fails} ECHEC(S) sur ${checks} vérifications` : `\nOK : ${checks} vérifications`);
process.exit(fails ? 1 : 0);
