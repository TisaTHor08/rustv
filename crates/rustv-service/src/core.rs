//! État partagé et traitement des commandes du téléphone.
use crate::{apps, config::Config, icons, input};
use rustv_protocol::messages::*;
use std::collections::HashMap;
use std::sync::{mpsc::Sender, Mutex};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

/// Relâche Alt automatiquement si le téléphone ne confirme pas.
const ALT_SAFETY: Duration = Duration::from_secs(5);
const ICON_BATCH: usize = 25;

struct Conn {
    id: u64,
    device: String,
    tx: Sender<PcMsg>,
}

pub struct Core {
    pub cfg: Mutex<Config>,
    pub bt_addr: Mutex<Option<String>>,
    alt: Mutex<Option<Instant>>,   // Some(dernier événement) quand Alt est maintenu
    pinned: Mutex<Option<isize>>,  // HWND épinglé
    conns: Mutex<Vec<Conn>>,
    next_conn: Mutex<u64>,
    text_focus: Mutex<bool>,
    apps: Mutex<Vec<apps::App>>,
    icons: Mutex<HashMap<String, String>>,
}

impl Core {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg: Mutex::new(cfg),
            bt_addr: Mutex::new(None),
            alt: Mutex::new(None),
            pinned: Mutex::new(None),
            conns: Mutex::new(vec![]),
            next_conn: Mutex::new(0),
            text_focus: Mutex::new(false),
            apps: Mutex::new(vec![]),
            icons: Mutex::new(HashMap::new()),
        }
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
        self.refresh_apps();
        let _ = tx.send(self.apps_msg());
        self.stream_icons(tx);
        id
    }

    pub fn remove_conn(&self, id: u64) {
        let mut c = self.conns.lock().unwrap();
        c.retain(|x| x.id != id);
        if c.is_empty() {
            drop(c);
            self.release_alt(); // plus de téléphone : ne jamais laisser Alt coincé
            self.set_text_focus(false);
        }
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
        PcMsg::State { pinned: self.pinned.lock().unwrap().is_some(), text_focus: *self.text_focus.lock().unwrap() }
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
            PhoneMsg::Ping => {
                let _ = reply.send(PcMsg::Pong);
            }
        }
    }
}
