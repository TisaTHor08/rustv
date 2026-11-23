// RusTV : aide à « Se connecter avec Google », uniquement sur accounts.google.com.
// - choisit le compte quand UN SEUL compte est proposé (sinon ne touche à rien) ;
// - valide l'écran de consentement uniquement si la demande ne porte que sur l'identité (openid, email, profile) ;
// - n'agit jamais sur une page de mot de passe et ne lit ni ne transmet aucune donnée.
(() => {
  if (location.hostname !== 'accounts.google.com') return;
  const IDENTITY = ['openid', 'email', 'profile', 'https://www.googleapis.com/auth/userinfo.email', 'https://www.googleapis.com/auth/userinfo.profile'];
  const KEY = 'rustv_google_identity_only';
  const scopesOk = (search) => {
    const s = new URLSearchParams(search).get('scope');
    const l = (s || '').split(/\s+/).filter(Boolean);
    return l.length > 0 && l.every((x) => IDENTITY.includes(x));
  };
  try {
    if (/[?&]scope=/.test(location.search)) sessionStorage.setItem(KEY, scopesOk(location.search) ? '1' : '0');
  } catch (e) {}
  const identityOnly = () => {
    try {
      return sessionStorage.getItem(KEY) === '1';
    } catch (e) {
      return false;
    }
  };
  const visible = (el) => !!(el.offsetWidth || el.offsetHeight || el.getClientRects().length);
  let done = false;
  const step = () => {
    if (done || document.querySelector('input[type=password]')) return;
    const p = location.pathname;
    if (/choose|chooser|select/i.test(p)) {
      const items = [...document.querySelectorAll('[data-identifier], [data-email]')].filter(visible);
      if (items.length === 1) {
        done = true;
        (items[0].closest('li, [role=link], [role=button]') || items[0]).click();
      }
    } else if (/\/consent/i.test(p) && identityOnly()) {
      const b = [...document.querySelectorAll('button')].filter(visible).find((x) => /^(continue|continuer|allow|autoriser)$/i.test((x.textContent || '').trim()));
      if (b) {
        done = true;
        b.click();
      }
    }
  };
  let n = 0;
  const t = setInterval(() => {
    step();
    if (done || ++n > 40) clearInterval(t);
  }, 500);
})();
