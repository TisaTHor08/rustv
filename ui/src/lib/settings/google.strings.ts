import { registerStrings } from '../i18n';

// Textes de la page « Compte Google » (préfixe `google.`).
const fr = {
  'settings.google': 'Compte Google',
  'settings.google.d': 'Se connecter une fois, utiliser « Se connecter avec Google » partout',
  'google.title': 'Compte Google',
  'google.enable': 'Partage du compte Google',
  'google.status': 'État',
  'google.status.off': 'Désactivé',
  'google.status.none': 'Non connecté',
  'google.status.connecting': 'Connexion en cours… (Accueil pour fermer)',
  'google.status.ok': 'Connecté',
  'google.connect': 'Connecter un compte Google',
  'google.disconnect': 'Déconnecter et effacer les cookies Google',
  'google.disconnect.q': 'Effacer la session Google de RusTV et de toutes les applications ?',
  'google.disconnect.ok': 'Déconnecter',
  'google.disconnecting': 'Déconnexion en cours…',
  'google.app': '{name}',
  'google.err.disabled': 'Activez d’abord le partage',
  'google.err.failed': 'Échec',
  'google.about':
    'Vous saisissez votre mot de passe sur la page Google elle-même (clavier du téléphone) : RusTV ne le voit ni ne le stocke. Seuls les cookies de session Google sont copiés dans les applications cochées, au lancement. Google peut refuser une session copiée : voir docs/GOOGLE-ACCOUNT.md.',
};

const en = {
  'settings.google': 'Google account',
  'settings.google.d': 'Sign in once, use “Sign in with Google” everywhere',
  'google.title': 'Google account',
  'google.enable': 'Google account sharing',
  'google.status': 'Status',
  'google.status.off': 'Off',
  'google.status.none': 'Not connected',
  'google.status.connecting': 'Signing in… (Home to close)',
  'google.status.ok': 'Connected',
  'google.connect': 'Connect a Google account',
  'google.disconnect': 'Disconnect and erase Google cookies',
  'google.disconnect.q': 'Erase the Google session from RusTV and from every app?',
  'google.disconnect.ok': 'Disconnect',
  'google.disconnecting': 'Disconnecting…',
  'google.app': '{name}',
  'google.err.disabled': 'Turn sharing on first',
  'google.err.failed': 'Failed',
  'google.about':
    'You type your password on the Google page itself (phone keyboard): RusTV never sees or stores it. Only the Google session cookies are copied into the checked apps, at launch. Google may refuse a copied session: see docs/GOOGLE-ACCOUNT.md.',
};

registerStrings('fr', fr);
registerStrings('en', en);
