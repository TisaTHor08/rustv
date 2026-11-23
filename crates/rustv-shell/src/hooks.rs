//! Points d'accroche partagés : les modules (garde TV, Hyperion, Home Assistant…) s'abonnent aux changements du
//! mode TV sans que `tv.rs` ne les connaisse. Chaque module appelle `on_tv_mode` dans son `start`.
use std::sync::Mutex;
use tauri::AppHandle;

type TvHook = Box<dyn Fn(&AppHandle, bool) + Send + Sync>;

static TV_HOOKS: Mutex<Vec<TvHook>> = Mutex::new(Vec::new());

/// Enregistre un rappel appelé à chaque entrée (`true`) ou sortie (`false`) du mode TV.
pub fn on_tv_mode(f: impl Fn(&AppHandle, bool) + Send + Sync + 'static) {
    TV_HOOKS.lock().unwrap().push(Box::new(f));
}

/// Appelé par `tv.rs` quand le mode TV démarre ou s'arrête.
pub fn tv_mode_changed(app: &AppHandle, on: bool) {
    for h in TV_HOOKS.lock().unwrap().iter() {
        h(app, on);
    }
}
