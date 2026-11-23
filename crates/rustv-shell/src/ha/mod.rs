//! Intégration Home Assistant : RusTV devient un appareil MQTT (découverte automatique), état + commandes.
//! Voir docs/HOME-ASSISTANT.md. Désactivée par défaut ; le mot de passe est protégé par DPAPI et jamais journalisé.
mod command;
mod config;
mod discovery;
mod dpapi;
mod net;
mod state;
mod tiles;
mod topics;

use command::{Cmd, RateLimiter};
use config::{HaConfig, HaConfigIn, STORE};
use dpapi::Dpapi;
use rumqttc::{Client, ConnAck, Event, Incoming, Outgoing, Publish, QoS};
use rustv_core::Core;
use rustv_protocol::messages::{MediaAction, PcMsg, PhoneMsg};
use serde::Serialize;
use serde_json::{json, Value};
use state::{Np, Snapshot, StateCache};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
use tiles::TileMap;
use topics::Topics;

static HA: OnceLock<Arc<Ha>> = OnceLock::new();

#[derive(Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub connected: bool,
    pub last_error: Option<String>,
    pub entities: usize,
    pub last_publish: Option<u64>,
}

pub struct Ha {
    app: AppHandle,
    cfg: Mutex<HaConfig>,
    status: Mutex<Status>,
    client: Mutex<Option<Client>>,
    /// Incrémenté à chaque changement de réglages : la session en cours se ferme et repart.
    gen: AtomicU64,
    stop: AtomicBool,
    /// Découverte retirée à la demande : on ne la republie pas avant un nouvel ordre ou un changement de réglages.
    removed: AtomicBool,
    opening: AtomicBool,
    tiles: Mutex<TileMap>,
    cache: Mutex<StateCache>,
    current_app: Mutex<Option<String>>,
    limiter: Mutex<RateLimiter>,
    started: Instant,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl Ha {
    fn core(&self) -> Arc<Core> {
        self.app.state::<crate::Shell>().core.clone()
    }

    fn pc(&self) -> (String, String) {
        let core = self.core();
        let c = core.cfg.lock().unwrap();
        (c.pc_id.clone(), c.pc_name.clone())
    }

    fn tv_active(&self) -> bool {
        self.app.state::<crate::Shell>().tv.lock().unwrap().active
    }

    fn topics(&self, cfg: &HaConfig) -> Topics {
        Topics::new(&cfg.base(&self.pc().0))
    }

    fn ctx(&self, cfg: &HaConfig) -> discovery::Ctx {
        let (id, name) = self.pc();
        discovery::Ctx {
            node: config::sanitize_node(&id),
            topics: self.topics(cfg),
            prefix: cfg.prefix(),
            pc_name: name,
            version: env!("CARGO_PKG_VERSION").to_string(),
            options: self.tiles.lock().unwrap().names(),
        }
    }

    fn connected(&self) -> bool {
        self.status.lock().unwrap().connected
    }

    fn set_connected(&self, on: bool, err: Option<String>) {
        let mut s = self.status.lock().unwrap();
        s.connected = on;
        if on {
            s.last_error = None;
        } else if err.is_some() {
            s.last_error = err;
        }
    }

    fn publish(&self, topic: &str, payload: impl Into<Vec<u8>>, retain: bool) -> bool {
        let c = self.client.lock().unwrap().clone();
        let Some(c) = c else { return false };
        let ok = c.try_publish(topic, QoS::AtLeastOnce, retain, payload).is_ok();
        if ok {
            self.status.lock().unwrap().last_publish = Some(now_secs());
        }
        ok
    }

    /// Ferme proprement la session courante : « offline » explicite puis déconnexion (le Last Will ne sert qu'aux coupures brutales).
    fn close_client(&self) {
        let cfg = self.cfg.lock().unwrap().clone();
        let c = self.client.lock().unwrap().clone();
        if let Some(c) = c {
            let _ = c.try_publish(self.topics(&cfg).availability(), QoS::AtLeastOnce, true, "offline");
            let _ = c.try_disconnect();
        }
    }

    // ---------- tuiles ----------

    /// Relit catalogue, préférences et mods ; renvoie `true` si la liste a changé.
    fn refresh_tiles(&self) -> bool {
        let map = tiles::build(&crate::tv::catalog(), &crate::prefs::get(), &crate::mods::list());
        let mut cur = self.tiles.lock().unwrap();
        if *cur != map {
            *cur = map;
            true
        } else {
            false
        }
    }

    // ---------- découverte ----------

    fn publish_discovery(&self) -> usize {
        let cfg = self.cfg.lock().unwrap().clone();
        let ctx = self.ctx(&cfg);
        let es = discovery::entities(&ctx);
        for e in &es {
            self.publish(&e.topic, e.payload.to_string(), true);
        }
        self.status.lock().unwrap().entities = es.len();
        es.len()
    }

    // ---------- état ----------

    fn publish_state(&self, tv_override: Option<bool>) {
        if !self.connected() || self.removed.load(SeqCst) {
            return;
        }
        let cfg = self.cfg.lock().unwrap().clone();
        let t = self.topics(&cfg);
        let core = self.core();
        let tv_on = tv_override.unwrap_or_else(|| self.tv_active());
        let now = core.now_playing().map(|n| Np { title: n.title, artist: n.artist, source: n.source, playing: n.playing, position: n.position, duration: n.duration });
        let cur_id = if tv_on { self.current_app.lock().unwrap().clone() } else { None };
        let (current_app, select_option) = {
            let tl = self.tiles.lock().unwrap();
            let name = cur_id.map(|id| tl.name_for(&id).map(String::from).unwrap_or(id));
            let opt = name.clone().filter(|n| tl.id_for(n).is_some());
            (name, opt)
        };
        let snap = Snapshot {
            tv_on,
            now,
            volume: core.volume(),
            phones: core.conn_count(),
            current_app,
            uptime_min: self.started.elapsed().as_secs() / 60,
            version: env!("CARGO_PKG_VERSION").to_string(),
            select_option,
        };
        let changed = self.cache.lock().unwrap().diff(state::topics(&t, &snap));
        for (topic, payload) in changed {
            self.publish(&topic, payload, true);
        }
    }

    // ---------- commandes ----------

    fn on_message(self: &Arc<Self>, topics: &Topics, status_topic: &str, p: Publish) {
        // Une commande retenue serait rejouée à chaque connexion : on l'ignore.
        if p.retain {
            return;
        }
        if p.topic == status_topic {
            if p.payload.as_ref().eq_ignore_ascii_case(b"online") && !self.removed.load(SeqCst) {
                self.publish_discovery();
                self.cache.lock().unwrap().reset();
                self.publish_state(None);
            }
            return;
        }
        let Some(sub) = topics.cmd_sub(&p.topic) else { return };
        if !self.limiter.lock().unwrap().allow(Instant::now()) {
            return;
        }
        if let Ok(cmd) = command::parse(sub, &p.payload) {
            self.exec(cmd);
        }
    }

    fn exec(self: &Arc<Self>, cmd: Cmd) {
        let core = self.core();
        let (tx, rx) = channel::<PcMsg>();
        drop(rx); // les réponses ne nous intéressent pas
        match cmd {
            Cmd::Tv(on) => core.handle(PhoneMsg::Tv { on }, &tx),
            Cmd::Mute(want) => {
                let cur = rustv_core::audio::get().or_else(|| core.volume()).map(|v| v.1);
                if cur.is_some() && cur != Some(want) {
                    core.handle(PhoneMsg::Media { a: MediaAction::Mute }, &tx);
                }
            }
            Cmd::Volume(level) => core.handle(PhoneMsg::SetVolume { level }, &tx),
            Cmd::App(name) => {
                let id = self.tiles.lock().unwrap().id_for(&name).map(String::from);
                if let Some(id) = id {
                    self.open_tile(id);
                }
            }
            Cmd::OpenTile(id) => self.open_tile(id),
            Cmd::Button(b) => {
                if let Some(m) = command::button_msg(b) {
                    core.handle(m, &tx);
                }
            }
        }
    }

    /// Ouvre une tuile ; si le mode TV est éteint, le démarre d'abord (au mieux : plusieurs écrans = sélecteur, abandon).
    fn open_tile(self: &Arc<Self>, id: String) {
        let core = self.core();
        let (tx, _rx) = channel::<PcMsg>();
        if self.tv_active() {
            core.handle(PhoneMsg::OpenTile { id }, &tx);
            return;
        }
        if self.opening.swap(true, SeqCst) {
            return;
        }
        let me = self.clone();
        std::thread::spawn(move || {
            core.handle(PhoneMsg::Tv { on: true }, &tx);
            let end = Instant::now() + Duration::from_secs(15);
            while Instant::now() < end && !me.tv_active() {
                std::thread::sleep(Duration::from_millis(250));
            }
            if me.tv_active() {
                std::thread::sleep(Duration::from_millis(2500)); // l'interface doit être prête à écouter « open-tile »
                core.handle(PhoneMsg::OpenTile { id }, &tx);
            }
            me.opening.store(false, SeqCst);
        });
    }

    // ---------- connexion ----------

    fn sleep(&self, gen: u64, ms: u64) {
        let end = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < end && !self.stop.load(SeqCst) && self.gen.load(SeqCst) == gen {
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn run(self: Arc<Self>) {
        let mut backoff = 1u64;
        while !self.stop.load(SeqCst) {
            let gen = self.gen.load(SeqCst);
            let cfg = self.cfg.lock().unwrap().clone();
            if !cfg.enabled || cfg.host.is_empty() {
                self.set_connected(false, None);
                self.status.lock().unwrap().last_error = None;
                self.sleep(gen, 1000);
                continue;
            }
            self.session(&cfg, gen, &mut backoff);
        }
    }

    fn session(self: &Arc<Self>, cfg: &HaConfig, gen: u64, backoff: &mut u64) {
        let topics = self.topics(cfg);
        let status_topic = format!("{}/status", cfg.prefix());
        let password = cfg.password(&Dpapi);
        if !cfg.password_enc.is_empty() && password.is_none() {
            self.set_connected(false, Some("The saved password cannot be read (saved by another Windows account): enter it again.".into()));
            self.sleep(gen, 5000);
            return;
        }
        let opts = net::options(cfg, &cfg.client(&self.pc().0), password, Some((topics.availability(), "offline")));
        let (client, mut conn) = Client::new(opts, 256);
        conn.eventloop.set_network_options(net::network());
        *self.client.lock().unwrap() = Some(client.clone());
        let mut closing = false;
        // `iter()` : la connexion avance sans être annulée ; les changements de réglages la réveillent par `try_disconnect`.
        for ev in conn.iter() {
            if !closing && (self.stop.load(SeqCst) || self.gen.load(SeqCst) != gen) {
                closing = true;
                self.close_client();
            }
            match ev {
                Ok(Event::Incoming(Incoming::ConnAck(ConnAck { code, .. }))) if code == rumqttc::ConnectReturnCode::Success => {
                    *backoff = 1;
                    self.set_connected(true, None);
                    let _ = client.try_publish(topics.availability(), QoS::AtLeastOnce, true, "online");
                    let _ = client.try_subscribe(topics.cmd_filter(), QoS::AtLeastOnce);
                    let _ = client.try_subscribe(status_topic.clone(), QoS::AtLeastOnce);
                    self.refresh_tiles();
                    if !self.removed.load(SeqCst) {
                        self.publish_discovery();
                    }
                    self.cache.lock().unwrap().reset();
                    self.publish_state(None);
                }
                Ok(Event::Incoming(Incoming::Publish(p))) => self.on_message(&topics, &status_topic, p),
                Ok(Event::Outgoing(Outgoing::Disconnect)) => break,
                Ok(_) => {}
                Err(e) => {
                    self.set_connected(false, Some(net::friendly_error(&e)));
                    if closing || self.stop.load(SeqCst) || self.gen.load(SeqCst) != gen {
                        break;
                    }
                    self.sleep(gen, *backoff * 1000);
                    *backoff = (*backoff * 2).min(60);
                    if self.stop.load(SeqCst) || self.gen.load(SeqCst) != gen {
                        break;
                    }
                }
            }
        }
        *self.client.lock().unwrap() = None;
        self.set_connected(false, None);
    }

    /// Nouveaux réglages : la session en cours se ferme et une nouvelle démarre.
    fn reconfigure(&self) {
        self.gen.fetch_add(1, SeqCst);
        self.removed.store(false, SeqCst);
        self.close_client();
    }

    fn state_loop(self: Arc<Self>) {
        let mut last_mtime: Option<SystemTime> = None;
        let mut last_full = Instant::now();
        while !self.stop.load(SeqCst) {
            std::thread::sleep(Duration::from_secs(1));
            let mtime = std::fs::metadata(crate::settings::dir().join("prefs.json")).and_then(|m| m.modified()).ok();
            if mtime != last_mtime || last_full.elapsed() > Duration::from_secs(60) {
                last_mtime = mtime;
                last_full = Instant::now();
                if self.refresh_tiles() && self.connected() && !self.removed.load(SeqCst) {
                    self.publish_discovery();
                    self.cache.lock().unwrap().reset();
                }
            }
            self.publish_state(None);
        }
    }

    /// Événements du noyau : dernière tuile ouverte, retour à l'accueil, téléphones connectés.
    fn event_loop(self: Arc<Self>) {
        for ev in self.core().subscribe() {
            match ev {
                rustv_core::Event::OpenTile(id) => *self.current_app.lock().unwrap() = Some(id),
                rustv_core::Event::Home | rustv_core::Event::Tv(false) => *self.current_app.lock().unwrap() = None,
                _ => {}
            }
            self.publish_state(None);
        }
    }
}

fn get() -> Result<Arc<Ha>, String> {
    HA.get().cloned().ok_or_else(|| "not_started".to_string())
}

/// Démarre l'intégration (threads de fond ; ne fait rien tant qu'elle n'est pas activée dans les réglages).
pub fn start(app: &AppHandle) {
    let cfg: HaConfig = crate::store::load(STORE);
    let ha = Arc::new(Ha {
        app: app.clone(),
        cfg: Mutex::new(cfg),
        status: Mutex::new(Status::default()),
        client: Mutex::new(None),
        gen: AtomicU64::new(0),
        stop: AtomicBool::new(false),
        removed: AtomicBool::new(false),
        opening: AtomicBool::new(false),
        tiles: Mutex::new(TileMap::default()),
        cache: Mutex::new(StateCache::default()),
        current_app: Mutex::new(None),
        limiter: Mutex::new(RateLimiter::new(20, Duration::from_secs(1))),
        started: Instant::now(),
    });
    if HA.set(ha.clone()).is_err() {
        return;
    }
    {
        let h = ha.clone();
        crate::hooks::on_tv_mode(move |_app, on| {
            if !on {
                *h.current_app.lock().unwrap() = None;
            }
            h.publish_state(Some(on));
        });
    }
    for f in [Ha::run as fn(Arc<Ha>), Ha::state_loop, Ha::event_loop] {
        let h = ha.clone();
        std::thread::spawn(move || f(h));
    }
}

/// Arrêt propre (à appeler avant de quitter) : « offline » publié, déconnexion, attente brève.
#[allow(dead_code)]
pub fn shutdown() {
    if let Ok(ha) = get() {
        ha.stop.store(true, SeqCst);
        ha.close_client();
        let end = Instant::now() + Duration::from_millis(1500);
        while Instant::now() < end && ha.client.lock().unwrap().is_some() {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

// ---------- commandes Tauri ----------

#[tauri::command]
pub async fn ha_get() -> Result<Value, String> {
    let ha = get()?;
    let cfg = ha.cfg.lock().unwrap().clone();
    let status = ha.status.lock().unwrap().clone();
    Ok(json!({ "config": cfg.view(&ha.pc().0), "status": status }))
}

#[tauri::command]
pub async fn ha_set(cfg: HaConfigIn) -> Result<(), String> {
    let ha = get()?;
    let mut next = ha.cfg.lock().unwrap().clone();
    next.apply(cfg, &Dpapi).map_err(|e| e.to_string())?;
    crate::store::save(STORE, &next)?;
    *ha.cfg.lock().unwrap() = next;
    ha.reconfigure();
    Ok(())
}

#[tauri::command]
pub async fn ha_test() -> Result<String, String> {
    let ha = get()?;
    let cfg = ha.cfg.lock().unwrap().clone();
    if cfg.host.is_empty() {
        return Err("Enter the broker address first.".into());
    }
    let password = cfg.password(&Dpapi);
    if !cfg.password_enc.is_empty() && password.is_none() {
        return Err("The saved password cannot be read: enter it again.".into());
    }
    let (pc_id, _) = ha.pc();
    let mut cid = cfg.client(&pc_id);
    cid.truncate(58);
    let cid = format!("{cid}-test");
    let topic = ha.topics(&cfg).test();
    let body = json!({ "msg": "RusTV connection test", "version": env!("CARGO_PKG_VERSION") }).to_string().into_bytes();
    tauri::async_runtime::spawn_blocking(move || net::exchange(&cfg, password, &cid, vec![(topic, body, false)]))
        .await
        .map_err(|_| "Test interrupted.".to_string())??;
    Ok("Connected: the broker accepted the login and a test message.".into())
}

#[tauri::command]
pub async fn ha_publish_discovery() -> Result<usize, String> {
    let ha = get()?;
    if !ha.connected() {
        return Err("not_connected".into());
    }
    ha.removed.store(false, SeqCst);
    ha.refresh_tiles();
    let n = ha.publish_discovery();
    ha.cache.lock().unwrap().reset();
    ha.publish_state(None);
    Ok(n)
}

#[tauri::command]
pub async fn ha_remove_discovery() -> Result<usize, String> {
    let ha = get()?;
    let cfg = ha.cfg.lock().unwrap().clone();
    if cfg.host.is_empty() {
        return Err("Enter the broker address first.".into());
    }
    let password = cfg.password(&Dpapi);
    let ctx = ha.ctx(&cfg);
    let msgs: Vec<(String, Vec<u8>, bool)> = discovery::entities(&ctx).into_iter().map(|e| (e.topic, Vec::new(), true)).collect();
    let n = msgs.len();
    let mut cid = cfg.client(&ha.pc().0);
    cid.truncate(58);
    let cid = format!("{cid}-tmp");
    // Tant que le retrait n'est pas annulé, ni la découverte ni l'état ne sont republiés.
    ha.removed.store(true, SeqCst);
    tauri::async_runtime::spawn_blocking(move || net::exchange(&cfg, password, &cid, msgs)).await.map_err(|_| "Interrupted.".to_string())??;
    ha.status.lock().unwrap().entities = 0;
    Ok(n)
}
