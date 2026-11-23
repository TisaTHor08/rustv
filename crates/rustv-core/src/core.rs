//! État partagé et traitement des commandes du téléphone.
use crate::{apps, audio, config::Config, icons, input};
use rustv_protocol::messages::*;
use rustv_protocol::pairing::PairingInfo;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::input::keys::*;
use crate::input::HWND;

/// Relâche Alt automatiquement si le téléphone ne confirme pas.
const ALT_SAFETY: Duration = Duration::from_secs(5);
const ICON_BATCH: usize = 25;

struct Conn {
    id: u64,
    device: String,
    tx: Sender<PcMsg>,
}

/// Styles d'accueil TV reconnus (identiques à ceux de l'interface TV).
pub const OS_STYLES: [&str; 9] = ["rustv", "googletv", "tizen", "webos", "vidaa", "titan", "fire", "tvos", "roku"];

/// Événements destinés au shell TV (interface), émis par les commandes du téléphone.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Retour à l'accueil TV.
    Home,
    /// Démarrer / arrêter le mode TV.
    Tv(bool),
    /// La liste des appareils ou leur état de connexion a changé.
    DevicesChanged,
    /// Le téléphone demande d'ouvrir une tuile de l'accueil.
    OpenTile(String),
}

/// Média en cours (SMTC ou lecteur RusTV).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
    pub source: String,
}

/// État du lecteur RusTV (mpv), fourni par le shell.
#[derive(Debug, Clone, Default)]
pub struct PlayerInfo {
    pub title: String,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub used: bool,
    pub connected: bool,
}

pub struct Core {
    pub cfg: Mutex<Config>,
    pub bt_addr: Mutex<Option<String>>,
    events: Mutex<Vec<Sender<Event>>>,
    tv_mode: Mutex<bool>,
    os_style: Mutex<Option<String>>,
    player: Mutex<Option<PlayerInfo>>,
    smtc: Mutex<Option<NowPlaying>>,
    last_np: Mutex<Option<PcMsg>>,
    volume: Mutex<Option<(u8, bool)>>,
    remote_mods: Mutex<serde_json::Value>,
    alt: Mutex<Option<Instant>>,   // Some(dernier événement) quand Alt est maintenu
    pinned: Mutex<Option<isize>>,  // HWND épinglé
    conns: Mutex<Vec<Conn>>,
    next_conn: Mutex<u64>,
    text_focus: Mutex<bool>,
    apps: Mutex<Vec<apps::App>>,
    icons: Mutex<HashMap<String, String>>,
    /// Coffre Bitwarden (CLI `bw`) : voir `vault.rs`.
    pub vault: Arc<crate::vault::Vault>,
    typer: Arc<crate::vault::WinTyper>,
    vault_ver: AtomicU64,
}

impl Core {
    pub fn new(cfg: Config) -> Self {
        Self::with_vault(cfg, Arc::new(crate::vault::Vault::system()))
    }

    pub fn with_vault(cfg: Config, vault: Arc<crate::vault::Vault>) -> Self {
        Self {
            cfg: Mutex::new(cfg),
            bt_addr: Mutex::new(None),
            events: Mutex::new(vec![]),
            tv_mode: Mutex::new(false),
            os_style: Mutex::new(None),
            player: Mutex::new(None),
            smtc: Mutex::new(None),
            last_np: Mutex::new(None),
            volume: Mutex::new(None),
            remote_mods: Mutex::new(serde_json::Value::Array(vec![])),
            alt: Mutex::new(None),
            pinned: Mutex::new(None),
            conns: Mutex::new(vec![]),
            next_conn: Mutex::new(0),
            text_focus: Mutex::new(false),
            apps: Mutex::new(vec![]),
            icons: Mutex::new(HashMap::new()),
            vault,
            typer: Arc::new(crate::vault::WinTyper { focus: Arc::new(AtomicBool::new(false)) }),
            vault_ver: AtomicU64::new(0),
        }
    }

    // ---- événements vers le shell ----

    pub fn subscribe(&self) -> Receiver<Event> {
        let (tx, rx) = channel();
        self.events.lock().unwrap().push(tx);
        rx
    }

    fn emit(&self, e: Event) {
        self.events.lock().unwrap().retain(|s| s.send(e.clone()).is_ok());
    }

    /// Le shell signale que le mode TV est actif ou non (le téléphone adapte son interface).
    pub fn set_tv_mode(&self, on: bool) {
        let mut t = self.tv_mode.lock().unwrap();
        if *t != on {
            *t = on;
            drop(t);
            self.broadcast(self.state_msg());
        }
    }

    /// Style d'accueil TV choisi : le téléphone adopte la télécommande correspondante.
    /// Refuse un identifiant inconnu ; ne rediffuse rien si le style ne change pas.
    pub fn set_os_style(&self, id: &str) -> Result<(), String> {
        if !OS_STYLES.contains(&id) {
            return Err("unknown os style".into());
        }
        let new = if id == "rustv" { None } else { Some(id.to_string()) };
        let mut cur = self.os_style.lock().unwrap();
        if *cur != new {
            *cur = new;
            drop(cur);
            self.broadcast(self.state_msg());
        }
        Ok(())
    }

    // ---- média en cours ----

    pub fn set_player_info(&self, p: Option<PlayerInfo>) {
        *self.player.lock().unwrap() = p;
        self.refresh_np();
    }

    pub fn set_smtc(&self, s: Option<NowPlaying>) {
        *self.smtc.lock().unwrap() = s;
        self.refresh_np();
    }

    fn np_msg(&self) -> PcMsg {
        let cur = match self.player.lock().unwrap().clone() {
            // Le lecteur RusTV a la priorité sur les autres sources.
            Some(p) => Some(NowPlaying { title: p.title, artist: String::new(), playing: p.playing, position: p.position, duration: p.duration, source: "RusTV".into() }),
            None => self.smtc.lock().unwrap().clone(),
        };
        match cur {
            Some(n) => PcMsg::NowPlaying { active: true, title: n.title, artist: n.artist, playing: n.playing, position: n.position, duration: n.duration, source: n.source },
            None => PcMsg::NowPlaying { active: false, title: String::new(), artist: String::new(), playing: false, position: 0.0, duration: 0.0, source: String::new() },
        }
    }

    /// Le shell publie les contributions des mods actifs pour la télécommande ; elles sont envoyées aux téléphones connectés.
    pub fn set_remote_mods(&self, mods: serde_json::Value) {
        let mut cur = self.remote_mods.lock().unwrap();
        if *cur != mods {
            *cur = mods.clone();
            drop(cur);
            self.broadcast(PcMsg::RemoteMods { mods });
        }
    }

    /// Mémorise le volume du PC et le diffuse s'il a changé.
    pub fn set_volume(&self, level: u8, muted: bool) {
        let mut v = self.volume.lock().unwrap();
        if *v != Some((level, muted)) {
            *v = Some((level, muted));
            drop(v);
            self.broadcast(PcMsg::Volume { level, muted });
        }
    }

    /// Média en cours (lecteur RusTV ou SMTC), `None` si rien ne joue.
    pub fn now_playing(&self) -> Option<NowPlaying> {
        match self.np_msg() {
            PcMsg::NowPlaying { active: true, title, artist, playing, position, duration, source } => Some(NowPlaying { title, artist, playing, position, duration, source }),
            _ => None,
        }
    }

    /// Dernier volume connu du PC (niveau 0 à 100, muet).
    pub fn volume(&self) -> Option<(u8, bool)> {
        *self.volume.lock().unwrap()
    }

    fn refresh_np(&self) {
        let msg = self.np_msg();
        let mut last = self.last_np.lock().unwrap();
        if last.as_ref() != Some(&msg) {
            *last = Some(msg.clone());
            drop(last);
            self.broadcast(msg);
        }
    }

    // ---- appairage ----

    /// Crée un nouvel appairage (clé fraîche) et renvoie le contenu du QR code + l'id de l'appareil.
    pub fn new_pairing(&self) -> (PairingInfo, String) {
        let bt = self.bt_addr.lock().unwrap().clone();
        let mut cfg = self.cfg.lock().unwrap();
        let dev = cfg.new_device();
        let info = PairingInfo {
            v: rustv_protocol::PROTOCOL_VERSION,
            id: cfg.pc_id.clone(),
            name: cfg.pc_name.clone(),
            device: dev.id.clone(),
            key: dev.key,
            ips: crate::server::local_ips(),
            port: rustv_protocol::TCP_PORT,
            bt,
        };
        drop(cfg);
        self.emit(Event::DevicesChanged);
        (info, dev.id)
    }

    /// Annule un appairage dont le QR n'a pas été scanné.
    pub fn cancel_pairing(&self, device: &str) {
        let mut cfg = self.cfg.lock().unwrap();
        if cfg.devices.iter().any(|d| d.id == device && !d.used) {
            cfg.revoke(device);
        }
        drop(cfg);
        self.emit(Event::DevicesChanged);
    }

    pub fn revoke_device(&self, device: &str) {
        self.cfg.lock().unwrap().revoke(device);
        self.kick_device(device);
        self.emit(Event::DevicesChanged);
    }

    pub fn devices(&self) -> Vec<DeviceInfo> {
        let connected = self.connected_devices();
        self.cfg
            .lock()
            .unwrap()
            .devices
            .iter()
            .map(|d| DeviceInfo { id: d.id.clone(), name: d.name.clone(), used: d.used, connected: connected.contains(&d.id) })
            .collect()
    }

    // ---- connexions ----

    pub fn add_conn(self: &std::sync::Arc<Self>, tx: Sender<PcMsg>, device: &str) -> u64 {
        self.cfg.lock().unwrap().mark_used(device);
        let mut n = self.next_conn.lock().unwrap();
        *n += 1;
        let id = *n;
        drop(n);
        self.conns.lock().unwrap().push(Conn { id, device: device.to_string(), tx: tx.clone() });
        let _ = tx.send(self.state_msg());
        let _ = tx.send(self.np_msg());
        if let Some((level, muted)) = *self.volume.lock().unwrap() {
            let _ = tx.send(PcMsg::Volume { level, muted });
        }
        let mods = self.remote_mods.lock().unwrap().clone();
        if mods.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
            let _ = tx.send(PcMsg::RemoteMods { mods });
        }
        self.refresh_apps();
        let _ = tx.send(self.apps_msg());
        self.stream_icons(tx);
        self.emit(Event::DevicesChanged);
        id
    }

    pub fn remove_conn(&self, id: u64) {
        let mut c = self.conns.lock().unwrap();
        c.retain(|x| x.id != id);
        let empty = c.is_empty();
        drop(c);
        if empty {
            self.release_alt(); // plus de téléphone : ne jamais laisser Alt coincé
            self.set_text_focus(false);
            self.vault.lock_async(); // plus de téléphone : le coffre se verrouille
        }
        self.emit(Event::DevicesChanged);
    }

    pub fn conn_count(&self) -> usize {
        self.conns.lock().unwrap().len()
    }

    pub fn connected_devices(&self) -> Vec<String> {
        self.conns.lock().unwrap().iter().map(|c| c.device.clone()).collect()
    }

    /// Coupe les connexions d'un appareil révoqué (la fermeture du canal ferme le socket).
    pub fn kick_device(&self, device: &str) {
        self.conns.lock().unwrap().retain(|c| c.device != device);
    }

    fn broadcast(&self, m: PcMsg) {
        for c in self.conns.lock().unwrap().iter() {
            let _ = c.tx.send(m.clone());
        }
    }

    fn state_msg(&self) -> PcMsg {
        PcMsg::State {
            pinned: self.pinned.lock().unwrap().is_some(),
            text_focus: *self.text_focus.lock().unwrap(),
            tv_mode: *self.tv_mode.lock().unwrap(),
            os_style: self.os_style.lock().unwrap().clone(),
        }
    }

    // ---- apps & icônes ----

    fn refresh_apps(&self) {
        let custom = self.cfg.lock().unwrap().custom_apps.clone();
        *self.apps.lock().unwrap() = apps::list(&custom);
    }

    fn apps_msg(&self) -> PcMsg {
        let cache = self.icons.lock().unwrap();
        PcMsg::Apps {
            apps: self
                .apps
                .lock()
                .unwrap()
                .iter()
                .map(|a| AppEntry { id: a.id.clone(), name: a.name.clone(), icon: cache.get(&a.id).cloned().unwrap_or_default() })
                .collect(),
        }
    }

    /// Extrait les icônes manquantes en arrière-plan et les envoie par lots.
    fn stream_icons(self: &std::sync::Arc<Self>, tx: Sender<PcMsg>) {
        let me = self.clone();
        std::thread::spawn(move || {
            icons::com_init();
            let todo: Vec<apps::App> = {
                let cache = me.icons.lock().unwrap();
                me.apps.lock().unwrap().iter().filter(|a| a.id.starts_with("s:") && !cache.contains_key(&a.id)).cloned().collect()
            };
            // 1) icônes déjà en cache (une seule fois pour ce téléphone) — incluses dans apps_msg
            let mut batch = Vec::new();
            for a in todo {
                if let Some(png) = icons::icon_png_base64(&a.target) {
                    me.icons.lock().unwrap().insert(a.id.clone(), png.clone());
                    batch.push(IconEntry { id: a.id, icon: png });
                    if batch.len() >= ICON_BATCH {
                        if tx.send(PcMsg::Icons { icons: std::mem::take(&mut batch) }).is_err() {
                            return;
                        }
                    }
                }
            }
            if !batch.is_empty() {
                let _ = tx.send(PcMsg::Icons { icons: batch });
            }
        });
    }

    /// Appelé par l'écoute UI Automation quand le focus change.
    pub fn set_text_focus(&self, on: bool) {
        let mut t = self.text_focus.lock().unwrap();
        if *t != on {
            *t = on;
            self.typer.focus.store(on, Ordering::Relaxed);
            drop(t);
            self.broadcast(self.state_msg());
        }
    }

    // ---- Alt+Tab ----

    fn release_alt(&self) {
        let mut a = self.alt.lock().unwrap();
        if a.take().is_some() {
            input::key_up(VK_MENU);
        }
    }

    // ---- boucle de fond (200 ms) : sécurité Alt + maintien topmost ----

    pub fn tick(&self) {
        {
            let a = *self.alt.lock().unwrap();
            if let Some(t) = a {
                if t.elapsed() > ALT_SAFETY {
                    self.release_alt();
                }
            }
        }
        // Coffre : auto-verrouillage, et diffusion de l'état quand il change.
        if self.vault.expire(Instant::now()) {
            let v = self.vault.clone();
            std::thread::spawn(move || v.bw_lock());
        }
        let ver = self.vault.version();
        if self.vault_ver.swap(ver, Ordering::Relaxed) != ver {
            self.broadcast(self.vault.state_msg());
        }
        let pinned = *self.pinned.lock().unwrap();
        if let Some(h) = pinned {
            let hwnd = HWND(h as *mut _);
            if input::is_window(hwnd) {
                input::set_topmost(hwnd, true);
            } else {
                *self.pinned.lock().unwrap() = None;
                self.broadcast(self.state_msg());
            }
        }
    }

    // ---- commandes ----

    pub fn handle(&self, msg: PhoneMsg, reply: &Sender<PcMsg>) {
        match msg {
            PhoneMsg::Key { k } => match k {
                Key::Up => input::tap(VK_UP),
                Key::Down => input::tap(VK_DOWN),
                Key::Left => input::tap(VK_LEFT),
                Key::Right => input::tap(VK_RIGHT),
                Key::Enter => {
                    let holding = self.alt.lock().unwrap().is_some();
                    if holding {
                        self.release_alt(); // OK valide la sélection d'onglet
                    } else {
                        input::tap(VK_RETURN);
                    }
                }
                Key::Back => input::tap(VK_ESCAPE),
                Key::BackLong => input::chord(&[VK_MENU], VK_LEFT),
                // Touche « Menu contextuel » : ouvre le menu d'options d'une tuile sur l'accueil TV.
                Key::Menu => input::tap(VK_APPS),
                // F3 : ouvre la recherche universelle de l'accueil TV.
                Key::Search => input::tap(VK_F3),
            },
            PhoneMsg::AltTab { a } => {
                let mut alt = self.alt.lock().unwrap();
                match a {
                    AltTab::Open => {
                        if alt.is_none() {
                            input::key_down(VK_MENU);
                        }
                        input::tap(VK_TAB);
                        *alt = Some(Instant::now());
                    }
                    AltTab::Left | AltTab::Right => {
                        if alt.is_some() {
                            input::tap(if a == AltTab::Left { VK_LEFT } else { VK_RIGHT });
                            *alt = Some(Instant::now());
                        }
                    }
                    AltTab::Confirm => {
                        drop(alt);
                        self.release_alt();
                    }
                }
            }
            PhoneMsg::Pin => {
                {
                    let mut p = self.pinned.lock().unwrap();
                    if let Some(h) = p.take() {
                        input::set_topmost(HWND(h as *mut _), false);
                    } else {
                        let fg = input::foreground();
                        if !fg.0.is_null() {
                            input::set_topmost(fg, true);
                            *p = Some(fg.0 as isize);
                        }
                    }
                }
                self.broadcast(self.state_msg());
            }
            PhoneMsg::Text { backspaces, text } => {
                for _ in 0..backspaces.min(500) {
                    input::tap(VK_BACK);
                }
                input::type_text(&text);
            }
            PhoneMsg::Mouse { dx, dy, click, scroll } => {
                if dx != 0 || dy != 0 {
                    input::mouse_move(dx, dy);
                }
                if let Some(b) = click {
                    input::mouse_click(b == MouseButton::Right);
                }
                if scroll != 0 {
                    input::mouse_scroll(scroll);
                }
            }
            PhoneMsg::Launch { app_id } => {
                // Seules les entrées listées sont lançables : jamais de chemin arbitraire.
                let target = self.apps.lock().unwrap().iter().find(|a| a.id == app_id).map(|a| a.target.clone());
                if let Some(t) = target {
                    apps::launch(&t);
                }
            }
            PhoneMsg::GetApps => {
                self.refresh_apps();
                let _ = reply.send(self.apps_msg());
            }
            PhoneMsg::Home => self.emit(Event::Home),
            PhoneMsg::Tv { on } => self.emit(Event::Tv(on)),
            PhoneMsg::Media { a } => match a {
                // Les touches multimédia sont routées par Windows vers la session média active (SMTC).
                MediaAction::PlayPause => input::tap(VK_MEDIA_PLAY_PAUSE),
                MediaAction::Next => input::tap(VK_MEDIA_NEXT_TRACK),
                MediaAction::Prev => input::tap(VK_MEDIA_PREV_TRACK),
                MediaAction::Stop => input::tap(VK_MEDIA_STOP),
                MediaAction::VolUp => input::tap(VK_VOLUME_UP),
                MediaAction::VolDown => input::tap(VK_VOLUME_DOWN),
                MediaAction::Mute => input::tap(VK_VOLUME_MUTE),
            },
            PhoneMsg::GetRemoteMods => {
                let mods = self.remote_mods.lock().unwrap().clone();
                let _ = reply.send(PcMsg::RemoteMods { mods });
            }
            PhoneMsg::OpenTile { id } => self.emit(Event::OpenTile(id)),
            PhoneMsg::SetVolume { level } => {
                std::thread::spawn(move || audio::set(level));
            }
            PhoneMsg::Ping => {
                let _ = reply.send(PcMsg::Pong);
            }
            // `bw` est lent : un thread par opération du coffre (elles sont sérialisées dans `Vault`).
            m @ (PhoneMsg::VaultStatus
            | PhoneMsg::VaultUnlock { .. }
            | PhoneMsg::VaultLogin { .. }
            | PhoneMsg::VaultLock
            | PhoneMsg::VaultSearch { .. }
            | PhoneMsg::VaultFill { .. }) => {
                let (v, t, r) = (self.vault.clone(), self.typer.clone(), reply.clone());
                std::thread::spawn(move || crate::vault::dispatch(&v, &*t, m, &r));
            }
        }
    }
}

#[cfg(test)]
mod os_style_tests {
    use super::*;

    fn core() -> Core {
        let cfg = Config { pc_id: "t".into(), pc_name: "t".into(), devices: vec![], custom_apps: vec![] };
        Core::new(cfg)
    }

    fn style_of(m: PcMsg) -> Option<String> {
        match m {
            PcMsg::State { os_style, .. } => os_style,
            _ => panic!("state attendu"),
        }
    }

    #[test]
    fn rejects_unknown_and_dedups() {
        let c = core();
        let (tx, rx) = channel();
        c.conns.lock().unwrap().push(Conn { id: 1, device: "d".into(), tx });
        assert!(c.set_os_style("nope").is_err());
        assert!(rx.try_recv().is_err());
        assert!(c.set_os_style("tizen").is_ok());
        assert_eq!(style_of(rx.try_recv().unwrap()), Some("tizen".into()));
        // Même style : aucune rediffusion.
        assert!(c.set_os_style("tizen").is_ok());
        assert!(rx.try_recv().is_err());
        // Retour au style par défaut : champ absent.
        assert!(c.set_os_style("rustv").is_ok());
        assert_eq!(style_of(rx.try_recv().unwrap()), None);
        assert!(c.set_os_style("rustv").is_ok());
        assert!(rx.try_recv().is_err());
    }
}
