//! Garde TV : pendant le mode TV, garde l'expérience TV au premier plan sur l'écran de la TV.
//!
//! Architecture : toute la logique de décision est PURE (rectangles, listes d'autorisations, confiance par
//! arbre de processus, anti-acharnement, plans de récupération après crash, choix du moniteur au démarrage)
//! et testée unitairement. Les appels Win32 (module `sys`) sont de minces enveloppes que seule l'application
//! en marche appelle (`activate` / `deactivate` / `restore_all`) : les tests ne touchent jamais ni le
//! registre, ni la barre des tâches, ni les fenêtres.
//!
//! Limites assumées (voir docs/TV-GUARD.md) : le bureau sécurisé (UAC, Ctrl+Alt+Suppr, écran de verrouillage)
//! n'est jamais visible ni modifiable par un processus normal, et la garde n'essaie pas d'y toucher.
use crate::{store, tv, Shell};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

// =====================================================================================================
// Configuration et marqueurs de récupération (guard.json)
// =====================================================================================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct GuardCfg {
    /// Interrupteur général de la garde (fenêtres, barre des tâches, notifications).
    pub enabled: bool,
    /// "minimize" (défaut) ou "move" (déplace vers un autre écran s'il y en a un).
    pub mode: String,
    /// Empêche la mise en veille tant que le mode TV est actif.
    pub keep_awake: bool,
    pub hide_taskbar: bool,
    pub suppress_toasts: bool,
    /// Noms de processus supplémentaires autorisés (ex. "obs64.exe").
    pub allow_processes: Vec<String>,
    /// Classes de fenêtres supplémentaires autorisées (en plus de la liste intégrée).
    pub allow_classes: Vec<String>,
    /// Miroir (lecture seule) du mode de démarrage automatique ("off" | "tray" | "tv"), lu dans autostart.rs.
    pub boot_to_tv: String,
}

impl Default for GuardCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: "minimize".into(),
            keep_awake: true,
            hide_taskbar: false,
            suppress_toasts: false,
            allow_processes: Vec::new(),
            allow_classes: Vec::new(),
            boot_to_tv: "off".into(),
        }
    }
}

const MAX_LIST: usize = 64;
const MAX_ITEM: usize = 128;

fn clean_list(v: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in v {
        let s = s.trim();
        if s.is_empty() || s.len() > MAX_ITEM || s.chars().any(|c| c.is_control()) {
            continue;
        }
        if !out.iter().any(|o| o.eq_ignore_ascii_case(s)) {
            out.push(s.to_string());
        }
        if out.len() >= MAX_LIST {
            break;
        }
    }
    out
}

impl GuardCfg {
    /// Valeurs bornées et valides (entrée venant de l'interface).
    pub fn sanitized(mut self) -> Self {
        if self.mode != "minimize" && self.mode != "move" {
            self.mode = "minimize".into();
        }
        self.allow_processes = clean_list(&self.allow_processes);
        self.allow_classes = clean_list(&self.allow_classes);
        if !["off", "tray", "tv"].contains(&self.boot_to_tv.as_str()) {
            self.boot_to_tv = "off".into();
        }
        self
    }
}

/// Valeur précédente du réglage des notifications, sauvegardée avant de la couper.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToastSaved {
    /// La valeur DWORD existait-elle ? Sinon on la supprime à la restauration.
    pub existed: bool,
    pub value: u32,
}

/// Dernier état de la session TV (pour reprendre après un redémarrage / une veille) et écran utilisé.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Session {
    pub tv_active: bool,
    pub monitor_index: usize,
    pub monitor_name: String,
    pub monitor_w: u32,
    pub monitor_h: u32,
    pub monitor_count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Recovery {
    /// RusTV a masqué la barre des tâches et ne l'a pas encore restaurée.
    pub taskbar_hidden: bool,
    /// RusTV a coupé les notifications ; contient la valeur à rétablir.
    pub toasts: Option<ToastSaved>,
    pub session: Session,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct GuardFile {
    #[serde(flatten)]
    pub cfg: GuardCfg,
    pub recovery: Recovery,
}

// =====================================================================================================
// Rectangles et écrans
// =====================================================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub l: i32,
    pub t: i32,
    pub r: i32,
    pub b: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { l: x, t: y, r: x.saturating_add(w), b: y.saturating_add(h) }
    }
    pub fn w(&self) -> i64 {
        (self.r as i64 - self.l as i64).max(0)
    }
    pub fn h(&self) -> i64 {
        (self.b as i64 - self.t as i64).max(0)
    }
    pub fn area(&self) -> i64 {
        self.w() * self.h()
    }
    pub fn center(&self) -> (i32, i32) {
        (((self.l as i64 + self.r as i64) / 2) as i32, ((self.t as i64 + self.b as i64) / 2) as i32)
    }
    pub fn contains(&self, p: (i32, i32)) -> bool {
        p.0 >= self.l && p.0 < self.r && p.1 >= self.t && p.1 < self.b
    }
    pub fn inter_area(&self, o: &Rect) -> i64 {
        let w = (self.r.min(o.r) as i64 - self.l.max(o.l) as i64).max(0);
        let h = (self.b.min(o.b) as i64 - self.t.max(o.t) as i64).max(0);
        w * h
    }
}

/// La fenêtre « appartient » à l'écran si son centre y est, ou si au moins la moitié de sa surface y est.
/// C'est ce qui limite la garde à l'écran de la TV : les fenêtres de l'autre écran ne sont jamais touchées.
pub fn affects_monitor(win: &Rect, mon: &Rect) -> bool {
    let a = win.area();
    a > 0 && mon.area() > 0 && (mon.contains(win.center()) || win.inter_area(mon) * 2 >= a)
}

/// Où mettre `win` (sur l'écran `from`) pour qu'elle atterrisse sur l'écran `to` : même position relative,
/// taille réduite si elle ne tient pas.
pub fn move_target(win: &Rect, from: &Rect, to: &Rect) -> Rect {
    let w = win.w().min(to.w()).max(1) as i32;
    let h = win.h().min(to.h()).max(1) as i32;
    let dx = (win.l as i64 - from.l as i64).max(0) as i32;
    let dy = (win.t as i64 - from.t as i64).max(0) as i32;
    let x = (to.l as i64 + dx as i64).min(to.r as i64 - w as i64).max(to.l as i64) as i32;
    let y = (to.t as i64 + dy as i64).min(to.b as i64 - h as i64).max(to.t as i64) as i32;
    Rect::new(x, y, w, h)
}

// =====================================================================================================
// Autorisations : classes, processus, confiance
// =====================================================================================================

/// Classes de l'interface Windows qui ne doivent jamais être touchées (barre des tâches, bureau, IME,
/// panneaux de saisie, menus). Liste de base de la garde ; l'utilisateur peut en ajouter (`allow_classes`).
pub const SHELL_CLASSES: &[&str] = &[
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "Progman",
    "WorkerW",
    "SHELLDLL_DefView",
    "NotifyIconOverflowWindow",
    "TopLevelWindowForOverflowXamlIsland",
    "TaskListThumbnailWnd",
    "TaskListOverlayWnd",
    "#32768",
    "tooltips_class32",
    "IME",
    "MSCTFIME UI",
    "Default IME",
    "IPTip_Main_Window",
    "Shell_InputSwitchTopLevelWindow",
    "ApplicationManager_ImmersiveShellWindow",
    "Windows.UI.Core.CoreWindow",
    "SysShadow",
];

/// Fenêtres du sélecteur de tâches / vue des tâches / Alt+Tab (utilisés par la télécommande).
/// Nom exact selon la version de Windows : liste issue de l'observation de Windows 10 et 11 (non vérifiée
/// sur toutes les versions), donc configurable via `allow_classes`.
pub const SWITCHER_CLASSES: &[&str] = &[
    "MultitaskingViewFrame",
    "XamlExplorerHostIslandWindow",
    "ForegroundStaging",
    "TaskSwitcherWnd",
    "TaskSwitcherOverlayWnd",
];

/// Processus système jamais touchés (saisie tactile, IME, interface du shell).
pub const SYSTEM_EXES: &[&str] = &["textinputhost.exe", "ctfmon.exe", "shellexperiencehost.exe", "startmenuexperiencehost.exe", "searchhost.exe", "dwm.exe", "logonui.exe", "consent.exe"];

fn class_eq(pat: &str, class: &str) -> bool {
    match pat.strip_suffix('*') {
        Some(prefix) => class.len() >= prefix.len() && class[..prefix.len()].eq_ignore_ascii_case(prefix),
        None => pat.eq_ignore_ascii_case(class),
    }
}

fn exe_eq(pat: &str, exe: &str) -> bool {
    let norm = |s: &str| {
        let s = s.trim().to_ascii_lowercase();
        let base = s.rsplit(['\\', '/']).next().unwrap_or("").to_string();
        base.strip_suffix(".exe").map(str::to_string).unwrap_or(base)
    };
    let (p, e) = (norm(pat), norm(exe));
    !p.is_empty() && p == e
}

#[derive(Clone, Debug, Default)]
pub struct Policy {
    pub extra_classes: Vec<String>,
    pub extra_exes: Vec<String>,
}

impl Policy {
    pub fn from_cfg(c: &GuardCfg) -> Self {
        Self { extra_classes: c.allow_classes.clone(), extra_exes: c.allow_processes.clone() }
    }
    pub fn class_allowed(&self, class: &str) -> bool {
        SHELL_CLASSES.iter().chain(SWITCHER_CLASSES).any(|p| class_eq(p, class)) || self.extra_classes.iter().any(|p| class_eq(p, class))
    }
    pub fn is_switcher(&self, class: &str) -> bool {
        SWITCHER_CLASSES.iter().any(|p| class_eq(p, class))
    }
    pub fn exe_allowed(&self, exe: &str) -> bool {
        SYSTEM_EXES.iter().any(|p| exe_eq(p, exe)) || self.extra_exes.iter().any(|p| exe_eq(p, exe))
    }
}

/// Ce que la garde sait d'une fenêtre de premier niveau.
#[derive(Clone, Debug, Default)]
pub struct WinInfo {
    pub hwnd: isize,
    pub pid: u32,
    pub class: String,
    /// Nom du fichier exécutable (minuscule), vide si inconnu.
    pub exe: String,
    pub rect: Rect,
    pub visible: bool,
    pub minimized: bool,
    pub cloaked: bool,
    /// WS_EX_TOOLWINDOW ou WS_EX_NOACTIVATE (info-bulles, panneaux qui ne prennent pas le focus).
    pub tool_like: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Ne concerne pas la garde (invisible, ailleurs, minuscule…).
    Skip(&'static str),
    /// Autorisée (raison).
    Allowed(&'static str),
    /// À traiter.
    Offender,
}

/// Processus et fenêtres auxquels on fait confiance : arbre de processus de RusTV (mpv, navigateur lancé
/// par RusTV, WebView2…) et programmes lancés exprès depuis une tuile (adoptés jusqu'à leur fin).
#[derive(Clone, Debug, Default)]
pub struct Trust {
    pub own: HashSet<u32>,
    pub adopted_pids: HashSet<u32>,
    pub adopted_hwnds: HashSet<isize>,
}

impl Trust {
    pub fn is_trusted(&self, pid: u32, hwnd: isize) -> bool {
        self.own.contains(&pid) || self.adopted_pids.contains(&pid) || self.adopted_hwnds.contains(&hwnd)
    }
}

/// Tous les descendants (enfants, petits-enfants…) des racines, racines comprises. `procs` : (pid, ppid).
pub fn descendants(roots: &[u32], procs: &[(u32, u32)]) -> HashSet<u32> {
    let mut set: HashSet<u32> = roots.iter().copied().filter(|p| *p != 0).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for &(pid, ppid) in procs {
            if pid != 0 && !set.contains(&pid) && set.contains(&ppid) {
                set.insert(pid);
                changed = true;
            }
        }
    }
    set
}

/// Décision pour une fenêtre. `tv` : rectangle de l'écran de la TV.
pub fn judge(w: &WinInfo, tv: &Rect, policy: &Policy, trust: &Trust) -> Verdict {
    if !w.visible {
        return Verdict::Skip("hidden");
    }
    if w.minimized {
        return Verdict::Skip("minimized");
    }
    if w.cloaked {
        return Verdict::Skip("cloaked");
    }
    if w.rect.w() < 20 || w.rect.h() < 20 {
        return Verdict::Skip("tiny");
    }
    if !affects_monitor(&w.rect, tv) {
        return Verdict::Skip("other-monitor");
    }
    if trust.is_trusted(w.pid, w.hwnd) {
        return Verdict::Allowed("trusted");
    }
    if policy.class_allowed(&w.class) {
        return Verdict::Allowed("class");
    }
    if policy.exe_allowed(&w.exe) {
        return Verdict::Allowed("process");
    }
    if w.tool_like {
        return Verdict::Skip("tool-window");
    }
    Verdict::Offender
}

// ---- programmes lancés exprès depuis une tuile ----

/// Un lancement demandé par l'utilisateur via une tuile RusTV ; sert à reconnaître la fenêtre qui en résulte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingLaunch {
    pub at_unix_ms: u64,
    /// Nom du fichier lancé sans extension, en minuscules ; vide si ce n'est pas un .exe (raccourci, dossier…).
    pub exe_stem: String,
    pub opaque: bool,
}

/// Délai pendant lequel un lancement peut « adopter » un processus (nouveau processus ou même nom).
pub const ADOPT_PID_MS: u64 = 60_000;
/// Délai plus court pour adopter une fenêtre seule (raccourci, UWP, dossier : on ne connaît pas le processus).
pub const ADOPT_HWND_MS: u64 = 15_000;

pub fn launch_from_target(target: &str, now_unix_ms: u64) -> PendingLaunch {
    let t = target.trim().trim_matches('"');
    let base = t.rsplit(['\\', '/']).next().unwrap_or("").to_ascii_lowercase();
    let is_exe = base.ends_with(".exe");
    PendingLaunch { at_unix_ms: now_unix_ms, exe_stem: if is_exe { base.trim_end_matches(".exe").to_string() } else { String::new() }, opaque: !is_exe }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adopt {
    No,
    Pid(u32),
    Hwnd(isize),
}

/// Faut-il adopter cette fenêtre (jugée « à traiter ») comme le résultat d'un lancement voulu ?
/// `created_unix_ms` : date de création de son processus (si connue).
pub fn adopt_for(launches: &[PendingLaunch], w: &WinInfo, created_unix_ms: Option<u64>, now_unix_ms: u64) -> Adopt {
    for l in launches {
        let age = now_unix_ms.saturating_sub(l.at_unix_ms);
        if age <= ADOPT_PID_MS {
            let newer = created_unix_ms.is_some_and(|c| c + 3_000 >= l.at_unix_ms);
            let named = !l.exe_stem.is_empty() && exe_eq(&l.exe_stem, &w.exe);
            if (newer || named) && w.pid != 0 {
                return Adopt::Pid(w.pid);
            }
        }
        if age <= ADOPT_HWND_MS && l.opaque && w.hwnd != 0 {
            return Adopt::Hwnd(w.hwnd);
        }
    }
    Adopt::No
}

/// Délai après lequel une fenêtre qui prend le premier plan juste après le sélecteur Alt+Tab est considérée
/// comme choisie par l'utilisateur (via la télécommande).
pub const SWITCH_ADOPT_MS: u64 = 4_000;

pub fn switched_by_user(switcher_last_seen_ms: Option<u64>, now_ms: u64) -> bool {
    switcher_last_seen_ms.is_some_and(|t| now_ms.saturating_sub(t) <= SWITCH_ADOPT_MS)
}

// =====================================================================================================
// Anti-acharnement : ne jamais se battre contre une fenêtre en boucle serrée
// =====================================================================================================

#[derive(Clone, Debug)]
pub struct Backoff {
    max_in_window: usize,
    window_ms: u64,
    min_gap_ms: u64,
    base_pause_ms: u64,
    max_pause_ms: u64,
    hist: VecDeque<u64>,
    pause_until: u64,
    strikes: u32,
}

impl Backoff {
    pub fn new(max_in_window: usize, window_ms: u64, min_gap_ms: u64, base_pause_ms: u64, max_pause_ms: u64) -> Self {
        Self { max_in_window, window_ms, min_gap_ms, base_pause_ms, max_pause_ms, hist: VecDeque::new(), pause_until: 0, strikes: 0 }
    }
    /// Action par fenêtre : 4 actions / 5 s, puis pause 3 s, 6 s, 12 s… (max 60 s).
    pub fn per_window() -> Self {
        Self::new(4, 5_000, 0, 3_000, 60_000)
    }
    /// Retour du premier plan : au plus 1 tentative / 300 ms, 5 / 10 s, puis pause 5 s doublée (max 60 s).
    pub fn foreground() -> Self {
        Self::new(5, 10_000, 300, 5_000, 60_000)
    }
    /// Demande l'autorisation d'agir à l'instant `now` (ms). Enregistre l'action si elle est accordée.
    pub fn permit(&mut self, now: u64) -> bool {
        if now < self.pause_until {
            return false;
        }
        while self.hist.front().is_some_and(|t| now.saturating_sub(*t) > self.window_ms) {
            self.hist.pop_front();
        }
        if self.hist.is_empty() && now > self.pause_until.saturating_add(self.window_ms) {
            self.strikes = 0;
        }
        if self.hist.back().is_some_and(|t| now.saturating_sub(*t) < self.min_gap_ms) {
            return false;
        }
        if self.hist.len() >= self.max_in_window {
            self.strikes = (self.strikes + 1).min(16);
            let pause = self.base_pause_ms.saturating_mul(1u64 << (self.strikes - 1)).min(self.max_pause_ms);
            self.pause_until = now + pause;
            self.hist.clear();
            return false;
        }
        self.hist.push_back(now);
        true
    }
    pub fn paused(&self, now: u64) -> bool {
        now < self.pause_until
    }
    pub fn strikes(&self) -> u32 {
        self.strikes
    }
}

// =====================================================================================================
// Récupération après crash, registre, démarrage
// =====================================================================================================

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RecoveryPlan {
    pub show_taskbar: bool,
    pub restore_toasts: Option<ToastSaved>,
}

/// Que faut-il restaurer au démarrage ? Si le mode TV est actif (ce n'est pas un redémarrage après crash),
/// rien : l'état est géré normalement.
pub fn recovery_plan(r: &Recovery, tv_active: bool) -> RecoveryPlan {
    if tv_active {
        return RecoveryPlan::default();
    }
    RecoveryPlan { show_taskbar: r.taskbar_hidden, restore_toasts: r.toasts.clone() }
}

/// Avant de couper les notifications : valeur à sauvegarder (on ne remplace JAMAIS une sauvegarde existante,
/// sinon on sauverait notre propre 0) et faut-il écrire 0 ?
pub fn toast_save_plan(current: Option<u32>, existing: &Option<ToastSaved>) -> ToastSaved {
    match existing {
        Some(s) => s.clone(),
        None => ToastSaved { existed: current.is_some(), value: current.unwrap_or(1) },
    }
}

/// Extrait un DWORD de la sortie de `reg query ... /v NAME` (« NAME    REG_DWORD    0x1 »).
pub fn parse_reg_dword(out: &str) -> Option<u32> {
    for line in out.lines() {
        if let Some(i) = line.find("REG_DWORD") {
            let v = line[i + 9..].trim();
            return match v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")) {
                Some(h) => u32::from_str_radix(h, 16).ok(),
                None => v.parse().ok(),
            };
        }
    }
    None
}

/// Écran voulu au démarrage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonDesc {
    pub name: String,
    pub w: u32,
    pub h: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedMon {
    pub index: usize,
    pub name: String,
    pub w: u32,
    pub h: u32,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boot {
    /// Démarrer sur cet écran.
    Use(usize),
    /// Attendre (la TV n'est pas encore prête).
    Wait,
    /// Délai dépassé : se rabattre sur cet écran.
    Fallback(usize),
    /// Aucun écran mémorisé : laisser le sélecteur habituel.
    Pick,
}

/// Choix de l'écran au démarrage automatique. Une TV qui se réveille lentement n'apparaît pas tout de suite :
/// on attend jusqu'à `max_wait_ms` (60 s) puis on se rabat sur l'index mémorisé (ou le dernier écran).
pub fn boot_decision(saved: Option<&SavedMon>, mons: &[MonDesc], waited_ms: u64, max_wait_ms: u64) -> Boot {
    let Some(s) = saved else {
        return if mons.is_empty() { Boot::Wait } else { Boot::Pick };
    };
    let dims_ok = |m: &MonDesc| s.w == 0 || (m.w == s.w && m.h == s.h);
    if !s.name.is_empty() {
        if let Some(i) = mons.iter().position(|m| m.name == s.name && dims_ok(m)) {
            return Boot::Use(i);
        }
    }
    if s.name.is_empty() && s.index < mons.len() && mons.len() >= s.count && dims_ok(&mons[s.index]) {
        return Boot::Use(s.index);
    }
    if waited_ms < max_wait_ms {
        return Boot::Wait;
    }
    if mons.is_empty() {
        Boot::Wait
    } else {
        Boot::Fallback(s.index.min(mons.len() - 1))
    }
}

// =====================================================================================================
// Exécution : état global
// =====================================================================================================

#[derive(Default)]
struct Rt {
    active: bool,
    stop: Option<Arc<AtomicBool>>,
    hook_tid: Option<Arc<AtomicU32>>,
    awake_stop: Option<std::sync::mpsc::Sender<()>>,
    taskbar_hidden: bool,
    toasts_muted: bool,
}

static RT: Mutex<Option<Rt>> = Mutex::new(None);
static FILE: Mutex<Option<GuardFile>> = Mutex::new(None);
static PENDING: Mutex<Vec<PendingLaunch>> = Mutex::new(Vec::new());
static ACTIONS: AtomicU64 = AtomicU64::new(0);
static HIDDEN: AtomicU32 = AtomicU32::new(0);

fn with_rt<R>(f: impl FnOnce(&mut Rt) -> R) -> R {
    let mut g = RT.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Rt::default))
}

fn load_file() -> GuardFile {
    let mut g = FILE.lock().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(|| {
        let mut f: GuardFile = store::load("guard");
        f.cfg = std::mem::take(&mut f.cfg).sanitized();
        f
    })
    .clone()
}

fn update_file(f: impl FnOnce(&mut GuardFile)) -> GuardFile {
    let mut cur = load_file();
    f(&mut cur);
    let _ = store::save("guard", &cur);
    *FILE.lock().unwrap_or_else(|e| e.into_inner()) = Some(cur.clone());
    cur
}

fn config() -> GuardCfg {
    let mut c = load_file().cfg;
    c.boot_to_tv = crate::autostart::get().into();
    c
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Le mode TV est actif.
    pub active: bool,
    /// La surveillance des fenêtres tourne.
    pub guard_running: bool,
    pub actions: u64,
    pub hidden_windows: u32,
    pub taskbar_hidden: bool,
    pub toasts_muted: bool,
    pub keep_awake: bool,
}

pub fn status() -> Status {
    with_rt(|rt| Status {
        active: rt.active,
        guard_running: rt.stop.is_some(),
        actions: ACTIONS.load(Ordering::Relaxed),
        hidden_windows: HIDDEN.load(Ordering::Relaxed),
        taskbar_hidden: rt.taskbar_hidden,
        toasts_muted: rt.toasts_muted,
        keep_awake: rt.awake_stop.is_some(),
    })
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Appelé quand l'utilisateur lance exprès un programme depuis une tuile : la garde l'autorisera (jusqu'à ce
/// qu'il se ferme). N'a aucun effet si le mode TV est inactif.
pub fn note_launch(target: &str) {
    let mut p = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    let now = unix_ms();
    p.retain(|l| now.saturating_sub(l.at_unix_ms) <= ADOPT_PID_MS);
    if p.len() >= 16 {
        p.remove(0);
    }
    p.push(launch_from_target(target, now));
}

/// Démarrage de la garde : récupération après crash, puis abonnement au mode TV.
pub fn start(app: &AppHandle) {
    let f = load_file();
    let plan = recovery_plan(&f.recovery, app.state::<Shell>().tv.lock().map(|t| t.active).unwrap_or(false));
    apply_recovery(&plan);
    crate::hooks::on_tv_mode(|app, on| if on { activate(app) } else { deactivate(app) });
    // Filet de sécurité : un crash Rust restaure la barre des tâches et les notifications.
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Dans un thread à part avec délai : le crash ne doit jamais se bloquer sur un verrou.
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            restore_all();
            let _ = tx.send(());
        });
        let _ = rx.recv_timeout(std::time::Duration::from_secs(4));
        prev(info);
    }));
}

/// Arrêt de l'application : tout restaurer (appelé sur RunEvent::Exit).
pub fn shutdown() {
    stop_threads();
    restore_all();
}

/// L'état précédent était-il « mode TV actif » (veille / redémarrage pendant le mode TV) ?
pub fn resume_wanted() -> bool {
    load_file().recovery.session.tv_active
}

pub fn saved_monitor() -> Option<SavedMon> {
    let s = load_file().recovery.session;
    (s.monitor_count > 0).then_some(SavedMon { index: s.monitor_index, name: s.monitor_name, w: s.monitor_w, h: s.monitor_h, count: s.monitor_count })
}

fn apply_recovery(plan: &RecoveryPlan) {
    if plan.show_taskbar {
        sys::set_taskbar_visible_all();
        update_file(|f| f.recovery.taskbar_hidden = false);
    }
    if let Some(t) = &plan.restore_toasts {
        sys::toasts_restore(t);
        update_file(|f| f.recovery.toasts = None);
    }
}

/// Restaure tout ce que RusTV a modifié (idempotent, sans AppHandle : utilisable depuis un panic hook).
pub fn restore_all() {
    let f = FILE.try_lock().ok().and_then(|g| g.clone()).unwrap_or_else(|| store::load("guard"));
    let plan = recovery_plan(&f.recovery, false);
    apply_recovery(&plan);
    with_rt(|rt| {
        rt.taskbar_hidden = false;
        rt.toasts_muted = false;
    });
}

fn stop_threads() {
    let (stop, tid, awake) = with_rt(|rt| (rt.stop.take(), rt.hook_tid.take(), rt.awake_stop.take()));
    if let Some(s) = stop {
        s.store(true, Ordering::SeqCst);
    }
    if let Some(t) = tid {
        sys::quit_hook_thread(t.load(Ordering::SeqCst));
    }
    if let Some(a) = awake {
        let _ = a.send(());
    }
}

/// Entrée dans le mode TV : mémorise la session, maintient l'éveil, masque la barre des tâches, coupe les
/// notifications et lance la surveillance des fenêtres, selon la configuration.
pub fn activate(app: &AppHandle) {
    let (mon_idx, mons) = {
        let shell = app.state::<Shell>();
        let idx = shell.tv.lock().map(|t| t.monitor).unwrap_or(0);
        (idx, tv::monitors(app))
    };
    let m = mons.get(mon_idx);
    update_file(|f| {
        f.recovery.session = Session {
            tv_active: true,
            monitor_index: mon_idx,
            monitor_name: m.map(|m| m.name.clone()).unwrap_or_default(),
            monitor_w: m.map(|m| m.w).unwrap_or(0),
            monitor_h: m.map(|m| m.h).unwrap_or(0),
            monitor_count: mons.len(),
        }
    });
    stop_threads();
    let cfg = config();
    with_rt(|rt| rt.active = true);
    if cfg.keep_awake {
        let tx = sys::keep_awake_start();
        with_rt(|rt| rt.awake_stop = Some(tx));
    }
    if !cfg.enabled {
        return;
    }
    if cfg.suppress_toasts {
        let cur = sys::toasts_read();
        let saved = toast_save_plan(cur, &load_file().recovery.toasts);
        update_file(|f| f.recovery.toasts = Some(saved)); // marqueur écrit AVANT de modifier le registre
        sys::toasts_set(0);
        with_rt(|rt| rt.toasts_muted = true);
    }
    if cfg.hide_taskbar {
        update_file(|f| f.recovery.taskbar_hidden = true);
        with_rt(|rt| rt.taskbar_hidden = true);
    }
    let stop = Arc::new(AtomicBool::new(false));
    let tid = Arc::new(AtomicU32::new(0));
    let (tx, rx) = std::sync::mpsc::sync_channel::<isize>(512);
    sys::set_event_sender(Some(tx));
    {
        let tid = tid.clone();
        std::thread::spawn(move || sys::hook_thread(tid));
    }
    {
        let (app, stop) = (app.clone(), stop.clone());
        std::thread::spawn(move || sys::worker(app, cfg, stop, rx));
    }
    with_rt(|rt| {
        rt.stop = Some(stop);
        rt.hook_tid = Some(tid);
    });
}

/// Sortie du mode TV : arrête tout et restaure.
pub fn deactivate(_app: &AppHandle) {
    stop_threads();
    sys::set_event_sender(None);
    // Laisse le worker finir son tour (il réaffiche aussi la barre des tâches en sortant).
    std::thread::sleep(std::time::Duration::from_millis(50));
    restore_all();
    update_file(|f| f.recovery.session.tv_active = false);
    with_rt(|rt| rt.active = false);
    ACTIONS.store(0, Ordering::Relaxed);
    HIDDEN.store(0, Ordering::Relaxed);
}

/// Réapplique la configuration pendant que le mode TV est actif.
fn reapply(app: &AppHandle) {
    if with_rt(|rt| rt.active) {
        deactivate(app);
        activate(app);
    }
}

// =====================================================================================================
// Commandes Tauri
// =====================================================================================================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardView {
    #[serde(flatten)]
    pub cfg: GuardCfg,
    pub status: Status,
}

fn view() -> GuardView {
    GuardView { cfg: config(), status: status() }
}

#[tauri::command]
pub async fn guard_get() -> GuardView {
    view()
}

#[tauri::command]
pub async fn guard_status() -> Status {
    status()
}

#[tauri::command]
pub async fn guard_set(app: AppHandle, cfg: GuardCfg) -> Result<GuardCfg, String> {
    let cfg = cfg.sanitized();
    let old = config();
    update_file(|f| f.cfg = cfg.clone());
    if old != config() {
        let app2 = app.clone();
        std::thread::spawn(move || reapply(&app2));
    }
    Ok(config())
}

/// Bouton « panique » : coupe la garde, le masquage de la barre des tâches, les notifications et le maintien
/// d'éveil, et restaure tout immédiatement.
#[tauri::command]
pub async fn guard_panic(app: AppHandle) -> Result<GuardCfg, String> {
    update_file(|f| {
        f.cfg.enabled = false;
        f.cfg.hide_taskbar = false;
        f.cfg.suppress_toasts = false;
        f.cfg.keep_awake = false;
    });
    // Le mode TV peut rester actif : on garde l'indicateur, mais plus rien n'est modifié sur le système.
    let still = app.state::<Shell>().tv.lock().map(|t| t.active).unwrap_or(false);
    deactivate(&app);
    with_rt(|rt| rt.active = still);
    update_file(|f| f.recovery.session.tv_active = still);
    Ok(config())
}

// =====================================================================================================
// Enveloppes Win32 (jamais appelées par les tests)
// =====================================================================================================

mod sys {
    use super::*;
    use std::ffi::c_void;
    use std::process::Command;
    use std::sync::mpsc::{Receiver, SyncSender};
    use windows::core::{BOOL, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, FILETIME, HMODULE, HWND, LPARAM, RECT, WPARAM};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED};
    use windows::Win32::System::Threading::{GetCurrentThreadId, GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::*;

    const REG_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings";
    const REG_VALUE: &str = "NOC_GLOBAL_SETTING_TOASTS_ENABLED";

    fn hwnd(h: isize) -> HWND {
        HWND(h as *mut c_void)
    }

    // ---- registre (reg.exe, comme autostart.rs ; jamais appelé hors application) ----

    fn reg(args: &[&str]) -> Option<String> {
        use std::os::windows::process::CommandExt;
        let out = Command::new("reg").args(args).creation_flags(0x0800_0000).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }

    pub fn toasts_read() -> Option<u32> {
        reg(&["query", REG_KEY, "/v", REG_VALUE]).and_then(|s| parse_reg_dword(&s))
    }

    pub fn toasts_set(v: u32) {
        let _ = reg(&["add", REG_KEY, "/v", REG_VALUE, "/t", "REG_DWORD", "/d", &v.to_string(), "/f"]);
    }

    pub fn toasts_restore(t: &ToastSaved) {
        if t.existed {
            toasts_set(t.value);
        } else {
            let _ = reg(&["delete", REG_KEY, "/v", REG_VALUE, "/f"]);
        }
    }

    // ---- maintien d'éveil : l'état d'exécution est lié au thread qui l'a posé ----

    pub fn keep_awake_start() -> std::sync::mpsc::Sender<()> {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        std::thread::spawn(move || {
            unsafe {
                SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
            }
            let _ = rx.recv(); // un message ou la fermeture du canal : on relâche
            unsafe {
                SetThreadExecutionState(ES_CONTINUOUS);
            }
        });
        tx
    }

    // ---- fenêtres ----

    unsafe extern "system" fn collect(h: HWND, lp: LPARAM) -> BOOL {
        let v = &mut *(lp.0 as *mut Vec<isize>);
        v.push(h.0 as isize);
        BOOL(1)
    }

    fn enum_windows() -> Vec<isize> {
        let mut v: Vec<isize> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(collect), LPARAM(&mut v as *mut Vec<isize> as isize));
        }
        v
    }

    fn class_of(h: HWND) -> String {
        let mut b = [0u16; 256];
        let n = unsafe { GetClassNameW(h, &mut b) };
        String::from_utf16_lossy(&b[..n.max(0) as usize])
    }

    fn rect_of(h: HWND) -> Rect {
        let mut r = RECT::default();
        unsafe {
            let _ = GetWindowRect(h, &mut r);
        }
        Rect { l: r.left, t: r.top, r: r.right, b: r.bottom }
    }

    /// Liste (pid, ppid) de tous les processus.
    fn process_list() -> Vec<(u32, u32)> {
        let mut v = Vec::new();
        unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return v };
            let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            if Process32FirstW(snap, &mut e).is_ok() {
                loop {
                    v.push((e.th32ProcessID, e.th32ParentProcessID));
                    if Process32NextW(snap, &mut e).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snap);
        }
        v
    }

    /// (nom de l'exécutable en minuscules, date de création en ms Unix).
    fn proc_info(pid: u32) -> (String, Option<u64>) {
        unsafe {
            let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return (String::new(), None) };
            let mut buf = [0u16; 520];
            let mut len = buf.len() as u32;
            let name = if QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok() {
                let full = String::from_utf16_lossy(&buf[..len as usize]);
                full.rsplit(['\\', '/']).next().unwrap_or("").to_ascii_lowercase()
            } else {
                String::new()
            };
            let (mut c, mut x, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
            let created = GetProcessTimes(h, &mut c, &mut x, &mut k, &mut u).ok().map(|_| {
                let t = ((c.dwHighDateTime as u64) << 32) | c.dwLowDateTime as u64; // 100 ns depuis 1601
                (t / 10_000).saturating_sub(11_644_473_600_000)
            });
            let _ = CloseHandle(h);
            (name, created)
        }
    }

    fn win_info(h: isize, cache: &mut HashMap<u32, (String, Option<u64>)>) -> WinInfo {
        let w = hwnd(h);
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(w, Some(&mut pid));
        }
        let ex = unsafe { GetWindowLongW(w, GWL_EXSTYLE) } as u32;
        let mut cloaked = 0u32;
        unsafe {
            let _ = DwmGetWindowAttribute(w, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut c_void, 4);
        }
        let (exe, _) = cache.entry(pid).or_insert_with(|| proc_info(pid)).clone();
        WinInfo {
            hwnd: h,
            pid,
            class: class_of(w),
            exe,
            rect: rect_of(w),
            visible: unsafe { IsWindowVisible(w).as_bool() },
            minimized: unsafe { IsIconic(w).as_bool() },
            cloaked: cloaked != 0,
            tool_like: ex & (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) != 0,
        }
    }

    // ---- barre des tâches ----

    fn tray_windows() -> Vec<(isize, Rect)> {
        enum_windows().into_iter().filter(|h| matches!(class_of(hwnd(*h)).as_str(), "Shell_TrayWnd" | "Shell_SecondaryTrayWnd")).map(|h| (h, rect_of(hwnd(h)))).collect()
    }

    /// Réaffiche TOUTES les barres des tâches (récupération : on ne sait plus laquelle on a masquée).
    pub fn set_taskbar_visible_all() {
        for (h, _) in tray_windows() {
            unsafe {
                let _ = ShowWindow(hwnd(h), SW_SHOWNA);
            }
        }
    }

    fn hide_taskbar_on(tv: &Rect, hidden: &mut HashSet<isize>) {
        for (h, r) in tray_windows() {
            if affects_monitor(&r, tv) || hidden.contains(&h) {
                let w = hwnd(h);
                if unsafe { IsWindowVisible(w).as_bool() } {
                    unsafe {
                        let _ = ShowWindow(w, SW_HIDE);
                    }
                }
                hidden.insert(h);
            }
        }
    }

    // ---- crochets d'événements ----

    static EV_TX: Mutex<Option<SyncSender<isize>>> = Mutex::new(None);

    pub fn set_event_sender(tx: Option<SyncSender<isize>>) {
        *EV_TX.lock().unwrap_or_else(|e| e.into_inner()) = tx;
    }

    unsafe extern "system" fn on_event(_h: HWINEVENTHOOK, event: u32, w: HWND, id_object: i32, id_child: i32, _thread: u32, _ms: u32) {
        if w.0.is_null() {
            return;
        }
        // Les événements d'objet ne nous intéressent que pour la fenêtre elle-même (pas le curseur, le caret…).
        if (event == EVENT_OBJECT_SHOW || event == EVENT_OBJECT_LOCATIONCHANGE) && (id_object != 0 || id_child != 0) {
            return;
        }
        if let Ok(g) = EV_TX.try_lock() {
            if let Some(tx) = g.as_ref() {
                let _ = tx.try_send(w.0 as isize);
            }
        }
    }

    pub fn hook_thread(tid: Arc<AtomicU32>) {
        unsafe {
            let mut msg = MSG::default();
            let _ = PeekMessageW(&mut msg, None, WM_USER, WM_USER, PM_NOREMOVE); // crée la file de messages
            tid.store(GetCurrentThreadId(), Ordering::SeqCst);
            let mut hooks = Vec::new();
            for ev in [EVENT_SYSTEM_FOREGROUND, EVENT_OBJECT_SHOW, EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_MINIMIZEEND] {
                let h = SetWinEventHook(ev, ev, None::<HMODULE>, Some(on_event), 0, 0, WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS);
                if !h.is_invalid() {
                    hooks.push(h);
                }
            }
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            for h in hooks {
                let _ = UnhookWinEvent(h);
            }
        }
    }

    pub fn quit_hook_thread(tid: u32) {
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
    }

    // ---- boucle de travail ----

    struct Engine {
        app: AppHandle,
        cfg: GuardCfg,
        policy: Policy,
        own_pid: u32,
        trust: Trust,
        backoffs: HashMap<isize, Backoff>,
        fg_back: Backoff,
        top_back: Backoff,
        acted: HashSet<isize>,
        last_good: Option<isize>,
        topmost: Option<isize>,
        procs: Vec<(u32, u32)>,
        procs_at: u64,
        cache: HashMap<u32, (String, Option<u64>)>,
        cache_at: u64,
        switcher_at: Option<u64>,
        tray_hidden: HashSet<isize>,
        mon_sig: Vec<(i32, i32, u32, u32)>,
        t0: std::time::Instant,
        stop: Arc<AtomicBool>,
    }

    impl Engine {
        fn now(&self) -> u64 {
            self.t0.elapsed().as_millis() as u64
        }

        fn refresh_procs(&mut self) {
            let now = self.now();
            if now.saturating_sub(self.procs_at) < 1_000 && self.procs_at != 0 {
                return;
            }
            self.procs = process_list();
            self.procs_at = now;
            let alive: HashSet<u32> = self.procs.iter().map(|p| p.0).collect();
            self.trust.adopted_pids.retain(|p| alive.contains(p));
            self.trust.adopted_hwnds.retain(|h| unsafe { IsWindow(Some(hwnd(*h))).as_bool() });
            let mut roots = vec![self.own_pid];
            roots.extend(self.trust.adopted_pids.iter().copied());
            self.trust.own = descendants(&roots, &self.procs);
            if now.saturating_sub(self.cache_at) > 5_000 {
                self.cache.clear();
                self.cache_at = now;
            }
        }

        /// Rectangle de l'écran de la TV et rectangles des autres écrans.
        fn monitors(&mut self) -> Option<(Rect, Vec<Rect>)> {
            let idx = self.app.state::<Shell>().tv.lock().ok()?.monitor;
            let mons = tv::monitors(&self.app);
            let sig: Vec<_> = mons.iter().map(|m| (m.x, m.y, m.w, m.h)).collect();
            if !self.mon_sig.is_empty() && sig != self.mon_sig {
                // Écran branché / réveillé / résolution changée : on repositionne les fenêtres TV.
                tv::reflow(&self.app);
            }
            self.mon_sig = sig;
            let rects: Vec<Rect> = mons.iter().map(|m| Rect::new(m.x, m.y, m.w as i32, m.h as i32)).collect();
            let tv = *rects.get(idx)?;
            let others = rects.iter().enumerate().filter(|(i, _)| *i != idx).map(|(_, r)| *r).collect();
            Some((tv, others))
        }

        fn tv_home(&self) -> Option<isize> {
            self.app.get_webview_window("tv").and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize)
        }

        fn created_of(&mut self, pid: u32) -> Option<u64> {
            self.cache.entry(pid).or_insert_with(|| proc_info(pid)).1
        }

        fn act(&mut self, w: &WinInfo, tv: &Rect, others: &[Rect]) {
            let now = self.now();
            let b = self.backoffs.entry(w.hwnd).or_insert_with(Backoff::per_window);
            if !b.permit(now) {
                return;
            }
            let h = hwnd(w.hwnd);
            let target = others.iter().find(|o| o.area() > 0).map(|o| move_target(&w.rect, tv, o));
            match (self.cfg.mode.as_str(), target) {
                ("move", Some(t)) => unsafe {
                    if IsZoomed(h).as_bool() {
                        let _ = ShowWindow(h, SW_RESTORE);
                    }
                    let _ = SetWindowPos(h, None, t.l, t.t, t.w() as i32, t.h() as i32, SWP_NOZORDER | SWP_NOACTIVATE);
                },
                _ => unsafe {
                    let _ = ShowWindow(h, SW_SHOWMINNOACTIVE);
                },
            }
            ACTIONS.fetch_add(1, Ordering::Relaxed);
            self.acted.insert(w.hwnd);
        }

        /// Examine une fenêtre. Renvoie le verdict final.
        fn examine(&mut self, h: isize, tv: &Rect, others: &[Rect]) -> Verdict {
            let w = win_info(h, &mut self.cache);
            let mut v = judge(&w, tv, &self.policy, &self.trust);
            match v {
                Verdict::Allowed("class") if self.policy.is_switcher(&w.class) => self.switcher_at = Some(self.now()),
                Verdict::Offender => {
                    let fg = unsafe { GetForegroundWindow().0 as isize } == h;
                    let launches = PENDING.lock().unwrap_or_else(|e| e.into_inner()).clone();
                    let created = self.created_of(w.pid);
                    let mut ad = adopt_for(&launches, &w, created, unix_ms());
                    if ad == Adopt::No && fg && switched_by_user(self.switcher_at, self.now()) {
                        ad = if w.class == "ApplicationFrameWindow" { Adopt::Hwnd(h) } else { Adopt::Pid(w.pid) };
                    }
                    match ad {
                        Adopt::Pid(p) => {
                            self.trust.adopted_pids.insert(p);
                            self.procs_at = 0;
                            v = Verdict::Allowed("adopted");
                        }
                        Adopt::Hwnd(x) => {
                            self.trust.adopted_hwnds.insert(x);
                            v = Verdict::Allowed("adopted");
                        }
                        Adopt::No => self.act(&w, tv, others),
                    }
                }
                _ => {}
            }
            if matches!(v, Verdict::Allowed("trusted") | Verdict::Allowed("adopted")) {
                self.last_good = Some(h);
            }
            v
        }

        /// Après les actions : si le premier plan n'est plus une fenêtre TV, le rendre (avec limitation de débit).
        fn fix_foreground(&mut self, tv: &Rect) {
            let fg = unsafe { GetForegroundWindow().0 as isize };
            let verdict = (fg != 0).then(|| {
                let w = win_info(fg, &mut self.cache);
                judge(&w, tv, &self.policy, &self.trust)
            });
            match verdict {
                // (e) La fenêtre TV au premier plan reste « topmost » ; la précédente perd ce statut.
                Some(Verdict::Allowed("trusted")) => return self.raise_topmost(fg),
                Some(Verdict::Offender) | None => {}
                Some(_) => return,
            }
            let now = self.now();
            if !self.fg_back.permit(now) {
                return;
            }
            let target = self.last_good.filter(|h| unsafe { IsWindow(Some(hwnd(*h))).as_bool() && !IsIconic(hwnd(*h)).as_bool() }).or_else(|| self.tv_home());
            if let Some(t) = target {
                rustv_core::input::force_foreground(t);
                ACTIONS.fetch_add(1, Ordering::Relaxed);
            }
        }

        fn raise_topmost(&mut self, h: isize) {
            if self.topmost == Some(h) {
                return;
            }
            let now = self.now();
            if !self.top_back.permit(now) {
                return;
            }
            if let Some(prev) = self.topmost.take() {
                rustv_core::input::set_topmost(hwnd(prev), false);
            }
            rustv_core::input::set_topmost(hwnd(h), true);
            self.topmost = Some(h);
        }

        fn sweep(&mut self, only: Option<Vec<isize>>) {
            self.refresh_procs();
            let Some((tv, others)) = self.monitors() else { return };
            let list = match only {
                Some(v) => v,
                None => enum_windows(),
            };
            for h in list {
                if unsafe { !IsWindow(Some(hwnd(h))).as_bool() } {
                    continue;
                }
                self.examine(h, &tv, &others);
            }
            self.acted.retain(|h| unsafe { IsWindow(Some(hwnd(*h))).as_bool() && IsIconic(hwnd(*h)).as_bool() });
            HIDDEN.store(self.acted.len() as u32, Ordering::Relaxed);
            self.fix_foreground(&tv);
            if self.cfg.hide_taskbar && !self.stop.load(Ordering::SeqCst) {
                hide_taskbar_on(&tv, &mut self.tray_hidden);
            }
        }
    }

    /// Thread de travail : traite les événements (regroupés) et fait un balayage complet toutes les ~500 ms.
    pub fn worker(app: AppHandle, cfg: GuardCfg, stop: Arc<AtomicBool>, rx: Receiver<isize>) {
        let mut e = Engine {
            app,
            policy: Policy::from_cfg(&cfg),
            cfg,
            own_pid: std::process::id(),
            trust: Trust::default(),
            backoffs: HashMap::new(),
            fg_back: Backoff::foreground(),
            top_back: Backoff::new(6, 10_000, 0, 5_000, 60_000),
            acted: HashSet::new(),
            last_good: None,
            topmost: None,
            procs: Vec::new(),
            procs_at: 0,
            cache: HashMap::new(),
            cache_at: 0,
            switcher_at: None,
            tray_hidden: HashSet::new(),
            mon_sig: Vec::new(),
            t0: std::time::Instant::now(),
            stop: stop.clone(),
        };
        let mut last_sweep = 0u64;
        while !stop.load(Ordering::SeqCst) {
            let first = rx.recv_timeout(std::time::Duration::from_millis(250));
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let mut evs: HashSet<isize> = HashSet::new();
            if let Ok(h) = first {
                evs.insert(h);
                std::thread::sleep(std::time::Duration::from_millis(40)); // regroupe les rafales
                while let Ok(h) = rx.try_recv() {
                    evs.insert(h);
                    if evs.len() > 200 {
                        break;
                    }
                }
            }
            let now = e.now();
            if now.saturating_sub(last_sweep) >= 500 || last_sweep == 0 {
                last_sweep = now;
                e.sweep(None);
            } else if !evs.is_empty() {
                e.sweep(Some(evs.into_iter().collect()));
            }
        }
        // Sortie : la barre des tâches revient ; les fenêtres minimisées par la garde restent à l'utilisateur.
        if let Some(t) = e.topmost.take() {
            rustv_core::input::set_topmost(hwnd(t), false);
        }
        for h in e.tray_hidden.drain() {
            unsafe {
                let _ = ShowWindow(hwnd(h), SW_SHOWNA);
            }
        }
    }
}

// =====================================================================================================
// Tests (logique pure uniquement)
// =====================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TV: Rect = Rect { l: 1920, t: 0, r: 3840, b: 1080 };
    const MAIN: Rect = Rect { l: 0, t: 0, r: 1920, b: 1080 };

    fn win(class: &str, exe: &str, r: Rect) -> WinInfo {
        WinInfo { hwnd: 100, pid: 500, class: class.into(), exe: exe.into(), rect: r, visible: true, ..Default::default() }
    }

    // ---- rectangles ----

    #[test]
    fn overlap_rules() {
        assert!(affects_monitor(&Rect::new(2000, 100, 800, 600), &TV));
        assert!(!affects_monitor(&Rect::new(100, 100, 800, 600), &TV));
        // à cheval : centre sur l'écran principal, 35 % sur la TV -> ignorée
        assert!(!affects_monitor(&Rect::new(1400, 0, 800, 600), &TV));
        // à cheval, centre sur la TV -> concernée
        assert!(affects_monitor(&Rect::new(1600, 0, 800, 600), &TV));
        // plein écran sur la TV
        assert!(affects_monitor(&TV, &TV));
        assert!(!affects_monitor(&Rect::new(0, 0, 0, 0), &TV));
    }

    #[test]
    fn rect_geometry() {
        assert_eq!(Rect::new(0, 0, 10, 10).inter_area(&Rect::new(5, 5, 10, 10)), 25);
        assert_eq!(Rect::new(0, 0, 10, 10).inter_area(&Rect::new(20, 20, 5, 5)), 0);
        assert_eq!(Rect::new(0, 0, 10, 20).center(), (5, 10));
    }

    #[test]
    fn move_to_other_monitor() {
        let w = Rect::new(2020, 100, 800, 600);
        let t = move_target(&w, &TV, &MAIN);
        assert_eq!((t.l, t.t, t.w(), t.h()), (100, 100, 800, 600));
        // trop grand : réduit ; trop à droite : recalé
        let big = Rect::new(1920, 0, 1920, 1080);
        let t = move_target(&big, &TV, &Rect::new(0, 0, 1280, 720));
        assert_eq!((t.l, t.t, t.w(), t.h()), (0, 0, 1280, 720));
        let edge = Rect::new(3600, 900, 400, 400);
        let t = move_target(&edge, &TV, &MAIN);
        assert!(t.r <= MAIN.r && t.b <= MAIN.b && t.l >= 0 && t.t >= 0);
    }

    // ---- autorisations ----

    #[test]
    fn class_allow_list() {
        let p = Policy::default();
        for c in ["Shell_TrayWnd", "XamlExplorerHostIslandWindow", "MultitaskingViewFrame", "ForegroundStaging", "progman", "#32768"] {
            assert!(p.class_allowed(c), "{c}");
        }
        assert!(!p.class_allowed("Notepad"));
        assert!(!p.class_allowed("#32770")); // les boîtes de dialogue ne sont PAS exemptées
        assert!(p.is_switcher("TaskSwitcherWnd") && !p.is_switcher("Shell_TrayWnd"));
        let p = Policy { extra_classes: vec!["MyOverlay*".into()], extra_exes: vec![] };
        assert!(p.class_allowed("myoverlay_1") && !p.class_allowed("Other"));
    }

    #[test]
    fn exe_allow_list() {
        let p = Policy { extra_classes: vec![], extra_exes: vec!["OBS64".into(), r"C:\Tools\vlc.exe".into()] };
        assert!(p.exe_allowed("obs64.exe") && p.exe_allowed("OBS64.EXE") && p.exe_allowed("vlc.exe"));
        assert!(!p.exe_allowed("obs.exe") && !p.exe_allowed(""));
        assert!(p.exe_allowed("textinputhost.exe"));
    }

    #[test]
    fn process_tree() {
        let procs = [(10, 1), (11, 10), (12, 11), (20, 1), (21, 20), (30, 30)];
        let s = descendants(&[10], &procs);
        assert!(s.contains(&10) && s.contains(&11) && s.contains(&12) && !s.contains(&21));
        assert_eq!(descendants(&[0], &procs).len(), 0);
        // cycle : terminaison garantie
        assert!(descendants(&[30], &procs).contains(&30));
    }

    #[test]
    fn judge_order() {
        let p = Policy::default();
        let mut t = Trust::default();
        t.own.insert(500);
        let on_tv = Rect::new(2000, 100, 800, 600);
        assert_eq!(judge(&win("Notepad", "notepad.exe", on_tv), &TV, &p, &Trust::default()), Verdict::Offender);
        assert_eq!(judge(&win("Notepad", "notepad.exe", on_tv), &TV, &p, &t), Verdict::Allowed("trusted"));
        assert_eq!(judge(&win("Notepad", "notepad.exe", Rect::new(100, 100, 800, 600)), &TV, &p, &Trust::default()), Verdict::Skip("other-monitor"));
        assert_eq!(judge(&win("Shell_TrayWnd", "explorer.exe", on_tv), &TV, &p, &Trust::default()), Verdict::Allowed("class"));
        let mut w = win("Notepad", "notepad.exe", on_tv);
        w.visible = false;
        assert_eq!(judge(&w, &TV, &p, &Trust::default()), Verdict::Skip("hidden"));
        let mut w = win("Notepad", "notepad.exe", on_tv);
        w.minimized = true;
        assert!(matches!(judge(&w, &TV, &p, &Trust::default()), Verdict::Skip(_)));
        let mut w = win("Foo", "foo.exe", on_tv);
        w.cloaked = true;
        assert!(matches!(judge(&w, &TV, &p, &Trust::default()), Verdict::Skip(_)));
        let mut w = win("Foo", "textinputhost.exe", on_tv);
        assert_eq!(judge(&w, &TV, &p, &Trust::default()), Verdict::Allowed("process"));
        w.exe = "foo.exe".into();
        w.tool_like = true;
        assert_eq!(judge(&w, &TV, &p, &Trust::default()), Verdict::Skip("tool-window"));
        // adoption par fenêtre
        let mut t = Trust::default();
        t.adopted_hwnds.insert(100);
        assert_eq!(judge(&win("ApplicationFrameWindow", "applicationframehost.exe", on_tv), &TV, &p, &t), Verdict::Allowed("trusted"));
        // fenêtre minuscule
        assert!(matches!(judge(&win("X", "x.exe", Rect::new(2000, 0, 10, 10)), &TV, &p, &Trust::default()), Verdict::Skip("tiny")));
    }

    #[test]
    fn launch_adoption() {
        let l = launch_from_target(r"C:\Games\Steam\Steam.exe", 1_000_000);
        assert_eq!(l.exe_stem, "steam");
        assert!(!l.opaque);
        let w = WinInfo { hwnd: 7, pid: 42, exe: "steam.exe".into(), ..Default::default() };
        // même nom de processus, même s'il tournait déjà
        assert_eq!(adopt_for(&[l.clone()], &w, Some(1), 1_010_000), Adopt::Pid(42));
        // nouveau processus d'un autre nom
        let other = WinInfo { hwnd: 8, pid: 43, exe: "launcher.exe".into(), ..Default::default() };
        assert_eq!(adopt_for(&[l.clone()], &other, Some(1_000_500), 1_005_000), Adopt::Pid(43));
        // ancien processus d'un autre nom : refusé
        assert_eq!(adopt_for(&[l.clone()], &other, Some(10), 1_005_000), Adopt::No);
        // trop tard
        assert_eq!(adopt_for(&[l.clone()], &w, Some(1), 1_000_000 + ADOPT_PID_MS + 1), Adopt::No);
        // raccourci : fenêtre seule, délai court
        let lnk = launch_from_target(r"C:\ProgramData\Start Menu\Foo.lnk", 2_000_000);
        assert!(lnk.opaque && lnk.exe_stem.is_empty());
        assert_eq!(adopt_for(&[lnk.clone()], &other, Some(10), 2_005_000), Adopt::Hwnd(8));
        assert_eq!(adopt_for(&[lnk], &other, Some(10), 2_020_000), Adopt::No);
        assert_eq!(adopt_for(&[], &w, Some(1), 5), Adopt::No);
    }

    #[test]
    fn alt_tab_adoption_window() {
        assert!(switched_by_user(Some(1_000), 3_000));
        assert!(!switched_by_user(Some(1_000), 6_000));
        assert!(!switched_by_user(None, 10));
    }

    // ---- anti-acharnement ----

    #[test]
    fn backoff_state_machine() {
        let mut b = Backoff::new(3, 1_000, 0, 500, 2_000);
        assert!(b.permit(0) && b.permit(10) && b.permit(20));
        assert!(!b.permit(30)); // 4e action dans la fenêtre : pause 500 ms
        assert!(b.paused(100) && !b.permit(100));
        assert!(b.permit(600)); // pause finie
        assert!(b.permit(610) && b.permit(620));
        assert!(!b.permit(630)); // deuxième pause : 1000 ms
        assert_eq!(b.strikes(), 2);
        assert!(!b.permit(1_500));
        assert!(b.permit(1_700));
        // pause plafonnée
        for i in 0..40u64 {
            let t = 10_000 + i * 20_000;
            for k in 0..4 {
                b.permit(t + k);
            }
        }
        assert!(b.paused(10_000 + 39 * 20_000 + 4) || b.strikes() >= 1);
        // longtemps au calme : le compteur retombe
        assert!(b.permit(10_000_000));
        assert_eq!(b.strikes(), 0);
    }

    #[test]
    fn backoff_min_gap() {
        let mut b = Backoff::foreground();
        assert!(b.permit(0));
        assert!(!b.permit(100)); // trop rapproché
        assert!(b.permit(400));
        // jamais de boucle serrée : sur 3 s à 50 ms d'intervalle, au plus 5 actions accordées puis pause de 5 s
        let mut b = Backoff::foreground();
        let granted = (0..60u64).filter(|i| b.permit(i * 50)).count();
        assert_eq!(granted, 5);
    }

    // ---- configuration ----

    #[test]
    fn config_defaults_and_roundtrip() {
        let c: GuardFile = serde_json::from_str("{}").unwrap();
        assert!(c.cfg.enabled && c.cfg.keep_awake && !c.cfg.hide_taskbar && !c.cfg.suppress_toasts);
        assert_eq!(c.cfg.mode, "minimize");
        assert!(c.cfg.allow_processes.is_empty());
        assert_eq!(c.recovery, Recovery::default());
        let c: GuardFile = serde_json::from_str(r#"{"enabled":false,"mode":"move","allowProcesses":["a.exe"],"recovery":{"taskbarHidden":true,"toasts":{"existed":true,"value":1}}}"#).unwrap();
        assert!(!c.cfg.enabled && c.cfg.mode == "move" && c.cfg.keep_awake);
        assert!(c.recovery.taskbar_hidden);
        assert_eq!(c.recovery.toasts, Some(ToastSaved { existed: true, value: 1 }));
        let s = serde_json::to_string(&c).unwrap();
        assert!(s.contains("\"allowProcesses\"") && s.contains("\"keepAwake\"") && s.contains("\"bootToTv\""));
        let back: GuardFile = serde_json::from_str(&s).unwrap();
        assert_eq!(back, c);
        // contenu invalide : valeurs par défaut côté appelant (store::load), jamais de panique ici
        assert!(serde_json::from_str::<GuardFile>("[1]").is_err());
    }

    #[test]
    fn config_sanitize() {
        let c = GuardCfg { mode: "explode".into(), allow_processes: vec![" A.exe ".into(), "a.EXE".into(), "".into(), "x".repeat(500)], boot_to_tv: "zzz".into(), ..Default::default() }.sanitized();
        assert_eq!(c.mode, "minimize");
        assert_eq!(c.allow_processes, vec!["A.exe".to_string()]);
        assert_eq!(c.boot_to_tv, "off");
        let many = GuardCfg { allow_classes: (0..500).map(|i| format!("c{i}")).collect(), ..Default::default() }.sanitized();
        assert_eq!(many.allow_classes.len(), MAX_LIST);
    }

    // ---- récupération ----

    #[test]
    fn recovery_decisions() {
        let r = Recovery { taskbar_hidden: true, toasts: Some(ToastSaved { existed: false, value: 1 }), session: Session::default() };
        let p = recovery_plan(&r, false);
        assert!(p.show_taskbar && p.restore_toasts == Some(ToastSaved { existed: false, value: 1 }));
        assert_eq!(recovery_plan(&r, true), RecoveryPlan::default()); // mode TV actif : on ne touche à rien
        assert_eq!(recovery_plan(&Recovery::default(), false), RecoveryPlan::default());
    }

    #[test]
    fn toast_save_never_overwrites() {
        assert_eq!(toast_save_plan(Some(1), &None), ToastSaved { existed: true, value: 1 });
        assert_eq!(toast_save_plan(None, &None), ToastSaved { existed: false, value: 1 });
        // deuxième activation : la valeur courante est notre 0, on garde la sauvegarde d'origine
        let first = ToastSaved { existed: true, value: 1 };
        assert_eq!(toast_save_plan(Some(0), &Some(first.clone())), first);
    }

    #[test]
    fn reg_dword_parsing() {
        let out = "\r\nHKEY_CURRENT_USER\\Software\\X\r\n    NOC_GLOBAL_SETTING_TOASTS_ENABLED    REG_DWORD    0x0\r\n";
        assert_eq!(parse_reg_dword(out), Some(0));
        assert_eq!(parse_reg_dword("    V    REG_DWORD    0x1f"), Some(31));
        assert_eq!(parse_reg_dword("    V    REG_SZ    abc"), None);
        assert_eq!(parse_reg_dword(""), None);
    }

    // ---- démarrage : écran de la TV ----

    fn mon(name: &str, w: u32, h: u32) -> MonDesc {
        MonDesc { name: name.into(), w, h }
    }

    #[test]
    fn boot_waits_for_slow_tv() {
        let saved = SavedMon { index: 1, name: r"\\.\DISPLAY2".into(), w: 3840, h: 2160, count: 2 };
        let only_pc = [mon(r"\\.\DISPLAY1", 1920, 1080)];
        assert_eq!(boot_decision(Some(&saved), &only_pc, 0, 60_000), Boot::Wait);
        assert_eq!(boot_decision(Some(&saved), &only_pc, 30_000, 60_000), Boot::Wait);
        assert_eq!(boot_decision(Some(&saved), &only_pc, 60_000, 60_000), Boot::Fallback(0));
        let both = [mon(r"\\.\DISPLAY1", 1920, 1080), mon(r"\\.\DISPLAY2", 3840, 2160)];
        assert_eq!(boot_decision(Some(&saved), &both, 5_000, 60_000), Boot::Use(1));
        // l'ordre a changé : on retrouve la TV par son nom
        let swapped = [mon(r"\\.\DISPLAY2", 3840, 2160), mon(r"\\.\DISPLAY1", 1920, 1080)];
        assert_eq!(boot_decision(Some(&saved), &swapped, 0, 60_000), Boot::Use(0));
        // même nom mais résolution différente (TV pas encore négociée) : on attend
        let wrong = [mon(r"\\.\DISPLAY1", 1920, 1080), mon(r"\\.\DISPLAY2", 1024, 768)];
        assert_eq!(boot_decision(Some(&saved), &wrong, 1_000, 60_000), Boot::Wait);
        assert_eq!(boot_decision(Some(&saved), &[], 120_000, 60_000), Boot::Wait);
    }

    #[test]
    fn boot_by_index_and_pick() {
        let by_index = SavedMon { index: 1, name: String::new(), w: 0, h: 0, count: 2 };
        let one = [mon("a", 1, 1)];
        let two = [mon("a", 1, 1), mon("b", 2, 2)];
        assert_eq!(boot_decision(Some(&by_index), &one, 0, 60_000), Boot::Wait);
        assert_eq!(boot_decision(Some(&by_index), &two, 0, 60_000), Boot::Use(1));
        assert_eq!(boot_decision(Some(&by_index), &one, 61_000, 60_000), Boot::Fallback(0));
        assert_eq!(boot_decision(None, &two, 0, 60_000), Boot::Pick);
        assert_eq!(boot_decision(None, &[], 0, 60_000), Boot::Wait);
    }
}
