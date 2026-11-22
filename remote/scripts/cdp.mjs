// Pilote une fenêtre WebView2 via le protocole de débogage (Chrome DevTools Protocol).
// Lancer RusTV avec : WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222
// Usage : node scripts/cdp.mjs <urlContient> <commande> [args...]
//   shot <fichier.png>       capture d'écran de la page
//   key <ArrowDown|Enter|…>  envoie une touche (plusieurs possibles : key A B C)
//   eval <js>                évalue du JavaScript et affiche le résultat
//   list                     liste les pages
import fs from 'node:fs';

const [, , match, cmd, ...args] = process.argv;
const port = process.env.CDP_PORT ?? '9222';
const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
if (cmd === 'list') {
  for (const t of targets) console.log(t.type, t.url);
  process.exit(0);
}
const t = targets.find((x) => x.type === 'page' && x.url.includes(match));
if (!t) {
  console.log('aucune page contenant', match);
  process.exit(1);
}
const ws = new WebSocket(t.webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let id = 0;
const pending = new Map();
ws.onmessage = (m) => {
  const d = JSON.parse(m.data);
  if (d.id && pending.has(d.id)) pending.get(d.id)(d);
};
const call = (method, params = {}) =>
  new Promise((res) => {
    const i = ++id;
    pending.set(i, res);
    ws.send(JSON.stringify({ id: i, method, params }));
  });

if (cmd === 'shot') {
  const r = await call('Page.captureScreenshot', { format: 'png' });
  fs.writeFileSync(args[0], Buffer.from(r.result.data, 'base64'));
  console.log('saved', args[0]);
} else if (cmd === 'key') {
  const codes = { ArrowDown: 40, ArrowUp: 38, ArrowLeft: 37, ArrowRight: 39, Enter: 13, Escape: 27, Backspace: 8, F3: 114, ContextMenu: 93 };
  for (const k of args) {
    const code = codes[k] ?? k.toUpperCase().charCodeAt(0);
    const base = { key: k, code: k, windowsVirtualKeyCode: code, nativeVirtualKeyCode: code };
    await call('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...base });
    await call('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
    await new Promise((r) => setTimeout(r, 350));
  }
  console.log('keys sent');
} else if (cmd === 'type') {
  await call('Input.insertText', { text: args.join(' ') });
  console.log('typed');
} else if (cmd === 'logs') {
  // Affiche les erreurs et console.log de la page pendant N ms (pendant que d'autres commandes agissent).
  await call('Runtime.enable');
  await call('Log.enable');
  ws.onmessage = (m) => {
    const d = JSON.parse(m.data);
    if (d.method === 'Runtime.exceptionThrown') console.log('EXC', d.params.exceptionDetails.exception?.description ?? d.params.exceptionDetails.text);
    else if (d.method === 'Runtime.consoleAPICalled') console.log('LOG', d.params.type, d.params.args.map((a) => a.value ?? a.description).join(' '));
    else if (d.method === 'Log.entryAdded') console.log('ENTRY', d.params.entry.level, d.params.entry.text);
  };
  await new Promise((r) => setTimeout(r, Number(args[0] ?? 4000)));
} else if (cmd === 'eval') {
  const r = await call('Runtime.evaluate', { expression: args.join(' '), returnByValue: true, awaitPromise: true });
  console.log(JSON.stringify(r.result?.result?.value ?? r.result));
}
ws.close();
process.exit(0);
