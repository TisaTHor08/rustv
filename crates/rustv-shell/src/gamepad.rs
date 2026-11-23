//! Manette de jeu (XInput) utilisée comme télécommande du mode TV.
//!
//! Les boutons deviennent exactement les mêmes messages que ceux du téléphone (`PhoneMsg`) et passent par
//! `Core::handle` : flèches, Entrée, Retour (Échap), Retour long (Alt+Gauche), Menu, Recherche, médias, volume,
//! Alt+Tab, Accueil, souris. Rien n'est injecté si la fenêtre au premier plan n'est pas RusTV, un processus
//! lancé par RusTV (mpv, navigateur) ou une appli de la liste d'autorisation : les jeux gardent la manette.
//!
//! Toute la logique (sens du stick, répétition, fronts, mapping, décision premier plan, combo d'échappement)
//! est pure et testée ; les appels XInput / Win32 restent minces.
#![cfg_attr(not(windows), allow(dead_code, unused_imports))]
use crate::{store, Shell};
use rustv_protocol::messages::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

// ---------- configuration ----------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub enabled: bool,
    /// Zone morte des sticks, 0.1 à 0.9.
    pub deadzone: f32,
    pub repeat_delay_ms: u32,
    pub repeat_rate_ms: u32,
    /// 0 = aucune vibration, 1 faible, 2 moyenne, 3 forte.
    pub vibration: u8,
    #[serde(rename = "swapAB", alias = "swapAb")]
    pub swap_ab: bool,
    pub pointer_mode: bool,
    /// N'agit que quand le mode TV est actif.
    pub only_when_tv: bool,
    /// Noms d'exécutables supplémentaires où la manette agit (ex. `vlc.exe`).
    pub allow: Vec<String>,
    /// Noms d'exécutables où la manette n'agit jamais (même s'ils ont été lancés par RusTV).
    pub deny: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: true,
            deadzone: 0.35,
            repeat_delay_ms: 380,
            repeat_rate_ms: 90,
            vibration: 1,
            swap_ab: false,
            pointer_mode: false,
            only_when_tv: true,
            allow: vec![],
            deny: vec![],
        }
    }
}

fn clean_list(v: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in v.iter().take(64) {
        let s = s.trim().to_lowercase();
        if !s.is_empty() && s.len() <= 64 && !s.contains(['\\', '/']) && !out.contains(&s) {
            out.push(s);
        }
        if out.len() >= 32 {
            break;
        }
    }
    out
}

/// Borne toutes les valeurs reçues de l'interface.
pub fn sanitize(mut c: Config) -> Config {
    c.deadzone = if c.deadzone.is_finite() { c.deadzone.clamp(0.1, 0.9) } else { 0.35 };
    c.repeat_delay_ms = c.repeat_delay_ms.clamp(150, 1500);
    c.repeat_rate_ms = c.repeat_rate_ms.clamp(30, 400);
    c.vibration = c.vibration.min(3);
    c.allow = clean_list(&c.allow);
    c.deny = clean_list(&c.deny);
    c
}

static CFG: Mutex<Option<Config>> = Mutex::new(None);
/// Pause temporaire (combo Start + Retour) : jamais persistée, la manette revient active au prochain démarrage.
static PAUSED: AtomicBool = AtomicBool::new(false);

fn cfg() -> Config {
    let mut g = CFG.lock().unwrap();
    g.get_or_insert_with(|| sanitize(store::load::<Config>("gamepad"))).clone()
}

// ---------- logique pure ----------

pub const DPAD_UP: u16 = 0x0001;
pub const DPAD_DOWN: u16 = 0x0002;
pub const DPAD_LEFT: u16 = 0x0004;
pub const DPAD_RIGHT: u16 = 0x0008;
pub const START: u16 = 0x0010;
pub const BACK: u16 = 0x0020; // bouton « Vue » / « Select »
pub const L3: u16 = 0x0040;
pub const R3: u16 = 0x0080;
pub const LB: u16 = 0x0100;
pub const RB: u16 = 0x0200;
pub const BTN_A: u16 = 0x1000;
pub const BTN_B: u16 = 0x2000;
pub const BTN_X: u16 = 0x4000;
pub const BTN_Y: u16 = 0x8000;
const MAPPED: [u16; 10] = [BTN_A, BTN_B, BTN_X, BTN_Y, START, BACK, LB, RB, L3, R3];

const TRIGGER_ON: u8 = 70; // sur 255
const COMBO_MS: u64 = 3000;
const BACK_LONG_MS: u64 = 600;
/// Le noyau relâche Alt après 5 s sans action : on considère le sélecteur fermé un peu avant.
const ALT_WINDOW_MS: u64 = 4500;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    fn key(self) -> Key {
        match self {
            Dir::Up => Key::Up,
            Dir::Down => Key::Down,
            Dir::Left => Key::Left,
            Dir::Right => Key::Right,
        }
    }
    fn horizontal(self) -> bool {
        matches!(self, Dir::Left | Dir::Right)
    }
}

/// Direction d'un stick (x, y normalisés -1..1, y positif vers le haut). Zone morte circulaire avec hystérésis
/// (on relâche à 80 % de la zone morte) ; l'axe dominant gagne, avec une préférence pour l'axe déjà actif afin
/// d'éviter les bascules sur les diagonales.
pub fn stick_dir(x: f32, y: f32, dz: f32, prev: Option<Dir>) -> Option<Dir> {
    let limit = if prev.is_some() { dz * 0.8 } else { dz };
    if (x * x + y * y).sqrt() < limit {
        return None;
    }
    let (ax, ay) = (x.abs(), y.abs());
    let k = match prev {
        Some(d) if d.horizontal() => 0.75,
        Some(_) => 1.33,
        None => 1.0,
    };
    if ax >= ay * k {
        Some(if x > 0.0 { Dir::Right } else { Dir::Left })
    } else {
        Some(if y > 0.0 { Dir::Up } else { Dir::Down })
    }
}

pub fn dpad_dir(b: u16) -> Option<Dir> {
    if b & DPAD_UP != 0 {
        Some(Dir::Up)
    } else if b & DPAD_DOWN != 0 {
        Some(Dir::Down)
    } else if b & DPAD_LEFT != 0 {
        Some(Dir::Left)
    } else if b & DPAD_RIGHT != 0 {
        Some(Dir::Right)
    } else {
        None
    }
}

/// Répétition automatique façon clavier : un déclenchement immédiat, puis après `delay` toutes les `rate` ms.
pub struct Repeater<T> {
    cur: Option<T>,
    next_at: u64,
}

impl<T> Default for Repeater<T> {
    fn default() -> Self {
        Repeater { cur: None, next_at: 0 }
    }
}

impl<T: Copy + PartialEq> Repeater<T> {
    pub fn update(&mut self, now: u64, held: Option<T>, delay: u64, rate: u64) -> Option<T> {
        if held != self.cur {
            self.cur = held;
            if held.is_some() {
                self.next_at = now + delay;
            }
            return held;
        }
        if held.is_some() && now >= self.next_at {
            self.next_at = now + rate;
            return held;
        }
        None
    }
    pub fn reset(&mut self) {
        self.cur = None;
    }
}

/// Fronts montants et descendants entre deux états de boutons.
pub fn edges(prev: u16, cur: u16) -> (u16, u16) {
    (cur & !prev, prev & !cur)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vol {
    Up,
    Down,
}

fn vol_dir(lt: u8, rt: u8) -> Option<Vol> {
    match (lt >= TRIGGER_ON, rt >= TRIGGER_ON) {
        (true, false) => Some(Vol::Down),
        (false, true) => Some(Vol::Up),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act {
    Key(Key),
    /// Retour : Échap au relâchement, Retour long si maintenu.
    Back,
    /// Déclenchés au relâchement, sauf si le combo Start + Vue était en cours.
    MenuOnRelease,
    HomeOnRelease,
    /// Sélecteur de fenêtres (Alt+Tab) : vrai = précédent.
    Tab(bool),
    Media(MediaAction),
    /// Clic souris (vrai = droit), mode pointeur.
    Click(bool),
    None,
}

/// Table de correspondance bouton -> action.
pub fn map_button(mask: u16, swap_ab: bool, pointer: bool) -> Act {
    let (confirm, cancel) = if swap_ab { (BTN_B, BTN_A) } else { (BTN_A, BTN_B) };
    match mask {
        m if m == confirm => Act::Key(Key::Enter),
        m if m == cancel => Act::Back,
        BTN_X => Act::Key(Key::Menu),
        BTN_Y => Act::Key(Key::Search),
        START => Act::MenuOnRelease,
        BACK => Act::HomeOnRelease,
        LB => Act::Tab(true),
        RB => Act::Tab(false),
        L3 if pointer => Act::Click(true),
        L3 => Act::Media(MediaAction::PlayPause),
        R3 if pointer => Act::Click(false),
        _ => Act::None,
    }
}

/// Messages Alt+Tab pour LB / RB, comme les boutons du téléphone. `last` : date du dernier événement du sélecteur.
pub fn tab_msgs(left: bool, last: &mut Option<u64>, now: u64) -> Vec<AltTab> {
    let open = last.is_some_and(|t| now.saturating_sub(t) <= ALT_WINDOW_MS);
    *last = Some(now);
    match (open, left) {
        (true, true) => vec![AltTab::Left],
        (true, false) => vec![AltTab::Right],
        (false, false) => vec![AltTab::Open],
        (false, true) => vec![AltTab::Open, AltTab::Left],
    }
}

/// Combo d'échappement : Start + Vue maintenus `COMBO_MS`. Se réarme au relâchement.
#[derive(Default)]
pub struct Combo {
    since: Option<u64>,
    fired: bool,
}

impl Combo {
    pub fn update(&mut self, now: u64, held: bool) -> bool {
        if !held {
            self.reset();
            return false;
        }
        let s = *self.since.get_or_insert(now);
        if !self.fired && now.saturating_sub(s) >= COMBO_MS {
            self.fired = true;
            return true;
        }
        false
    }
    pub fn reset(&mut self) {
        self.since = None;
        self.fired = false;
    }
}

/// Accumulateur d'axe : zone morte, courbe quadratique, `rate` unités par seconde à pleine inclinaison.
/// Ne renvoie des unités entières qu'à partir de `min_emit`.
pub fn axis_accum(v: f32, dz: f32, rate: f32, dt_ms: u64, min_emit: f32, acc: &mut f32) -> i32 {
    let a = v.abs();
    if a <= dz {
        *acc = 0.0;
        return 0;
    }
    let s = ((a - dz) / (1.0 - dz)).powi(2) * v.signum();
    *acc += s * rate * dt_ms as f32 / 1000.0;
    if acc.abs() < min_emit {
        return 0;
    }
    let whole = acc.trunc();
    *acc -= whole;
    whole as i32
}

/// Impulsions de vibration : (décalage en ms, vitesse du moteur) ; la vitesse 0 arrête le moteur.
pub fn pulse_plan(level: u8, double: bool) -> Vec<(u64, u16)> {
    let s = [0u16, 12000, 26000, 45000][level.min(3) as usize];
    if s == 0 {
        return vec![];
    }
    if double {
        vec![(0, s), (45, 0), (110, s), (155, 0)]
    } else {
        vec![(0, s), (40, 0)]
    }
}

/// Description du premier plan, déjà résolue.
pub struct FgInfo {
    /// Fenêtre d'un processus de RusTV lui-même.
    pub is_self: bool,
    /// Processus descendant de RusTV (mpv, navigateur lancé par RusTV).
    pub is_child: bool,
    /// Nom du fichier exécutable (`None` si inconnu : élevé, inaccessible).
    pub exe: Option<String>,
}

fn norm(s: &str) -> String {
    let l = s.trim().to_lowercase();
    l.strip_suffix(".exe").unwrap_or(&l).to_string()
}

/// La manette peut-elle agir sur cette fenêtre ? Un processus inconnu n'est jamais autorisé.
pub fn fg_allowed(f: &FgInfo, allow: &[String], deny: &[String]) -> bool {
    if f.is_self {
        return true;
    }
    let exe = f.exe.as_deref().map(norm);
    if let Some(e) = &exe {
        if deny.iter().any(|d| norm(d) == *e) {
            return false;
        }
    }
    if f.is_child {
        return true;
    }
    exe.is_some_and(|e| allow.iter().any(|a| norm(a) == e))
}

// ---------- machine d'état par manette ----------

#[derive(Clone, Copy, Default)]
pub struct Input {
    pub buttons: u16,
    pub lt: u8,
    pub rt: u8,
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
}

fn norm_axis(v: i16) -> f32 {
    (v as f32 / 32767.0).clamp(-1.0, 1.0)
}

#[derive(Default)]
pub struct PadRt {
    prev: u16,
    armed: bool,
    last_now: u64,
    dir_rep: Repeater<Dir>,
    stick_dir: Option<Dir>,
    vol_rep: Repeater<Vol>,
    back_down: Option<u64>,
    back_long: bool,
    tainted: bool,
    combo: Combo,
    alt_last: Option<u64>,
    acc_x: f32,
    acc_y: f32,
}

#[derive(Debug, PartialEq)]
pub enum Out {
    Msg(PhoneMsg),
    Rumble(Vec<(u64, u16)>),
    TogglePause,
}

pub struct Gate {
    /// Le premier plan est autorisé (et le mode TV actif si demandé).
    pub fg_ok: bool,
    /// Activée dans la config et pas en pause.
    pub on: bool,
}

/// Un pas de simulation pour une manette : état -> messages à envoyer.
pub fn step(rt: &mut PadRt, inp: &Input, now: u64, c: &Config, gate: &Gate) -> Vec<Out> {
    let mut out = Vec::new();
    let dt = now.saturating_sub(rt.last_now).min(100);
    rt.last_now = now;
    let b = inp.buttons;

    // Combo d'échappement : actif tant que le premier plan est autorisé, même en pause (pour réactiver).
    if gate.fg_ok && c.enabled {
        if rt.combo.update(now, b & START != 0 && b & BACK != 0) {
            out.push(Out::TogglePause);
        }
    } else {
        rt.combo.reset();
    }

    let (lx, ly, rx, ry) = (norm_axis(inp.lx), norm_axis(inp.ly), norm_axis(inp.rx), norm_axis(inp.ry));
    let sd = stick_dir(lx, ly, c.deadzone, rt.stick_dir);
    rt.stick_dir = sd;
    let dir = dpad_dir(b).or(sd);
    let vol = vol_dir(inp.lt, inp.rt);

    if !(gate.fg_ok && gate.on) {
        // Inactif : on ne mémorise que l'état, rien n'est émis et il faudra tout relâcher avant de reprendre.
        rt.armed = false;
        rt.prev = b;
        rt.dir_rep.reset();
        rt.vol_rep.reset();
        rt.back_down = None;
        rt.back_long = false;
        return out;
    }
    if !rt.armed {
        let neutral = b == 0 && dir.is_none() && vol.is_none() && ((rx * rx + ry * ry).sqrt() < c.deadzone);
        rt.prev = b;
        if neutral {
            rt.armed = true;
        }
        return out;
    }

    let (pressed, released) = edges(rt.prev, b);
    rt.prev = b;
    if b & START != 0 && b & BACK != 0 {
        rt.tainted = true;
    }
    let tainted = rt.tainted;
    let msg = |m: PhoneMsg| Out::Msg(m);

    for mask in MAPPED {
        let act = map_button(mask, c.swap_ab, c.pointer_mode);
        if pressed & mask != 0 {
            match act {
                Act::Key(k) => {
                    out.push(msg(PhoneMsg::Key { k }));
                    if k == Key::Enter {
                        rt.alt_last = None; // le noyau ferme le sélecteur sur Entrée
                        out.push(Out::Rumble(pulse_plan(c.vibration, false)));
                    }
                }
                Act::Back => {
                    rt.back_down = Some(now);
                    rt.back_long = false;
                }
                Act::Tab(left) => {
                    for a in tab_msgs(left, &mut rt.alt_last, now) {
                        out.push(msg(PhoneMsg::AltTab { a }));
                    }
                }
                Act::Media(a) => out.push(msg(PhoneMsg::Media { a })),
                Act::Click(right) => out.push(msg(PhoneMsg::Mouse { dx: 0, dy: 0, click: Some(if right { MouseButton::Right } else { MouseButton::Left }), scroll: 0 })),
                Act::MenuOnRelease | Act::HomeOnRelease | Act::None => {}
            }
        }
        if released & mask != 0 {
            match act {
                Act::Back => {
                    if rt.back_down.take().is_some() && !rt.back_long {
                        out.push(msg(PhoneMsg::Key { k: Key::Back }));
                        out.push(Out::Rumble(pulse_plan(c.vibration, false)));
                    }
                    rt.back_long = false;
                }
                Act::MenuOnRelease if !tainted => out.push(msg(PhoneMsg::Key { k: Key::Menu })),
                Act::HomeOnRelease if !tainted => out.push(msg(PhoneMsg::Home)),
                _ => {}
            }
        }
    }
    if b & (START | BACK) == 0 {
        rt.tainted = false; // après le traitement des relâchements
    }
    if let Some(t) = rt.back_down {
        if !rt.back_long && now.saturating_sub(t) >= BACK_LONG_MS {
            rt.back_long = true;
            out.push(msg(PhoneMsg::Key { k: Key::BackLong }));
            out.push(Out::Rumble(pulse_plan(c.vibration, true)));
        }
    }

    let (delay, rate) = (c.repeat_delay_ms as u64, c.repeat_rate_ms as u64);
    if let Some(d) = rt.dir_rep.update(now, dir, delay, rate) {
        out.push(msg(PhoneMsg::Key { k: d.key() }));
    }
    if let Some(v) = rt.vol_rep.update(now, vol, delay, rate) {
        let a = if v == Vol::Up { MediaAction::VolUp } else { MediaAction::VolDown };
        out.push(msg(PhoneMsg::Media { a }));
    }

    // Stick droit : défilement vertical, ou déplacement du pointeur en mode pointeur.
    if c.pointer_mode {
        let dx = axis_accum(rx, c.deadzone, 1100.0, dt, 1.0, &mut rt.acc_x);
        let dy = -axis_accum(ry, c.deadzone, 1100.0, dt, 1.0, &mut rt.acc_y);
        if dx != 0 || dy != 0 {
            out.push(msg(PhoneMsg::Mouse { dx, dy, click: None, scroll: 0 }));
        }
    } else {
        let scroll = axis_accum(ry, c.deadzone, 1000.0, dt, 20.0, &mut rt.acc_y);
        if scroll != 0 {
            out.push(msg(PhoneMsg::Mouse { dx: 0, dy: 0, click: None, scroll }));
        }
    }
    out
}

// ---------- XInput (appels minces, chargés dynamiquement) ----------

#[cfg(windows)]
use windows::core::{s, w, PWSTR};
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
#[cfg(windows)]
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
#[cfg(windows)]
use windows::Win32::UI::Input::XboxController::{BATTERY_DEVTYPE, BATTERY_DEVTYPE_GAMEPAD, XINPUT_BATTERY_INFORMATION, XINPUT_STATE, XINPUT_VIBRATION};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

#[cfg(windows)]
type GetState = unsafe extern "system" fn(u32, *mut XINPUT_STATE) -> u32;
#[cfg(windows)]
type SetState = unsafe extern "system" fn(u32, *const XINPUT_VIBRATION) -> u32;
#[cfg(windows)]
type GetBattery = unsafe extern "system" fn(u32, BATTERY_DEVTYPE, *mut XINPUT_BATTERY_INFORMATION) -> u32;

#[cfg(windows)]
struct XInput {
    get: GetState,
    set: SetState,
    /// Absent de xinput9_1_0.dll (Windows Vista/7 sans DLL récente).
    battery: Option<GetBattery>,
}

#[cfg(windows)]
static XI: OnceLock<Option<XInput>> = OnceLock::new();

#[cfg(windows)]
impl XInput {
    /// xinput1_4 (Windows 8+), puis xinput1_3, puis xinput9_1_0 ; la fonction batterie est facultative.
    fn load() -> Option<XInput> {
        let libs = [w!("xinput1_4.dll"), w!("xinput1_3.dll"), w!("xinput9_1_0.dll")];
        for l in libs {
            let Ok(h) = (unsafe { LoadLibraryW(l) }) else { continue };
            unsafe {
                let (Some(g), Some(s_)) = (GetProcAddress(h, s!("XInputGetState")), GetProcAddress(h, s!("XInputSetState"))) else { continue };
                let bat = GetProcAddress(h, s!("XInputGetBatteryInformation"));
                return Some(XInput {
                    get: std::mem::transmute::<unsafe extern "system" fn() -> isize, GetState>(g),
                    set: std::mem::transmute::<unsafe extern "system" fn() -> isize, SetState>(s_),
                    battery: bat.map(|b| std::mem::transmute::<unsafe extern "system" fn() -> isize, GetBattery>(b)),
                });
            }
        }
        None
    }
    fn state(&self, slot: u32) -> Option<Input> {
        let mut st: XINPUT_STATE = unsafe { std::mem::zeroed() };
        if unsafe { (self.get)(slot, &mut st) } != 0 {
            return None;
        }
        let g = st.Gamepad;
        Some(Input { buttons: g.wButtons.0, lt: g.bLeftTrigger, rt: g.bRightTrigger, lx: g.sThumbLX, ly: g.sThumbLY, rx: g.sThumbRX, ry: g.sThumbRY })
    }
    fn rumble(&self, slot: u32, speed: u16) {
        let v = XINPUT_VIBRATION { wLeftMotorSpeed: speed, wRightMotorSpeed: speed };
        unsafe { (self.set)(slot, &v) };
    }
    fn battery(&self, slot: u32) -> (u8, u8) {
        let Some(f) = self.battery else { return (255, 255) };
        let mut bi: XINPUT_BATTERY_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { f(slot, BATTERY_DEVTYPE_GAMEPAD, &mut bi) } != 0 {
            return (255, 255);
        }
        (bi.BatteryType.0, bi.BatteryLevel.0)
    }
}

// ---------- état partagé avec les commandes ----------

#[derive(Clone, Copy, Default)]
struct Pad {
    connected: bool,
    bt: u8,
    bl: u8,
}

static PADS: Mutex<[Pad; 4]> = Mutex::new([Pad { connected: false, bt: 255, bl: 255 }; 4]);

fn battery_type(t: u8, connected: bool) -> &'static str {
    match (connected, t) {
        (false, _) | (_, 0) => "disconnected",
        (_, 1) => "wired",
        (_, 2) => "alkaline",
        (_, 3) => "nimh",
        _ => "unknown",
    }
}
fn battery_level(l: u8, t: u8) -> &'static str {
    match (t, l) {
        (1, _) => "full", // filaire : pas de pile
        (_, 0) => "empty",
        (_, 1) => "low",
        (_, 2) => "medium",
        (_, 3) => "full",
        _ => "unknown",
    }
}

// ---------- premier plan ----------

#[cfg(windows)]
fn exe_of(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 520];
        let mut len = buf.len() as u32;
        let r = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        r.ok()?;
        let p = String::from_utf16_lossy(&buf[..len as usize]);
        p.rsplit(['\\', '/']).next().map(|s| s.to_string()).filter(|s| !s.is_empty())
    }
}

#[cfg(windows)]
fn parent_map() -> HashMap<u32, u32> {
    let mut m = HashMap::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return m };
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..std::mem::zeroed() };
        if Process32FirstW(snap, &mut e).is_ok() {
            loop {
                m.insert(e.th32ProcessID, e.th32ParentProcessID);
                if Process32NextW(snap, &mut e).is_err() {
                    break;
                }
            }
        }
        let _: HANDLE = snap;
        let _ = CloseHandle(snap);
    }
    m
}

/// `pid` descend-il de `me` (7 générations maximum) ?
#[cfg_attr(not(windows), allow(dead_code))]
fn descends_from(pid: u32, me: u32, parents: &HashMap<u32, u32>) -> bool {
    let mut cur = pid;
    for _ in 0..7 {
        match parents.get(&cur) {
            Some(&p) if p == me => return true,
            Some(&p) if p != 0 && p != cur => cur = p,
            _ => return false,
        }
    }
    false
}

#[cfg(windows)]
#[derive(Default)]
struct FgCache {
    checked: u64,
    ok: bool,
    known: HashMap<u32, (u64, bool, Option<String>)>,
}

#[cfg(windows)]
fn foreground_ok(c: &Config, cache: &mut FgCache, now: u64) -> bool {
    if cache.checked != 0 && now.saturating_sub(cache.checked) < 100 {
        return cache.ok;
    }
    cache.checked = now.max(1);
    let hwnd = rustv_core::input::foreground();
    let mut pid = 0u32;
    if !hwnd.0.is_null() {
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    }
    cache.ok = if pid == 0 {
        false // pas de fenêtre (bureau sécurisé, écran verrouillé) : inconnu
    } else {
        let me = std::process::id();
        if pid == me {
            true
        } else {
            if cache.known.len() > 64 {
                cache.known.clear();
            }
            let fresh = cache.known.get(&pid).is_some_and(|k| now.saturating_sub(k.0) < 5000);
            if !fresh {
                let child = descends_from(pid, me, &parent_map());
                cache.known.insert(pid, (now, child, exe_of(pid)));
            }
            let (_, child, exe) = cache.known.get(&pid).cloned().unwrap_or((0, false, None));
            fg_allowed(&FgInfo { is_self: false, is_child: child, exe }, &c.allow, &c.deny)
        }
    };
    cache.ok
}

// ---------- fil d'interrogation ----------

#[cfg(windows)]
fn run(app: AppHandle) {
    let Some(xi) = XI.get_or_init(XInput::load).as_ref() else {
        eprintln!("gamepad : XInput indisponible, manette désactivée");
        return;
    };
    let core = app.state::<Shell>().core.clone();
    let (tx, _rx) = channel::<PcMsg>();
    let t0 = Instant::now();
    let mut rts: [PadRt; 4] = Default::default();
    let mut rumble: [Vec<(u64, u16)>; 4] = Default::default();
    let mut connected = [false; 4];
    let mut last_probe = 0u64;
    let mut fg = FgCache::default();
    let mut bat_at = [0u64; 4];

    loop {
        let now = t0.elapsed().as_millis() as u64 + 1;
        let probe = now.saturating_sub(last_probe) >= 1000;
        if probe {
            last_probe = now;
        }
        let c = cfg();
        let tv_ok = !c.only_when_tv || app.state::<Shell>().tv.lock().map(|t| t.active).unwrap_or(false);
        let fg_ok = tv_ok && foreground_ok(&c, &mut fg, now);
        let gate = Gate { fg_ok, on: c.enabled && !PAUSED.load(Ordering::Relaxed) };

        for slot in 0..4usize {
            if !connected[slot] && !probe {
                continue;
            }
            let Some(inp) = xi.state(slot as u32) else {
                if connected[slot] {
                    connected[slot] = false;
                    rts[slot] = PadRt::default();
                    rumble[slot].clear();
                    PADS.lock().unwrap()[slot] = Pad { connected: false, bt: 255, bl: 255 };
                    let _ = app.emit("gamepad-changed", json!({ "slot": slot, "connected": false }));
                }
                continue;
            };
            if !connected[slot] {
                connected[slot] = true;
                rts[slot] = PadRt::default();
                bat_at[slot] = 0;
                if gate.fg_ok && gate.on {
                    rumble[slot] = pulse_plan(c.vibration, true).into_iter().map(|(o, s)| (now + o, s)).collect();
                }
                PADS.lock().unwrap()[slot].connected = true;
                let _ = app.emit("gamepad-changed", json!({ "slot": slot, "connected": true }));
            }
            if now >= bat_at[slot] {
                bat_at[slot] = now + 5000;
                let (bt, bl) = xi.battery(slot as u32);
                let mut p = PADS.lock().unwrap();
                p[slot].bt = bt;
                p[slot].bl = bl;
            }
            for o in step(&mut rts[slot], &inp, now, &c, &gate) {
                match o {
                    Out::Msg(m) => core.handle(m, &tx),
                    Out::Rumble(plan) => rumble[slot].extend(plan.into_iter().map(|(off, s)| (now + off, s))),
                    Out::TogglePause => {
                        let paused = !PAUSED.fetch_xor(true, Ordering::Relaxed);
                        eprintln!("gamepad : mapping {}", if paused { "suspendu" } else { "repris" });
                        let _ = app.emit("gamepad-changed", json!({ "paused": paused }));
                        rumble[slot].extend([(now, 30000u16), (now + 220, 0)]);
                    }
                }
            }
            // Exécute les impulsions échues (la vitesse 0 arrête le moteur).
            while rumble[slot].first().is_some_and(|(at, _)| *at <= now) {
                let (_, s) = rumble[slot].remove(0);
                xi.rumble(slot as u32, s);
            }
        }
        let any = connected.iter().any(|c| *c);
        std::thread::sleep(Duration::from_millis(if any { 8 } else { 250 }));
    }
}

/// Démarre le fil d'interrogation XInput et suit le mode TV (la pause du combo est levée à chaque entrée).
pub fn start(app: &AppHandle) {
    let _ = cfg();
    crate::hooks::on_tv_mode(|_app, on| {
        if on {
            PAUSED.store(false, Ordering::Relaxed);
        }
    });
    #[cfg(windows)]
    {
        let h = app.clone();
        std::thread::spawn(move || run(h));
    }
    // Hors Windows : pas de backend manette (voir docs/PORTING.md : evdev/gilrs sous Linux, GameController sous macOS).
    #[cfg(not(windows))]
    let _ = app;
}

// ---------- commandes ----------

#[tauri::command]
pub async fn gamepad_get() -> Result<Value, String> {
    let pads: Vec<Value> = PADS
        .lock()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, p)| json!({ "slot": i, "connected": p.connected, "batteryType": battery_type(p.bt, p.connected), "batteryLevel": if p.connected { battery_level(p.bl, p.bt) } else { "unknown" } }))
        .collect();
    #[cfg(windows)]
    let available = XI.get().map(|x| x.is_some()).unwrap_or(true);
    #[cfg(not(windows))]
    let available = false;
    Ok(json!({ "config": cfg(), "pads": pads, "paused": PAUSED.load(Ordering::Relaxed), "available": available }))
}

#[tauri::command]
pub async fn gamepad_set(cfg: Config) -> Result<Config, String> {
    let c = sanitize(cfg);
    store::save("gamepad", &c)?;
    *CFG.lock().unwrap() = Some(c.clone());
    Ok(c)
}

#[tauri::command]
pub async fn gamepad_test_vibration(slot: u8) -> Result<(), String> {
    if slot > 3 || !PADS.lock().unwrap()[slot as usize].connected {
        return Err("manette absente".into());
    }
    #[cfg(not(windows))]
    return Err(rustv_core::platform::UNSUPPORTED.to_string());
    #[cfg(windows)]
    {
        let Some(xi) = XI.get().and_then(|x| x.as_ref()) else { return Err("XInput indisponible".into()) };
        let level = cfg().vibration.max(2);
        let speed = pulse_plan(level, false).first().map(|p| p.1).unwrap_or(26000);
        xi.rumble(slot as u32, speed);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            if let Some(Some(xi)) = XI.get() {
                xi.rumble(slot as u32, 0);
            }
        });
        Ok(())
    }
}

// ---------- tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_() -> Config {
        Config::default()
    }
    fn on() -> Gate {
        Gate { fg_ok: true, on: true }
    }
    fn keys(o: &[Out]) -> Vec<Key> {
        o.iter().filter_map(|x| if let Out::Msg(PhoneMsg::Key { k }) = x { Some(*k) } else { None }).collect()
    }
    fn armed() -> PadRt {
        let mut rt = PadRt::default();
        step(&mut rt, &Input::default(), 1, &cfg_(), &on());
        assert!(rt.armed);
        rt
    }

    #[test]
    fn stick_deadzone_and_axis() {
        assert_eq!(stick_dir(0.2, 0.2, 0.35, None), None);
        assert_eq!(stick_dir(0.9, 0.1, 0.35, None), Some(Dir::Right));
        assert_eq!(stick_dir(-0.9, 0.1, 0.35, None), Some(Dir::Left));
        assert_eq!(stick_dir(0.1, 0.9, 0.35, None), Some(Dir::Up));
        assert_eq!(stick_dir(0.1, -0.9, 0.35, None), Some(Dir::Down));
        // diagonale : l'axe actif garde la priorité
        assert_eq!(stick_dir(0.6, 0.7, 0.35, Some(Dir::Right)), Some(Dir::Right));
        assert_eq!(stick_dir(0.7, 0.6, 0.35, Some(Dir::Up)), Some(Dir::Up));
        assert_eq!(stick_dir(0.6, 0.7, 0.35, None), Some(Dir::Up));
        // hystérésis : maintenu un peu sous la zone morte
        assert_eq!(stick_dir(0.30, 0.0, 0.35, None), None);
        assert_eq!(stick_dir(0.30, 0.0, 0.35, Some(Dir::Right)), Some(Dir::Right));
    }

    #[test]
    fn dpad_priority() {
        assert_eq!(dpad_dir(DPAD_UP | DPAD_LEFT), Some(Dir::Up));
        assert_eq!(dpad_dir(DPAD_RIGHT), Some(Dir::Right));
        assert_eq!(dpad_dir(BTN_A), None);
    }

    #[test]
    fn repeat_timing() {
        let mut r = Repeater::<Dir>::default();
        assert_eq!(r.update(0, Some(Dir::Left), 380, 90), Some(Dir::Left));
        assert_eq!(r.update(100, Some(Dir::Left), 380, 90), None);
        assert_eq!(r.update(379, Some(Dir::Left), 380, 90), None);
        assert_eq!(r.update(380, Some(Dir::Left), 380, 90), Some(Dir::Left));
        assert_eq!(r.update(400, Some(Dir::Left), 380, 90), None);
        assert_eq!(r.update(470, Some(Dir::Left), 380, 90), Some(Dir::Left));
        // changement de direction : tir immédiat et nouveau délai
        assert_eq!(r.update(480, Some(Dir::Up), 380, 90), Some(Dir::Up));
        assert_eq!(r.update(700, Some(Dir::Up), 380, 90), None);
        assert_eq!(r.update(0, None, 380, 90), None);
        assert_eq!(r.update(1000, Some(Dir::Up), 380, 90), Some(Dir::Up));
    }

    #[test]
    fn edge_detection() {
        assert_eq!(edges(0, BTN_A), (BTN_A, 0));
        assert_eq!(edges(BTN_A | BTN_B, BTN_B | BTN_X), (BTN_X, BTN_A));
        assert_eq!(edges(START, START), (0, 0));
    }

    #[test]
    fn mapping_and_swap() {
        assert_eq!(map_button(BTN_A, false, false), Act::Key(Key::Enter));
        assert_eq!(map_button(BTN_B, false, false), Act::Back);
        assert_eq!(map_button(BTN_A, true, false), Act::Back);
        assert_eq!(map_button(BTN_B, true, false), Act::Key(Key::Enter));
        assert_eq!(map_button(BTN_Y, false, false), Act::Key(Key::Search));
        assert_eq!(map_button(BTN_X, true, false), Act::Key(Key::Menu));
        assert_eq!(map_button(R3, false, false), Act::None);
        assert_eq!(map_button(R3, false, true), Act::Click(false));
        assert_eq!(map_button(L3, false, false), Act::Media(MediaAction::PlayPause));
    }

    #[test]
    fn tab_switcher() {
        let mut last = None;
        assert_eq!(tab_msgs(false, &mut last, 100), vec![AltTab::Open]);
        assert_eq!(tab_msgs(true, &mut last, 600), vec![AltTab::Left]);
        assert_eq!(tab_msgs(false, &mut last, 900), vec![AltTab::Right]);
        assert_eq!(tab_msgs(true, &mut last, 900 + ALT_WINDOW_MS + 1), vec![AltTab::Open, AltTab::Left]);
    }

    #[test]
    fn foreground_decision() {
        let own = FgInfo { is_self: true, is_child: false, exe: Some("rustv.exe".into()) };
        let child = FgInfo { is_self: false, is_child: true, exe: Some("mpv.exe".into()) };
        let game = FgInfo { is_self: false, is_child: false, exe: Some("Game.EXE".into()) };
        let unknown = FgInfo { is_self: false, is_child: false, exe: None };
        let none: Vec<String> = vec![];
        assert!(fg_allowed(&own, &none, &["rustv".into()]));
        assert!(fg_allowed(&child, &none, &none));
        assert!(!fg_allowed(&child, &none, &["MPV.exe".into()]));
        assert!(!fg_allowed(&game, &none, &none));
        assert!(fg_allowed(&game, &["game".into()], &none));
        assert!(!fg_allowed(&game, &["game.exe".into()], &["game".into()]));
        assert!(!fg_allowed(&unknown, &["x".into()], &none));
    }

    #[test]
    fn combo_timing() {
        let mut c = Combo::default();
        assert!(!c.update(0, true));
        assert!(!c.update(2999, true));
        assert!(c.update(3000, true));
        assert!(!c.update(3500, true)); // une seule fois
        assert!(!c.update(3600, false));
        assert!(!c.update(4000, true)); // réarmé
        assert!(c.update(7000, true));
        let mut d = Combo::default();
        assert!(!d.update(0, true));
        assert!(!d.update(2000, false));
        assert!(!d.update(3500, true)); // relâché : le délai repart de zéro
    }

    #[test]
    fn press_a_and_swap() {
        let mut rt = armed();
        let o = step(&mut rt, &Input { buttons: BTN_A, ..Default::default() }, 10, &cfg_(), &on());
        assert_eq!(keys(&o), vec![Key::Enter]);
        step(&mut rt, &Input::default(), 20, &cfg_(), &on());
        let mut c = cfg_();
        c.swap_ab = true;
        let mut rt = armed();
        let o = step(&mut rt, &Input { buttons: BTN_A, ..Default::default() }, 10, &c, &on());
        assert!(keys(&o).is_empty()); // A = Retour : émis au relâchement
        let o = step(&mut rt, &Input::default(), 50, &c, &on());
        assert_eq!(keys(&o), vec![Key::Back]);
    }

    #[test]
    fn back_long_press() {
        let mut rt = armed();
        let hold = Input { buttons: BTN_B, ..Default::default() };
        assert!(keys(&step(&mut rt, &hold, 10, &cfg_(), &on())).is_empty());
        assert!(keys(&step(&mut rt, &hold, 500, &cfg_(), &on())).is_empty());
        assert_eq!(keys(&step(&mut rt, &hold, 700, &cfg_(), &on())), vec![Key::BackLong]);
        assert!(keys(&step(&mut rt, &hold, 900, &cfg_(), &on())).is_empty());
        assert!(keys(&step(&mut rt, &Input::default(), 1000, &cfg_(), &on())).is_empty());
    }

    #[test]
    fn dpad_repeats_and_start_home() {
        let mut rt = armed();
        let d = Input { buttons: DPAD_DOWN, ..Default::default() };
        assert_eq!(keys(&step(&mut rt, &d, 10, &cfg_(), &on())), vec![Key::Down]);
        assert!(keys(&step(&mut rt, &d, 200, &cfg_(), &on())).is_empty());
        assert_eq!(keys(&step(&mut rt, &d, 400, &cfg_(), &on())), vec![Key::Down]);
        step(&mut rt, &Input::default(), 410, &cfg_(), &on());
        // Vue seule = Accueil au relâchement
        step(&mut rt, &Input { buttons: BACK, ..Default::default() }, 500, &cfg_(), &on());
        let o = step(&mut rt, &Input::default(), 520, &cfg_(), &on());
        assert!(o.contains(&Out::Msg(PhoneMsg::Home)));
        // Start + Vue : ni Menu ni Accueil
        step(&mut rt, &Input { buttons: START, ..Default::default() }, 600, &cfg_(), &on());
        step(&mut rt, &Input { buttons: START | BACK, ..Default::default() }, 610, &cfg_(), &on());
        let o = step(&mut rt, &Input::default(), 700, &cfg_(), &on());
        assert!(o.is_empty());
    }

    #[test]
    fn inactive_never_emits_and_rearms() {
        let mut rt = PadRt::default();
        let off = Gate { fg_ok: false, on: true };
        let o = step(&mut rt, &Input { buttons: BTN_A | DPAD_UP, ..Default::default() }, 10, &cfg_(), &off);
        assert!(o.is_empty());
        // le jeu rend le focus avec le bouton encore enfoncé : rien tant que tout n'est pas relâché
        let held = Input { buttons: BTN_A, ..Default::default() };
        assert!(step(&mut rt, &held, 20, &cfg_(), &on()).is_empty());
        assert!(step(&mut rt, &Input::default(), 30, &cfg_(), &on()).is_empty());
        assert_eq!(keys(&step(&mut rt, &held, 40, &cfg_(), &on())), vec![Key::Enter]);
        // désactivée : jamais rien
        let mut rt = armed();
        let disabled = Gate { fg_ok: true, on: false };
        assert!(step(&mut rt, &held, 50, &cfg_(), &disabled).is_empty());
    }

    #[test]
    fn escape_combo_toggles_even_when_paused() {
        let mut rt = armed();
        let both = Input { buttons: START | BACK, ..Default::default() };
        let paused = Gate { fg_ok: true, on: false };
        assert!(step(&mut rt, &both, 100, &cfg_(), &paused).is_empty());
        let o = step(&mut rt, &both, 3100, &cfg_(), &paused);
        assert!(o.contains(&Out::TogglePause));
        // dans un jeu (premier plan refusé) le combo est ignoré
        let mut rt2 = PadRt::default();
        let game = Gate { fg_ok: false, on: true };
        step(&mut rt2, &both, 0, &cfg_(), &game);
        assert!(step(&mut rt2, &both, 5000, &cfg_(), &game).is_empty());
    }

    #[test]
    fn triggers_and_scroll() {
        let mut rt = armed();
        let rtg = Input { rt: 200, ..Default::default() };
        let o = step(&mut rt, &rtg, 10, &cfg_(), &on());
        assert!(o.contains(&Out::Msg(PhoneMsg::Media { a: MediaAction::VolUp })));
        let lt = Input { lt: 255, ..Default::default() };
        let o = step(&mut rt, &lt, 20, &cfg_(), &on());
        assert!(o.contains(&Out::Msg(PhoneMsg::Media { a: MediaAction::VolDown })));
        step(&mut rt, &Input::default(), 30, &cfg_(), &on());
        let up = Input { ry: 32767, ..Default::default() };
        let mut total = 0;
        for i in 1..=20u64 {
            for o in step(&mut rt, &up, 30 + i * 8, &cfg_(), &on()) {
                if let Out::Msg(PhoneMsg::Mouse { scroll, .. }) = o {
                    total += scroll;
                }
            }
        }
        assert!(total > 0);
    }

    #[test]
    fn vibration_plans_and_sanitize() {
        assert!(pulse_plan(0, true).is_empty());
        assert_eq!(pulse_plan(1, false).len(), 2);
        assert_eq!(pulse_plan(3, true).len(), 4);
        assert!(pulse_plan(9, false)[0].1 > pulse_plan(1, false)[0].1);
        let c = sanitize(Config { deadzone: 5.0, repeat_delay_ms: 1, repeat_rate_ms: 99999, vibration: 9, allow: vec![" VLC.exe ".into(), "vlc.exe".into(), "C:\\x.exe".into()], ..Config::default() });
        assert_eq!((c.deadzone, c.repeat_delay_ms, c.repeat_rate_ms, c.vibration), (0.9, 150, 400, 3));
        assert_eq!(c.allow, vec!["vlc.exe".to_string()]);
        assert_eq!(sanitize(Config { deadzone: f32::NAN, ..Config::default() }).deadzone, 0.35);
    }

    #[test]
    fn battery_labels() {
        assert_eq!(battery_type(1, true), "wired");
        assert_eq!(battery_type(2, false), "disconnected");
        assert_eq!(battery_level(1, 2), "low");
        assert_eq!(battery_level(0, 1), "full");
    }

    #[test]
    fn config_json_shape() {
        let c: Config = serde_json::from_str(r#"{"swapAB":true}"#).unwrap();
        assert!(c.enabled && c.swap_ab);
        assert!(serde_json::to_string(&c).unwrap().contains("swapAB"));
        let c: Config = serde_json::from_str(r#"{"swapAb":true,"repeatDelayMs":500}"#).unwrap();
        assert!(c.swap_ab && c.repeat_delay_ms == 500);
    }
}
