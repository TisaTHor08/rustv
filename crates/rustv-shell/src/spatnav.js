// RusTV site adapter: spatial navigation with the arrow keys (injected into web apps).
// The phone remote sends real arrow key presses; this script moves the focus to the nearest
// focusable element in that direction. It stays out of the way inside text fields and while
// a video is playing (arrows keep their native seek/volume behaviour).
(() => {
  if (window.__rustvNav) return;
  window.__rustvNav = true;

  const SEL =
    'a[href],button,input,select,textarea,summary,[tabindex]:not([tabindex="-1"]),' +
    '[role="button"],[role="link"],[role="menuitem"],[role="tab"],[role="option"],[role="listitem"] a';

  const addStyle = () => {
    const s = document.createElement('style');
    s.textContent =
      ':focus{outline:4px solid #fff !important;outline-offset:3px !important;border-radius:6px;' +
      'box-shadow:0 0 0 9px rgba(76,141,255,.6) !important;}';
    (document.head || document.documentElement).appendChild(s);
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', addStyle);
  else addStyle();

  const visible = (el) => {
    const r = el.getBoundingClientRect();
    if (r.width < 8 || r.height < 8) return false;
    if (r.bottom < 0 || r.right < 0 || r.top > innerHeight || r.left > innerWidth) return false;
    const cs = getComputedStyle(el);
    return cs.visibility !== 'hidden' && cs.display !== 'none' && cs.opacity !== '0';
  };

  const isTextField = (el) =>
    !!el &&
    (el.isContentEditable ||
      el.tagName === 'TEXTAREA' ||
      (el.tagName === 'INPUT' && !['button', 'checkbox', 'radio', 'submit', 'range'].includes(el.type)));

  const videoPlaying = () => [...document.querySelectorAll('video')].some((v) => !v.paused && !v.ended);

  function move(key) {
    const all = [...document.querySelectorAll(SEL)].filter(visible);
    if (!all.length) return false;
    const cur = document.activeElement && document.activeElement !== document.body ? document.activeElement : null;
    if (!cur || !visible(cur)) {
      // Premier appui : on vise l'élément le plus proche du centre de l'écran.
      const cx = innerWidth / 2, cy = innerHeight / 2;
      const c = all.reduce((a, b) => (dist(b, cx, cy) < dist(a, cx, cy) ? b : a));
      c.focus({ preventScroll: true });
      return true;
    }
    const from = cur.getBoundingClientRect();
    const fx = from.left + from.width / 2, fy = from.top + from.height / 2;
    let best = null, bestScore = Infinity;
    for (const el of all) {
      if (el === cur || cur.contains(el) || el.contains(cur)) continue;
      const r = el.getBoundingClientRect();
      const dx = r.left + r.width / 2 - fx, dy = r.top + r.height / 2 - fy;
      let main, cross;
      if (key === 'ArrowRight') { main = dx; cross = Math.abs(dy); }
      else if (key === 'ArrowLeft') { main = -dx; cross = Math.abs(dy); }
      else if (key === 'ArrowDown') { main = dy; cross = Math.abs(dx); }
      else { main = -dy; cross = Math.abs(dx); }
      if (main <= 2) continue;
      const score = main + cross * 2.5;
      if (score < bestScore) { bestScore = score; best = el; }
    }
    if (!best) return false;
    best.focus({ preventScroll: true });
    best.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'smooth' });
    return true;
  }

  const dist = (el, x, y) => {
    const r = el.getBoundingClientRect();
    return Math.hypot(r.left + r.width / 2 - x, r.top + r.height / 2 - y);
  };

  addEventListener(
    'keydown',
    (e) => {
      if (!e.isTrusted || e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
      if (!/^Arrow(Left|Right|Up|Down)$/.test(e.key)) return;
      if (isTextField(document.activeElement) || videoPlaying()) return;
      if (move(e.key)) {
        e.preventDefault();
        e.stopPropagation();
      }
    },
    true,
  );
})();
