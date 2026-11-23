#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod browser;
mod commands;
mod display;
mod fs;
mod hooks;
mod lan;
mod library;
mod live;
mod media;
mod mods;
mod player;
mod prefs;
mod settings;
mod store;
mod tv;
mod update;

use rustv_core::{Core, Event};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

#[derive(Default)]
pub struct TvState {
    pub active: bool,
    pub monitor: usize,
}

pub struct Shell {
    pub media: Arc<media::Media>,
    pub player: Arc<player::Player>,
    pub live: Mutex<live::Cache>,
    pub core: Arc<Core>,
    pub settings: Mutex<settings::Settings>,
    pub tv: Mutex<TvState>,
}

fn open_pairing_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("pairing") {
        let _ = w.show();
        tv::force_focus(&w);
        return;
    }
    if let Ok(w) = WebviewWindowBuilder::new(app, "pairing", WebviewUrl::App("index.html?view=pairing".into()))
        .title("RusTV — Téléphones")
        .inner_size(560.0, 760.0)
        .center()
        .resizable(false)
        .build()
    {
        tv::force_focus(&w);
    }
}

fn main() {
    let core = Arc::new(Core::new(rustv_core::config::Config::load()));
    // Le port est déjà pris : une autre instance de RusTV tourne.
    if rustv_core::start(core.clone()).is_err() {
        return;
    }

    let app = tauri::Builder::default()
        .manage(Shell { media: media::Media::start(), player: Arc::new(player::Player::default()), live: Mutex::new(live::Cache::default()), core: core.clone(), settings: Mutex::new(settings::Settings::load()), tv: Mutex::new(TvState::default()) })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::tv_start,
            commands::tv_exit,
            commands::picker_move,
            commands::picker_choose,
            commands::launch_tile,
            commands::launch_custom,
            commands::prefs_get,
            commands::prefs_set,
            commands::media_url,
            commands::folder_images,
            commands::vault_status,
            commands::vault_lock,
            commands::vault_logout,
            commands::vault_set_autolock,
            commands::vault_set_server,
            commands::fs_roots,
            commands::fs_list,
            commands::windows_apps,
            commands::app_icon,
            commands::update_check,
            commands::update_install,
            commands::player_status,
            commands::player_install,
            commands::player_play,
            commands::player_stop,
            commands::player_state,
            commands::live_load,
            commands::live_nownext,
            commands::library_scan,
            commands::open_path,
            commands::mods_list,
            commands::mod_set_enabled,
            commands::mod_uninstall,
            commands::mod_text,
            commands::mod_base_url,
            commands::mod_storage_get,
            commands::mod_storage_set,
            commands::mod_fetch,
            commands::registry_fetch,
            commands::mod_install_prepare,
            commands::mod_install_commit,
            commands::mod_install_cancel,
            commands::mods_open_folder,
            commands::go_home,
            commands::browsers_detected,
            commands::pairing_start,
            commands::pairing_cancel,
            commands::device_revoke,
            commands::set_blackout,
            commands::set_autostart,
            commands::quit_app,
            display::audio_outputs,
            display::audio_set_default,
            display::audio_set_output_volume,
            display::audio_cycle_output,
            display::display_info,
            display::display_set_mode,
            display::display_confirm,
            display::display_revert,
            display::display_set_hdr,
        ])
        .setup(move |app| {
            let h = app.handle().clone();

            // Icône de la zone de notification.
            let start = MenuItem::with_id(app, "tv", "Démarrer le mode TV", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "tv_off", "Quitter le mode TV", true, None::<&str>)?;
            let pair = MenuItem::with_id(app, "pair", "Téléphones…", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quitter RusTV", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&start, &stop, &pair, &sep, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("RusTV")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, ev| match ev.id.as_ref() {
                    "tv" => tv::start(app),
                    "tv_off" => tv::exit(app),
                    "pair" => open_pairing_window(app),
                    "quit" => {
                        tv::exit(app);
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, ev| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                        open_pairing_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // Contributions des mods à la télécommande du téléphone.
            core.set_remote_mods(mods::remote_bundle());

            // Commandes du téléphone → interface.
            let rx = core.subscribe();
            std::thread::spawn(move || {
                for ev in rx {
                    match ev {
                        Event::Home => tv::go_home(&h),
                        Event::Tv(true) => tv::start(&h),
                        Event::Tv(false) => tv::exit(&h),
                        Event::OpenTile(id) => {
                            let _ = h.emit("open-tile", id);
                        }
                        Event::DevicesChanged => {
                            let _ = h.emit("devices-changed", ());
                        }
                    }
                }
            });

            // `rustv --tv [--screen N]` : démarre directement en mode TV (démarrage automatique, tests).
            let args: Vec<String> = std::env::args().collect();
            if args.iter().any(|a| a == "--tv") {
                let screen = args.iter().position(|a| a == "--screen").and_then(|i| args.get(i + 1)).and_then(|s| s.parse::<usize>().ok());
                match screen {
                    Some(i) => tv::choose(app.handle(), i),
                    None => tv::start(app.handle()),
                }
            } else if core.cfg.lock().unwrap().devices.is_empty() {
                // Premier lancement : aucun téléphone appairé → on ouvre la fenêtre d'appairage.
                open_pairing_window(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("erreur au démarrage de RusTV");

    app.run(|_app, ev| {
        // Fermer la dernière fenêtre ne quitte pas RusTV : il reste dans la zone de notification.
        if let RunEvent::ExitRequested { code, api, .. } = ev {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
