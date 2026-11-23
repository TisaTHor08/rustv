import type { OsStyleMeta } from '../registry';

// Style RusTV : valeurs d'origine (aucun jeton : app.css porte déjà les défauts).
const meta: OsStyleMeta = {
  id: 'rustv',
  nameKey: 'os.rustv',
  descKey: 'os.rustv.d',
  tokens: {},
  slots: ['status'],
  reducedMotionSafe: true,
};
export default meta;
