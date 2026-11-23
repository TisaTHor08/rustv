//! Touches de la télécommande dans mpv (`input.conf`), menu du lecteur (script Lua livré avec RusTV)
//! et formatage de la liste des pistes.
//!
//! Choix : le menu est un script Lua à nous (`--script=`), dessiné avec `osd-overlay` et piloté par des
//! raccourcis « forcés » (haut/bas/gauche/droite/OK/retour) qui n'existent que tant que le menu est ouvert.
//! On n'utilise pas le menu intégré de mpv (`select.lua`, mpv récent seulement, saisie au clavier) : le nôtre
//! fonctionne avec toutes les versions de mpv qui ont `mp.create_osd_overlay` (>= 0.31) et reste maîtrisé
//! par la télécommande. Le script ne peut pas être exécuté ici : il est court, défensif (pcall partout).
use serde_json::Value;

/// Contenu d'`input.conf` selon le type de lecture. Noms de touches mpv : MENU = touche « menu contextuel »
/// (VK_APPS), NEXT/PREV = touches média suivant/précédent.
pub fn input_conf(kind: &str) -> String {
    let lines: &[&str] = if kind == "live" {
        &[
            "UP script-message rustv zap -1",
            "DOWN script-message rustv zap 1",
            "LEFT add volume -5",
            "RIGHT add volume 5",
            "ENTER cycle pause",
            "MENU script-message rustv-menu",
            "NEXT script-message rustv zap 1",
            "PREV script-message rustv zap -1",
            "F3 ignore",
            "ESC quit",
            "BS quit",
        ]
    } else {
        &[
            "LEFT script-message rustv-seek -1",
            "RIGHT script-message rustv-seek 1",
            "UP script-message rustv-menu",
            "DOWN show-progress",
            "ENTER script-message rustv-ok",
            "MENU script-message rustv-menu",
            "NEXT script-message rustv next",
            "PREV script-message rustv prev",
            "F3 ignore",
            "ESC quit",
            "BS quit",
        ]
    };
    let mut s = lines.join("\n");
    s.push('\n');
    s
}

/// Libellé d'une piste (objet de `track-list`) : titre, langue, codec, canaux, drapeaux.
pub fn track_label(t: &Value) -> String {
    let mut p: Vec<String> = Vec::new();
    if let Some(s) = t["title"].as_str().filter(|s| !s.is_empty()) {
        p.push(s.to_string());
    }
    if let Some(s) = t["lang"].as_str().filter(|s| !s.is_empty()) {
        p.push(s.to_uppercase());
    }
    if let Some(s) = t["codec"].as_str().filter(|s| !s.is_empty()) {
        p.push(s.to_string());
    }
    if t["type"] == "audio" {
        match t["demux-channel-count"].as_i64() {
            Some(1) => p.push("mono".into()),
            Some(2) => p.push("stereo".into()),
            Some(6) => p.push("5.1".into()),
            Some(8) => p.push("7.1".into()),
            Some(n) if n > 0 => p.push(format!("{n}ch")),
            _ => {}
        }
    }
    if t["forced"].as_bool() == Some(true) {
        p.push("forced".into());
    }
    if t["external"].as_bool() == Some(true) {
        p.push("ext".into());
    }
    if p.is_empty() {
        format!("#{}", t["id"].as_i64().unwrap_or(0))
    } else {
        p.join(" - ")
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct TrackInfo {
    pub id: i64,
    pub label: String,
    pub selected: bool,
}

/// Pistes audio et sous-titres d'une valeur `track-list`.
pub fn tracks_from(list: &Value) -> (Vec<TrackInfo>, Vec<TrackInfo>) {
    let (mut audio, mut subs) = (Vec::new(), Vec::new());
    for t in list.as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter().take(64) {
        let info = TrackInfo { id: t["id"].as_i64().unwrap_or(0), label: track_label(t), selected: t["selected"].as_bool() == Some(true) };
        match t["type"].as_str() {
            Some("audio") => audio.push(info),
            Some("sub") => subs.push(info),
            _ => {}
        }
    }
    (audio, subs)
}

/// Texte du script Lua chargé par mpv (`--script=`). Options : `rustv-kind` (live|video|audio), `rustv-lang` (fr|en).
pub fn lua_script() -> String {
    LUA.to_string()
}

const LUA: &str = r##"-- RusTV : menu du lecteur pilote par la telecommande (genere par rustv-shell, ne pas modifier).
local mp = require("mp")

local KIND = mp.get_opt("rustv-kind") or "video"
local LANG = mp.get_opt("rustv-lang") or "fr"

local STR = {
  fr = { title = "Lecteur", audio = "Piste audio", subs = "Sous-titres", off = "Désactivés", none = "Aucune piste",
    sdelay = "Décalage", ssize = "Taille", speed = "Vitesse", aspect = "Format d'image et zoom", zoom = "Zoom",
    fill = "Remplir l'écran", reset = "Réinitialiser", chapters = "Chapitres", sleep = "Minuterie d'arrêt",
    info = "Informations", auto = "Auto", sleep_off = "Désactivée", sleep_set = "Arrêt dans %d min",
    hint = "Haut/Bas : choisir - OK : valider - Gauche/Droite : régler - Retour : fermer" },
  en = { title = "Player", audio = "Audio track", subs = "Subtitles", off = "Off", none = "No track",
    sdelay = "Delay", ssize = "Size", speed = "Speed", aspect = "Aspect ratio and zoom", zoom = "Zoom",
    fill = "Fill the screen", reset = "Reset", chapters = "Chapters", sleep = "Sleep timer",
    info = "Information", auto = "Auto", sleep_off = "Off", sleep_set = "Stop in %d min",
    hint = "Up/Down: choose - OK: select - Left/Right: adjust - Back: close" },
}
local L = STR[LANG] or STR.fr

local function safe(f, ...)
  local ok, err = pcall(f, ...)
  if not ok then mp.msg.warn("rustv: " .. tostring(err)) end
  return ok
end

local function esc(s)
  s = tostring(s or "")
  s = s:gsub("[{}]", ""):gsub("\\", "/"):gsub("[\r\n]", " ")
  return s
end

local function osd(s, secs) mp.osd_message(s, secs or 3) end
local function prop(name, default)
  local v = mp.get_property(name)
  if v == nil or v == "" then return default or "n/a" end
  return v
end
local function num(name, default) return mp.get_property_number(name) or default or 0 end

local function hms(t)
  t = math.floor(tonumber(t) or 0)
  local h, m = math.floor(t / 3600), math.floor((t % 3600) / 60)
  if h > 0 then return string.format("%d:%02d:%02d", h, m, t % 60) end
  return string.format("%d:%02d", m, t % 60)
end

local function track_label(t)
  local p = {}
  if t.title and t.title ~= "" then p[#p + 1] = t.title end
  if t.lang and t.lang ~= "" then p[#p + 1] = string.upper(t.lang) end
  if t.codec and t.codec ~= "" then p[#p + 1] = t.codec end
  if t.type == "audio" then
    local c = t["demux-channel-count"]
    if c == 1 then p[#p + 1] = "mono" elseif c == 2 then p[#p + 1] = "stereo"
    elseif c == 6 then p[#p + 1] = "5.1" elseif c == 8 then p[#p + 1] = "7.1"
    elseif c and c > 0 then p[#p + 1] = c .. "ch" end
  end
  if t.forced then p[#p + 1] = "forced" end
  if t.external then p[#p + 1] = "ext" end
  if #p == 0 then return "#" .. tostring(t.id) end
  return table.concat(p, " - ")
end

-- ---------- moteur de menu ----------

local overlay = mp.create_osd_overlay("ass-events")
overlay.res_x = 1920
overlay.res_y = 1080
local menu = nil
local KEYS = { "UP", "DOWN", "LEFT", "RIGHT", "ENTER", "ESC", "BS", "MENU" }

local function render()
  if not menu then overlay:remove() return end
  local m = menu.cur
  local out = { "{\\an7\\pos(80,70)\\fs44\\b1\\bord3\\shad0\\1c&HFFFFFF&\\3c&H000000&}" .. esc(m.title) }
  local n = #m.items
  local first = math.max(1, math.min(m.sel - 5, n - 11))
  local last = math.min(n, first + 11)
  for i = first, last do
    local it = m.items[i]
    local label = it.text and it.text() or it.label or ""
    if i == m.sel then
      out[#out + 1] = "{\\fs36\\b1\\1c&H3C5AFF&}> " .. esc(label)
    else
      out[#out + 1] = "{\\fs36\\b0\\1c&HEEEEEE&}   " .. esc(label)
    end
  end
  out[#out + 1] = "{\\fs26\\b0\\1c&HBBBBBB&}" .. esc(L.hint)
  overlay.data = table.concat(out, "\\N")
  overlay:update()
end

local function unbind()
  for _, k in ipairs(KEYS) do mp.remove_key_binding("rustv-menu-" .. k) end
end

local function close()
  menu = nil
  unbind()
  render()
end

local function back()
  if menu and #menu.stack > 0 then
    menu.cur = table.remove(menu.stack)
    render()
  else
    close()
  end
end

local function move(d)
  local m = menu.cur
  local n = #m.items
  if n == 0 then return end
  m.sel = (m.sel - 1 + d) % n + 1
  render()
end

local function on_key(name)
  return function()
    if not menu then return end
    safe(function()
      local m = menu.cur
      local it = m.items[m.sel]
      if name == "UP" then move(-1)
      elseif name == "DOWN" then move(1)
      elseif name == "LEFT" then if it and it.left then it.left() render() end
      elseif name == "RIGHT" then if it and it.right then it.right() render() end
      elseif name == "ENTER" then if it and it.enter then it.enter() end
      elseif name == "MENU" then close()
      else back() end
    end)
  end
end

local function bind()
  for _, k in ipairs(KEYS) do
    mp.add_forced_key_binding(k, "rustv-menu-" .. k, on_key(k), { repeatable = true })
  end
end

local function open_menu(def)
  def.sel = def.sel or 1
  if menu then
    table.insert(menu.stack, menu.cur)
    menu.cur = def
  else
    menu = { stack = {}, cur = def }
    bind()
  end
  render()
end

-- ---------- sous-menus ----------

local function track_menu(ttype, propname, title)
  local items = {}
  local list = mp.get_property_native("track-list") or {}
  local any = false
  for _, t in ipairs(list) do
    if t.type == ttype and t.selected then any = true end
  end
  if ttype == "sub" then
    items[#items + 1] = { label = (any and "   " or "* ") .. L.off, enter = function()
      mp.set_property(propname, "no") back() osd(L.subs .. ": " .. L.off) end }
  end
  for _, t in ipairs(list) do
    if t.type == ttype then
      local id = t.id
      local label = track_label(t)
      items[#items + 1] = { label = (t.selected and "* " or "   ") .. label, enter = function()
        mp.set_property(propname, tostring(id)) back() osd(title .. ": " .. label) end }
    end
  end
  if ttype == "sub" then
    items[#items + 1] = { text = function() return L.sdelay .. ": " .. string.format("%+.1f s", num("sub-delay")) end,
      left = function() mp.commandv("add", "sub-delay", "-0.1") end,
      right = function() mp.commandv("add", "sub-delay", "0.1") end }
    items[#items + 1] = { text = function() return L.ssize .. ": " .. string.format("%d%%", math.floor(num("sub-scale", 1) * 100 + 0.5)) end,
      left = function() mp.commandv("add", "sub-scale", "-0.1") end,
      right = function() mp.commandv("add", "sub-scale", "0.1") end }
  end
  if #items == 0 then items[1] = { label = L.none, enter = function() back() end } end
  return { title = title, items = items }
end

local function speed_menu()
  local items = {}
  for _, v in ipairs({ 0.5, 0.75, 1, 1.25, 1.5, 2 }) do
    items[#items + 1] = { label = string.format("x%g", v), enter = function()
      mp.set_property_number("speed", v) back() osd(L.speed .. ": " .. string.format("x%g", v)) end }
  end
  return { title = L.speed, items = items }
end

local function aspect_menu()
  local items = {}
  for _, a in ipairs({ { L.auto, "-1" }, { "16:9", "16:9" }, { "4:3", "4:3" }, { "2.35:1", "2.35:1" } }) do
    items[#items + 1] = { label = a[1], enter = function()
      mp.set_property("video-aspect-override", a[2]) back() osd(L.aspect .. ": " .. a[1]) end }
  end
  items[#items + 1] = { text = function() return L.zoom .. ": " .. string.format("%+.1f", num("video-zoom")) end,
    left = function() mp.commandv("add", "video-zoom", "-0.1") end,
    right = function() mp.commandv("add", "video-zoom", "0.1") end }
  items[#items + 1] = { label = L.fill, enter = function()
    mp.set_property_number("panscan", num("panscan") > 0 and 0 or 1) back() end }
  items[#items + 1] = { label = L.reset, enter = function()
    mp.set_property("video-aspect-override", "-1") mp.set_property_number("video-zoom", 0)
    mp.set_property_number("panscan", 0) back() end }
  return { title = L.aspect, items = items }
end

local function chapters_menu()
  local items = {}
  local list = mp.get_property_native("chapter-list") or {}
  local cur = num("chapter", -1)
  for i, c in ipairs(list) do
    local idx = i - 1
    items[#items + 1] = { label = (idx == cur and "* " or "   ") .. hms(c.time) .. "  " .. (c.title or ("#" .. i)), enter = function()
      mp.set_property_number("chapter", idx) close() end }
  end
  if #items == 0 then items[1] = { label = L.none, enter = function() back() end } end
  return { title = L.chapters, items = items, sel = math.max(1, cur + 1) }
end

local sleep_timer = nil
local function sleep_menu()
  local items = { { label = L.sleep_off, enter = function()
    if sleep_timer then sleep_timer:kill() sleep_timer = nil end back() osd(L.sleep .. ": " .. L.sleep_off) end } }
  for _, m in ipairs({ 15, 30, 60, 90 }) do
    items[#items + 1] = { label = string.format(L.sleep_set, m), enter = function()
      if sleep_timer then sleep_timer:kill() end
      sleep_timer = mp.add_timeout(m * 60, function() mp.commandv("quit") end)
      back() osd(string.format(L.sleep_set, m)) end }
  end
  return { title = L.sleep, items = items }
end

local function show_info()
  close()
  local w, h = num("width"), num("height")
  local lines = {
    prop("media-title"),
    string.format("%dx%d  %s  %.2f fps", w, h, prop("video-codec"), num("container-fps")),
    "audio: " .. prop("audio-codec-name") .. "  hwdec: " .. prop("hwdec-current", "no"),
    hms(num("time-pos")) .. " / " .. hms(num("duration")),
  }
  osd(table.concat(lines, "\n"), 6)
end

local function main_def()
  local items = {}
  local function sub(label, make) items[#items + 1] = { label = label, enter = function() open_menu(make()) end } end
  if KIND ~= "audio" then
    sub(L.audio, function() return track_menu("audio", "aid", L.audio) end)
    sub(L.subs, function() return track_menu("sub", "sid", L.subs) end)
  end
  if KIND ~= "live" then sub(L.speed, speed_menu) end
  if KIND ~= "audio" then sub(L.aspect, aspect_menu) end
  if KIND ~= "live" then
    local ch = mp.get_property_native("chapter-list") or {}
    if #ch > 0 then sub(L.chapters, chapters_menu) end
    sub(L.sleep, sleep_menu)
  end
  items[#items + 1] = { label = L.info, enter = function() safe(show_info) end }
  return { title = L.title, items = items }
end

-- ---------- messages envoyes par input.conf ----------

mp.register_script_message("rustv-menu", function()
  if menu then close() else safe(function() open_menu(main_def()) end) end
end)

mp.register_script_message("rustv-ok", function()
  if mp.get_property_bool("idle-active") then
    mp.commandv("script-message", "rustv", "skip-wait")
  else
    mp.commandv("cycle", "pause")
  end
end)

-- Saut gauche/droite : 10 s, puis 30 s et 60 s quand la touche reste enfoncee.
local last_seek, seek_count = 0, 0
mp.register_script_message("rustv-seek", function(dir)
  safe(function()
    local now = mp.get_time()
    if now - last_seek < 0.45 then seek_count = seek_count + 1 else seek_count = 1 end
    last_seek = now
    local step = seek_count > 12 and 60 or (seek_count > 4 and 30 or 10)
    mp.commandv("osd-msg-bar", "seek", tostring((tonumber(dir) or 1) * step), "relative")
  end)
end)

mp.register_event("end-file", function() if menu then close() end end)
"##;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn conf_video_keys() {
        let c = input_conf("video");
        for want in ["LEFT script-message rustv-seek -1", "MENU script-message rustv-menu", "UP script-message rustv-menu", "ENTER script-message rustv-ok", "NEXT script-message rustv next", "PREV script-message rustv prev", "ESC quit", "BS quit", "F3 ignore"] {
            assert!(c.lines().any(|l| l == want), "missing {want}");
        }
        assert!(c.ends_with('\n'));
    }

    #[test]
    fn conf_live_keys() {
        let c = input_conf("live");
        assert!(c.contains("UP script-message rustv zap -1"));
        assert!(c.contains("DOWN script-message rustv zap 1"));
        assert!(c.contains("MENU script-message rustv-menu"));
        assert!(c.contains("RIGHT add volume 5"));
        assert!(!c.contains("rustv-seek"));
    }

    #[test]
    fn lua_has_bindings() {
        let l = lua_script();
        for want in ["register_script_message(\"rustv-menu\"", "rustv-ok", "rustv-seek", "add_forced_key_binding", "track-list", "chapter-list", "sub-delay", "sub-scale", "video-aspect-override", "create_osd_overlay", "pcall", "\"BS\"", "\"ESC\""] {
            assert!(l.contains(want), "missing {want}");
        }
    }

    #[test]
    fn lua_brackets_balanced() {
        let l = lua_script().replace("end-file", "eof-event");
        for (o, c) in [('(', ')'), ('{', '}'), ('[', ']')] {
            assert_eq!(l.matches(o).count(), l.matches(c).count(), "unbalanced {o}{c}");
        }
        // blocs : chaque `function`/`if`/`for` ouvre un `end` (comptage grossier mais utile contre une coquille)
        let words = |w: &str| l.split(|ch: char| !(ch.is_alphanumeric() || ch == '_')).filter(|x| *x == w).count();
        let openers = words("function") + words("if") + words("for") + words("while");
        // pas d'opérateur `//` (absent de Lua 5.1/LuaJIT)
        assert!(!l.contains(" // "));
        assert_eq!(openers, words("end"));
    }

    #[test]
    fn track_labels() {
        assert_eq!(track_label(&json!({"id": 2, "type": "audio", "lang": "fre", "codec": "ac3", "demux-channel-count": 6})), "FRE - ac3 - 5.1");
        assert_eq!(track_label(&json!({"id": 3, "type": "sub", "title": "Forced", "lang": "en", "forced": true})), "Forced - EN - forced");
        assert_eq!(track_label(&json!({"id": 7, "type": "sub"})), "#7");
    }

    #[test]
    fn track_split() {
        let list = json!([
            {"id": 1, "type": "video", "selected": true},
            {"id": 1, "type": "audio", "lang": "eng", "selected": true},
            {"id": 2, "type": "audio", "lang": "fre"},
            {"id": 1, "type": "sub", "lang": "fre"}
        ]);
        let (a, s) = tracks_from(&list);
        assert_eq!(a.len(), 2);
        assert!(a[0].selected && !a[1].selected);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].label, "FRE");
        assert_eq!(tracks_from(&json!(null)), (vec![], vec![]));
    }
}
