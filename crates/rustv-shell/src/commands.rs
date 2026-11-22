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
