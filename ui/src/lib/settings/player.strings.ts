import { registerStrings } from '../i18n';

const fr = {
  'settings.player': 'Lecteur',
  'settings.player.d': 'Langues, reprise, épisode suivant, direct différé',
  'player.title': 'Lecteur',
  'player.audioLang': 'Langue audio préférée',
  'player.subLang': 'Langue des sous-titres',
  'player.subsOn': 'Sous-titres par défaut',
  'player.autoNext': 'Épisode suivant automatique',
  'player.resume': 'Reprendre là où on s\'est arrêté',
  'player.timeshift': 'Mémoire du direct différé',
  'player.clearHistory': 'Vider « Reprendre la lecture »',
  'player.clearTitle': 'Vider l\'historique ?',
  'player.clearText': 'La liste « Reprendre la lecture » sera effacée.',
  'player.cleared': 'Historique effacé',
  'player.on': 'Oui',
  'player.off': 'Non',
  'player.lang.auto': 'Automatique',
  'player.lang.fr': 'Français',
  'player.lang.en': 'Anglais',
  'player.lang.de': 'Allemand',
  'player.lang.es': 'Espagnol',
  'player.lang.it': 'Italien',
  'player.lang.ja': 'Japonais',
};
const en = {
  'settings.player': 'Player',
  'settings.player.d': 'Languages, resume, next episode, live timeshift',
  'player.title': 'Player',
  'player.audioLang': 'Preferred audio language',
  'player.subLang': 'Subtitle language',
  'player.subsOn': 'Subtitles on by default',
  'player.autoNext': 'Auto-play next episode',
  'player.resume': 'Resume where I stopped',
  'player.timeshift': 'Live timeshift buffer',
  'player.clearHistory': 'Clear "Continue watching"',
  'player.clearTitle': 'Clear history?',
  'player.clearText': 'The "Continue watching" list will be erased.',
  'player.cleared': 'History cleared',
  'player.on': 'Yes',
  'player.off': 'No',
  'player.lang.auto': 'Automatic',
  'player.lang.fr': 'French',
  'player.lang.en': 'English',
  'player.lang.de': 'German',
  'player.lang.es': 'Spanish',
  'player.lang.it': 'Italian',
  'player.lang.ja': 'Japanese',
};

registerStrings('fr', fr);
registerStrings('en', en);
export default { fr, en };
