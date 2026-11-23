import { registerStrings } from './i18n';

registerStrings('fr', {
  'platform.input.uinput_denied': 'Les touches du téléphone ne peuvent pas être envoyées : /dev/uinput est inaccessible. Voir docs/LINUX.md (règle udev, module uinput, groupe input).',
  'platform.input.unsupported_platform': 'Les touches du téléphone ne sont pas prises en charge sur ce système.',
  'platform.input.other': 'Les touches du téléphone sont indisponibles ({e}).',
  'platform.unavailable': 'Indisponible sur ce système',
  'platform.input.ok': 'Touches et souris : prêtes',
});
registerStrings('en', {
  'platform.input.uinput_denied': 'Phone key presses cannot be sent: /dev/uinput is not accessible. See docs/LINUX.md (udev rule, uinput module, input group).',
  'platform.input.unsupported_platform': 'Phone key presses are not supported on this system.',
  'platform.input.other': 'Phone key presses are unavailable ({e}).',
  'platform.unavailable': 'Unavailable on this system',
  'platform.input.ok': 'Keys and mouse: ready',
});
