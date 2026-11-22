//! Commandes appelées par l'interface (invoke).
use crate::{settings, tv, Shell};
use qrcode::{render::svg, QrCode};
use serde_json::{json, Value};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn get_state(app: AppHandle, shell: State<'_, Shell>) -> Result<Value, String> {
    let s = shell.settings.lock().unwrap().clone();
    Ok(json!({
        "tv": shell.tv.lock().unwrap().active,
        "monitors": tv::monitors(&app),
        "settings": s,
        "catalog": tv::catalog(),
        "devices": shell.core.devices(),
        "pcName": shell.core.cfg.lock().unwrap().pc_name,
        "autostart": crate::autostart::get(),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[tauri::command]
pub async fn tv_start(app: AppHandle) {
    tv::start(&app);
}

#[tauri::command]
pub async fn tv_exit(app: AppHandle) {
    tv::exit(&app);
}

#[tauri::command]
pub async fn picker_move(app: AppHandle, dir: i32) {
    tv::picker_move(&app, dir);
}

#[tauri::command]
pub async fn picker_choose(app: AppHandle, index: usize) {
    tv::choose(&app, index);
}

#[tauri::command]
pub async fn launch_tile(app: AppHandle, id: String, url: Option<String>) -> Result<String, String> {
    tv::launch(&app, &id, url)
}

/// Lance une tuile personnalisée (site web ou application Windows) décrite par l'interface.
#[tauri::command]
pub async fn launch_custom(app: AppHandle, tile: Value) -> Result<String, String> {
    tv::launch_value(&app, &tile, None)
}

#[tauri::command]
pub async fn prefs_get() -> Result<Value, String> {
    Ok(crate::prefs::get())
}

#[tauri::command]
pub async fn prefs_set(prefs: Value) -> Result<(), String> {
    crate::prefs::set(&prefs)
}

/// Enregistre un fichier local et renvoie une URL http://127.0.0.1 secrète utilisable dans <img>/<video>.
#[tauri::command]
pub async fn media_url(shell: State<'_, Shell>, path: String) -> Result<String, String> {
    shell.media.url(std::path::Path::new(&path)).ok_or_else(|| "Fichier introuvable".to_string())
}

/// URL des images d'un dossier (diaporama).
#[tauri::command]
pub async fn folder_images(shell: State<'_, Shell>, path: String) -> Result<Vec<String>, String> {
    Ok(crate::fs::images_in(&path, 200).iter().filter_map(|p| shell.media.url(std::path::Path::new(p))).collect())
}

#[tauri::command]
pub async fn fs_roots() -> Result<Vec<crate::fs::Root>, String> {
    Ok(crate::fs::roots())
}

#[tauri::command]
pub async fn fs_list(path: String, filter: String) -> Result<crate::fs::Listing, String> {
    crate::fs::list(&path, &filter)
}

/// Applications du menu Démarrer (pour créer une tuile personnalisée).
#[tauri::command]
pub async fn windows_apps() -> Result<Value, String> {
    Ok(json!(rustv_core::apps::list(&[])
        .into_iter()
        .filter(|a| a.id.starts_with("s:"))
        .map(|a| json!({ "id": a.id, "name": a.name, "target": a.target }))
        .collect::<Vec<_>>()))
}

/// Icône PNG (base64) d'un raccourci Windows.
#[tauri::command]
pub async fn app_icon(target: String) -> Result<Option<String>, String> {
    rustv_core::icons::com_init();
    Ok(rustv_core::icons::icon_png_base64(&target))
}

// ---------- mods ----------

#[tauri::command]
pub async fn mods_list() -> Result<Value, String> {
    Ok(Value::Array(crate::mods::list()))
}

#[tauri::command]
pub async fn mod_set_enabled(id: String, enabled: bool) -> Result<(), String> {
    crate::mods::set_enabled(&id, enabled)
}

#[tauri::command]
pub async fn mod_uninstall(id: String) -> Result<(), String> {
    crate::mods::uninstall(&id)
}

#[tauri::command]
pub async fn mod_text(id: String, rel: String) -> Result<String, String> {
    crate::mods::text(&id, &rel)
}

/// URL de base du dossier d'un mod (pour les iframes de widgets et de pages, et ses icônes).
#[tauri::command]
pub async fn mod_base_url(shell: State<'_, Shell>, id: String) -> Result<String, String> {
    if !crate::mods::valid_id(&id) {
        return Err("id invalide".into());
    }
    shell.media.mod_base(&crate::mods::root().join(&id)).ok_or_else(|| "mod introuvable".to_string())
}

#[tauri::command]
pub async fn mod_storage_get(id: String, key: String) -> Result<Value, String> {
    crate::mods::storage_get(&id, &key)
}

#[tauri::command]
pub async fn mod_storage_set(id: String, key: String, value: Value) -> Result<(), String> {
    crate::mods::storage_set(&id, &key, value)
}

#[tauri::command]
pub async fn mod_fetch(id: String, url: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || crate::mods::fetch(&id, &url)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn registry_fetch() -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(crate::mods::registry).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mod_install_prepare(entry: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || crate::mods::install_prepare(&entry)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mod_install_commit(id: String) -> Result<(), String> {
    crate::mods::install_commit(&id)
}

#[tauri::command]
pub async fn mod_install_cancel(id: String) -> Result<(), String> {
    crate::mods::install_cancel(&id);
    Ok(())
}

/// Ouvre le dossier des mods dans l'explorateur (pour installer un mod à la main).
#[tauri::command]
pub async fn mods_open_folder() -> Result<(), String> {
    let d = crate::mods::root();
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    rustv_core::apps::launch(&d.to_string_lossy());
    Ok(())
}

#[tauri::command]
pub async fn go_home(app: AppHandle) {
    tv::go_home(&app);
}

#[tauri::command]
pub async fn pairing_start(shell: State<'_, Shell>) -> Result<Value, String> {
    let (info, device) = shell.core.new_pairing();
    let svg = QrCode::new(info.to_qr_string().as_bytes())
        .map_err(|e| e.to_string())?
        .render::<svg::Color>()
        .min_dimensions(320, 320)
        .quiet_zone(true)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();
    Ok(json!({ "svg": svg, "device": device, "name": info.name, "bluetooth": info.bt.is_some() }))
}

#[tauri::command]
pub async fn pairing_cancel(shell: State<'_, Shell>, device: String) -> Result<(), String> {
    shell.core.cancel_pairing(&device);
    Ok(())
}

#[tauri::command]
pub async fn device_revoke(shell: State<'_, Shell>, id: String) -> Result<(), String> {
    shell.core.revoke_device(&id);
    Ok(())
}

#[tauri::command]
pub async fn set_blackout(shell: State<'_, Shell>, on: bool) -> Result<(), String> {
    let mut s = shell.settings.lock().unwrap();
    s.blackout = on;
    s.save();
    Ok(())
}

#[tauri::command]
pub async fn set_autostart(mode: String) -> Result<(), String> {
    if !["off", "tray", "tv"].contains(&mode.as_str()) {
        return Err("mode invalide".into());
    }
    crate::autostart::set(&mode);
    Ok(())
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) {
    tv::exit(&app);
    let _ = settings::dir(); // (réservé : nettoyage futur)
    app.exit(0);
}
