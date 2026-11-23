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

  // Dans un champ sur une seule ligne, Haut/Bas n'ont pas de sens natif : on les laisse naviguer.
  // Gauche/Droite ne naviguent que si le curseur est au bord. Zones de texte multi-lignes : comportement natif.
  const textBlocks = (el, key) => {
    if (!isTextField(el)) return false;
    if (el.tagName !== 'INPUT') return true;
    if (key === 'ArrowUp' || key === 'ArrowDown') return false;
    try {
      if (el.selectionStart !== el.selectionEnd) return true;
      return key === 'ArrowLeft' ? el.selectionStart > 0 : el.selectionStart < el.value.length;
    } catch {
      return false; // types sans sélection (email, number…)
    }
  };

  const videoPlaying = () => [...document.querySelectorAll('video')].some((v) => !v.paused && !v.ended);

  // Interfaces web « bureau » (NAS, tableaux de bord…) : beaucoup d'éléments cliquables sont de simples <div>.
  // On les reconnaît au curseur « main » et on retient le plus externe de chaque groupe.
  const CLICKABLE = 'div,li,span,img,figure,svg,i,p,label';
  function pointerTargets() {
    const out = [];
    for (const el of document.querySelectorAll(CLICKABLE)) {
      if (el.matches(SEL) || el.closest('a[href],button,input,select,textarea,[role="button"]')) continue;
      const p = el.parentElement;
      if (!p || getComputedStyle(el).cursor !== 'pointer' || getComputedStyle(p).cursor === 'pointer') continue;
      if (!visible(el)) continue;
      const r = el.getBoundingClientRect();
      if (r.width > innerWidth * 0.9 && r.height > innerHeight * 0.6) continue; // fond plein écran
      el.dataset.rustvNav = '1';
      if (!el.hasAttribute('tabindex')) el.tabIndex = -1;
      out.push(el);
    }
    return out;
  }

  function move(key) {
    const all = [...document.querySelectorAll(SEL)].filter(visible).concat(pointerTargets());
    if (!all.length) return false;
    let cur = document.activeElement && document.activeElement !== document.body ? document.activeElement : null;
    // Case à cocher ou bouton radio masqués par le site : on se repère sur leur conteneur visible (l'étiquette).
    if (cur && !visible(cur)) {
      let p = cur.parentElement;
      while (p && p !== document.body && !visible(p)) p = p.parentElement;
      if (p && p !== document.body) cur = p;
    }
    if (!cur || !visible(cur)) {
      // Premier appui : on vise l'élément le plus proche du centre de l'écran.
      const cx = innerWidth / 2, cy = innerHeight / 2;
      const c = all.reduce((a, b) => (dist(b, cx, cy) < dist(a, cx, cy) ? b : a));
      c.focus({ preventScroll: true });
      return true;
    }
    const from = cur.getBoundingClientRect();
    const fx = from.left + from.width / 2, fy = from.top + from.height / 2;
    const ranked = [];
    for (const el of all) {
      if (el === cur || cur.contains(el) || el.contains(cur)) continue;
      if (el.disabled || el.getAttribute('aria-disabled') === 'true') continue;
      const r = el.getBoundingClientRect();
      const dx = r.left + r.width / 2 - fx, dy = r.top + r.height / 2 - fy;
      let main, cross;
      if (key === 'ArrowRight') { main = dx; cross = Math.abs(dy); }
      else if (key === 'ArrowLeft') { main = -dx; cross = Math.abs(dy); }
      else if (key === 'ArrowDown') { main = dy; cross = Math.abs(dx); }
      else { main = -dy; cross = Math.abs(dx); }
      if (main <= 2) continue;
      ranked.push([main + cross * 2.5, el]);
    }
    ranked.sort((a, b) => a[0] - b[0]);
    // Certains éléments refusent le focus (désactivés par le site, masqués…) : on essaie le suivant.
    for (const [, el] of ranked.slice(0, 6)) {
      el.focus({ preventScroll: true });
      if (document.activeElement === el) {
        el.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'smooth' });
        return true;
      }
    }
    return false;
  }

  const dist = (el, x, y) => {
    const r = el.getBoundingClientRect();
    return Math.hypot(r.left + r.width / 2 - x, r.top + r.height / 2 - y);
  };

  addEventListener(
    'keydown',
    (e) => {
      const a = document.activeElement;
      if (e.isTrusted && e.key === 'Enter' && a && a.dataset && a.dataset.rustvNav) {
        e.preventDefault();
        a.click();
      }
    },
    true,
  );

  addEventListener(
    'keydown',
    (e) => {
      if (!e.isTrusted || e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
      if (!/^Arrow(Left|Right|Up|Down)$/.test(e.key)) return;
      if (textBlocks(document.activeElement, e.key) || videoPlaying()) return;
      if (move(e.key)) {
        e.preventDefault();
        e.stopPropagation();
      }
    },
    true,
  );
})();
