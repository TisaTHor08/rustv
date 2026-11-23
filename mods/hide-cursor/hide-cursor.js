// Hide cursor: hide the mouse pointer after 3 s without movement, show it again on movement.
// Runs inside every web page. Touches only: one <style> element it creates and mouse listeners.
(function () {
  if (window.__rustvHideCursor) return;
  window.__rustvHideCursor = true;
  var style = document.createElement('style');
  style.textContent = '.rustv-idle, .rustv-idle * { cursor: none !important; }';
  (document.head || document.documentElement).appendChild(style);
  var root = document.documentElement, timer = 0;
  function show() {
    root.classList.remove('rustv-idle');
    clearTimeout(timer);
    timer = setTimeout(function () { root.classList.add('rustv-idle'); }, 3000);
  }
  ['mousemove', 'mousedown', 'wheel'].forEach(function (e) {
    window.addEventListener(e, show, { passive: true, capture: true });
  });
  show();
})();
