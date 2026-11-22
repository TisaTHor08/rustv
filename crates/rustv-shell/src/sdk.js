// RusTV mod SDK (served at <mod base>/__rustv.js). Include it in widget and page HTML:
//   <script src="__rustv.js"></script>
// Everything goes through the host over postMessage; each call is permission-checked by RusTV.
(function () {
  var pending = new Map();
  var n = 0;
  var keyHandlers = [];

  function setScheme(mode) {
    document.documentElement.style.colorScheme = mode === 'light' ? 'light' : 'dark';
  }
  setScheme('dark');

  addEventListener('message', function (e) {
    var d = e.data || {};
    if (d.t === 'key') keyHandlers.forEach(function (f) { f(d.key); });
    else if (d.t === 'theme') setScheme(d.mode);
    else if (typeof d.id === 'number' && pending.has(d.id)) {
      var p = pending.get(d.id);
      pending.delete(d.id);
      if (d.error) p.rej(new Error(d.error));
      else p.res(d.result);
    }
  });

  function call(method, params) {
    return new Promise(function (res, rej) {
      var id = ++n;
      pending.set(id, { res: res, rej: rej });
      parent.postMessage({ id: id, method: method, params: params || {} }, '*');
    });
  }

  window.rustv = {
    /** Per-mod key/value storage (permission "storage"). */
    storage: {
      get: function (key) { return call('storage.get', { key: key }); },
      set: function (key, value) { return call('storage.set', { key: key, value: value }); },
    },
    /** GET an https URL (permission "network:https://origin"). Resolves { status, body }. */
    fetch: function (url) { return call('fetch', { url: url }); },
    fetchJson: function (url) { return call('fetch', { url: url }).then(function (r) { return JSON.parse(r.body); }); },
    /** Open a website ({ url, name? }) in a RusTV window (permission "launch"). */
    launch: function (spec) { return call('launch', spec); },
    toast: function (msg) { return call('toast', { msg: msg }); },
    /** Close the page (pages only). */
    close: function () { return call('close'); },
    info: function () { return call('info'); },
    /** Remote-control keys forwarded to pages: "ArrowUp", "Enter", ... ("Escape" closes the page by itself). */
    onKey: function (f) { keyHandlers.push(f); },
  };
})();
