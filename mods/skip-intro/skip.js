// Skip intro: click the skip buttons as soon as they appear.
// This code runs inside the streaming site's page (as declared by the "inject" permission).
var SELECTORS = [
  '[data-uia="player-skip-intro"]', // Netflix: skip intro
  '[data-uia="player-skip-recap"]', // Netflix: skip recap
  '.atvwebplayersdk-skipelement-button', // Prime Video: skip intro / recap
];

function tick() {
  for (var i = 0; i < SELECTORS.length; i++) {
    var el = document.querySelector(SELECTORS[i]);
    if (el && el.offsetParent !== null) {
      el.click();
      return;
    }
  }
}

new MutationObserver(tick).observe(document.documentElement, { childList: true, subtree: true });
setInterval(tick, 1500);
