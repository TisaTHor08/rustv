//! Liste des tuiles ouvrables (catalogue + tuiles perso + tuiles de mods + intégrées) pour l'entité `select`.
use serde_json::Value;
use std::collections::HashSet;

const MAX_OPTIONS: usize = 300;

/// Tuiles intégrées de l'accueil (id, nom anglais) : mêmes identifiants que `BUILTIN` côté interface.
const BUILTIN: &[(&str, &str)] = &[
    ("live", "Live TV"),
    ("media", "Library"),
    ("files", "Files"),
    ("set-wifi", "Wi-Fi"),
    ("set-bluetooth", "Bluetooth"),
    ("set-sound", "Sound"),
    ("set-display", "Displays"),
    ("set-power", "Power"),
];

#[derive(Clone, Debug, PartialEq)]
pub struct TileOpt {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TileMap {
    pub opts: Vec<TileOpt>,
}

impl TileMap {
    pub fn names(&self) -> Vec<String> {
        self.opts.iter().map(|o| o.name.clone()).collect()
    }

    pub fn id_for(&self, name: &str) -> Option<&str> {
        self.opts.iter().find(|o| o.name == name).map(|o| o.id.as_str())
    }

    pub fn name_for(&self, id: &str) -> Option<&str> {
        self.opts.iter().find(|o| o.id == id).map(|o| o.name.as_str())
    }
}

fn safe_id(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '-' }).collect()
}

/// Tuiles apportées par les mods actifs : (id de tuile côté interface, nom). Même règle que `modTiles()` de l'interface.
pub fn mod_tiles(mods: &[Value]) -> Vec<(String, String)> {
    let mut out = vec![];
    for m in mods {
        if m["enabled"] != true {
            continue;
        }
        let Some(mid) = m["id"].as_str() else { continue };
        let c = &m["manifest"]["contributes"];
        for key in ["tiles", "pages"] {
            for t in c[key].as_array().into_iter().flatten() {
                let Some(tid) = t["id"].as_str() else { continue };
                let name = t["name"].as_str().unwrap_or(tid);
                out.push((format!("mod-{}-{}", mid, safe_id(tid)), name.to_string()));
            }
        }
    }
    out
}

/// Construit la liste : noms personnalisés appliqués, tuiles masquées exclues, identifiants uniques,
/// noms rendus uniques (« Nom (2) »), triée par nom.
pub fn build(catalog: &[Value], prefs: &Value, mods: &[Value]) -> TileMap {
    let tiles = &prefs["tiles"];
    let hidden: HashSet<&str> = tiles["hidden"].as_array().into_iter().flatten().filter_map(|v| v.as_str()).collect();
    let renames = &tiles["names"];

    let mut raw: Vec<(String, String)> = BUILTIN.iter().map(|(i, n)| (i.to_string(), n.to_string())).collect();
    for t in catalog {
        if let (Some(i), Some(n)) = (t["id"].as_str(), t["name"].as_str()) {
            raw.push((i.into(), n.into()));
        }
    }
    for t in tiles["custom"].as_array().into_iter().flatten() {
        if let (Some(i), Some(n)) = (t["id"].as_str(), t["name"].as_str()) {
            raw.push((i.into(), n.into()));
        }
    }
    raw.extend(mod_tiles(mods));

    let mut seen_ids = HashSet::new();
    let mut items: Vec<(String, String)> = Vec::new();
    for (id, name) in raw {
        if !super::command::valid_tile_id(&id) || hidden.contains(id.as_str()) || !seen_ids.insert(id.clone()) {
            continue;
        }
        let name = renames[id.as_str()].as_str().unwrap_or(&name);
        let name: String = name.trim().chars().filter(|c| !c.is_control()).take(60).collect();
        if name.is_empty() {
            continue;
        }
        items.push((id, name));
    }
    items.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()).then(a.0.cmp(&b.0)));
    items.truncate(MAX_OPTIONS);

    let mut used: HashSet<String> = HashSet::new();
    let mut opts = Vec::new();
    for (id, name) in items {
        let mut cand = name.clone();
        let mut n = 2;
        while !used.insert(cand.to_lowercase()) {
            cand = format!("{name} ({n})");
            n += 1;
        }
        opts.push(TileOpt { id, name: cand });
    }
    TileMap { opts }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_dedups_and_maps() {
        let catalog = vec![json!({"id":"youtube","name":"YouTube"}), json!({"id":"plex","name":"Plex"})];
        let prefs = json!({"tiles":{
            "hidden":["set-power"],
            "names":{"plex":"Plex Salon"},
            "custom":[{"id":"c-1","name":"YouTube"},{"id":"bad id","name":"X"}]
        }});
        let mods = vec![
            json!({"id":"m1","enabled":true,"manifest":{"contributes":{"tiles":[{"id":"t.1","name":"Radio"}],"pages":[{"id":"p","name":"YouTube"}]}}}),
            json!({"id":"m2","enabled":false,"manifest":{"contributes":{"tiles":[{"id":"x","name":"Off"}]}}}),
        ];
        let m = build(&catalog, &prefs, &mods);
        let names = m.names();
        assert!(names.contains(&"Live TV".to_string()));
        assert!(!names.contains(&"Power".to_string()), "masquee");
        assert!(!names.contains(&"Off".to_string()), "mod desactive");
        assert!(names.contains(&"Plex Salon".to_string()) && !names.contains(&"Plex".to_string()));
        // trois tuiles nommees YouTube -> noms uniques
        let yt: Vec<_> = names.iter().filter(|n| n.starts_with("YouTube")).collect();
        assert_eq!(yt.len(), 3);
        let uniq: HashSet<_> = names.iter().map(|n| n.to_lowercase()).collect();
        assert_eq!(uniq.len(), names.len());
        assert_eq!(m.id_for("Radio"), Some("mod-m1-t-1"));
        assert_eq!(m.id_for("Plex Salon"), Some("plex"));
        assert_eq!(m.name_for("plex"), Some("Plex Salon"));
        assert!(m.id_for("bad id").is_none() && m.id_for("X").is_none());
        assert_eq!(m.id_for("Nope"), None);
    }

    #[test]
    fn duplicate_ids_keep_first_and_null_prefs_ok() {
        let catalog = vec![json!({"id":"a","name":"A"}), json!({"id":"a","name":"B"})];
        let m = build(&catalog, &Value::Null, &[]);
        assert_eq!(m.name_for("a"), Some("A"));
        assert!(m.opts.len() >= 9);
        let sorted: Vec<String> = m.names().iter().map(|n| n.to_lowercase()).collect();
        let mut s2 = sorted.clone();
        s2.sort();
        assert_eq!(sorted, s2);
    }
}
