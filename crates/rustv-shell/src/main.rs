#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod browser;
mod cast;
mod commands;
mod display;
mod dvr;
mod epg;
mod features;
mod fs;
mod guard;
mod gamepad;
mod ha;
mod hooks;
mod hyperion;
mod lan;
mod library;
#[cfg(target_os = "linux")]
#[path = "platform/linux/mod.rs"]
mod linux;
mod live;
mod media;
mod mods;
mod player;
mod prefs;
mod settings;
mod store;
mod tuner;
mod tv;
mod update;
mod usearch;
mod volume;
mod webremote;

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
    // `rustv --version` : affiche la version et quitte (aucun service démarré ; utile pour les tests de paquet).
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("rustv {}", env!("CARGO_PKG_VERSION"));
        return;
    }
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
            player::continue_list,
            player::continue_remove,
            player::continue_clear,
            player::player_pause_toggle,
            player::player_timeshift,
            player::player_go_live,
            player::player_set_track,
            player::player_menu,
            player::player_get_config,
            player::player_set_config,
            commands::live_load,
            commands::live_nownext,
            live::live_epg,
            epg::epg_get, epg::epg_set, epg::epg_refresh, epg::epg_channels, epg::epg_guide, epg::epg_search, epg::epg_tonight, epg::epg_live_nownext,
            tuner::tuner_discover,
            tuner::tuner_build_source,
            dvr::dvr_list,
            dvr::dvr_schedule,
            dvr::dvr_cancel,
            dvr::dvr_delete,
            dvr::dvr_get_config,
            dvr::dvr_set_config,
            dvr::dvr_start_now,
            dvr::dvr_series_add,
            dvr::dvr_series_remove,
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
            commands::set_os_style,
            commands::set_autostart,
            features::platform_features,
            features::linux_get,
            features::linux_set,
            commands::quit_app,
            tv::launch_cancel,
            guard::guard_get,
            guard::guard_set,
            guard::guard_status,
            guard::guard_panic,
            display::audio_outputs,
            display::audio_set_default,
            display::audio_set_output_volume,
            display::audio_cycle_output,
            display::display_info,
            display::display_set_mode,
            display::display_confirm,
            display::display_revert,
            display::display_set_hdr,
            gamepad::gamepad_get,
            gamepad::gamepad_set,
            gamepad::gamepad_test_vibration,
            ha::ha_get,
            ha::ha_set,
            ha::ha_test,
            ha::ha_publish_discovery,
            ha::ha_remove_discovery,
            cast::cast_get,
            cast::cast_set,
            hyperion::hyperion_get,
            hyperion::hyperion_set,
            hyperion::hyperion_install,
            hyperion::hyperion_start,
            hyperion::hyperion_stop,
            hyperion::hyperion_power,
            hyperion::hyperion_brightness,
            hyperion::hyperion_effect,
            hyperion::hyperion_clear,
            hyperion::hyperion_ambient,
            hyperion::hyperion_open_web_ui,
            usearch::usearch_query,
            usearch::usearch_launch,
            usearch::usearch_get_config,
            usearch::usearch_set_config,
            usearch::usearch_status,
            usearch::usearch_scan,
            usearch::usearch_scan_cancel,
            usearch::usearch_clear_cache,
            usearch::usearch_test,
            webremote::webremote_get,
            webremote::webremote_set,
            webremote::webremote_pair,
        ])
        .setup(move |app| {
            let h = app.handle().clone();
            dvr::start(app.handle());
            hyperion::start(app.handle());
            cast::start(app.handle());
            usearch::start(app.handle());
            epg::start(app.handle());
            webremote::start(app.handle());

            // Icône de la zone de notification.
            let start = MenuItem::with_id(app, "tv", "Démarrer le mode TV", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "tv_off", "Quitter le mode TV", true, None::<&str>)?;
            let pair = MenuItem::with_id(app, "pair", "Téléphones…", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quitter RusTV", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&start, &stop, &pair, &sep, &quit])?;
            let tray = TrayIconBuilder::new()
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
                        cast::cast_stop();
                        webremote::stop();
                        ha::shutdown();
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, ev| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                        open_pairing_window(tray.app_handle());
                    }
                })
                .build(app);
            // Windows : l'échec de l'icône reste fatal (comportement historique). Linux : sans hôte d'icônes
            // (GNOME sans extension) ou sans libayatana, RusTV continue et ouvre la fenêtre d'appairage.
            let mut tray_ok = true;
            if cfg!(target_os = "linux") {
                if let Err(e) = tray {
                    eprintln!("rustv: icône de notification indisponible : {e}");
                    tray_ok = false;
                }
            } else {
                tray?;
            }
            #[cfg(target_os = "linux")]
            let no_tray = !tray_ok || !linux::tray::host_available();
            #[cfg(not(target_os = "linux"))]
            let no_tray = false;
            let _ = tray_ok;

            gamepad::start(app.handle());
            ha::start(app.handle());
            volume::start(app.handle());

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

            // Garde TV : récupération après un éventuel crash, puis abonnement au mode TV.
            guard::start(app.handle());

            // `rustv --tv [--screen N]` : démarre directement en mode TV (démarrage automatique, tests).
            // `--boot` : démarrage silencieux à l'ouverture de session (attend l'écran de la TV jusqu'à 60 s,
            // pas de fenêtre d'appairage ; sans `--tv`, reprend le mode TV s'il était actif à l'arrêt).
            let args: Vec<String> = std::env::args().collect();
            let boot = args.iter().any(|a| a == "--boot");
            let screen = args.iter().position(|a| a == "--screen").and_then(|i| args.get(i + 1)).and_then(|s| s.parse::<usize>().ok());
            if args.iter().any(|a| a == "--tv") {
                match (boot, screen) {
                    (true, s) => tv::boot(app.handle(), s),
                    (false, Some(i)) => tv::choose(app.handle(), i),
                    (false, None) => tv::start(app.handle()),
                }
            } else if boot {
                if guard::resume_wanted() {
                    tv::boot(app.handle(), screen);
                }
            } else if core.cfg.lock().unwrap().devices.is_empty() || no_tray {
                // Premier lancement : aucun téléphone appairé → on ouvre la fenêtre d'appairage.
                open_pairing_window(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("erreur au démarrage de RusTV");

    app.run(|_app, ev| {
        // Quoi qu'il arrive, on rend la barre des tâches et les notifications.
        if let RunEvent::Exit = ev {
            guard::shutdown();
        }
        // Fermer la dernière fenêtre ne quitte pas RusTV : il reste dans la zone de notification.
        if let RunEvent::ExitRequested { code, api, .. } = ev {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
