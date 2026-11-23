//! Configs de découverte MQTT de Home Assistant : un appareil « RusTV <nom du PC> » et ses entités.
use super::command::BUTTONS;
use super::topics::Topics;
use serde_json::{json, Map, Value};

pub struct Ctx {
    /// Identifiant de noeud (id du PC nettoyé) : `[A-Za-z0-9_-]+`.
    pub node: String,
    pub topics: Topics,
    pub prefix: String,
    pub pc_name: String,
    pub version: String,
    pub options: Vec<String>,
}

pub struct Entity {
    pub component: &'static str,
    pub key: String,
    /// Topic de config retenu.
    pub topic: String,
    pub payload: Value,
}

fn device(c: &Ctx) -> Value {
    json!({
        "identifiers": [format!("rustv_{}", c.node)],
        "name": format!("RusTV {}", c.pc_name),
        "manufacturer": "RusTV",
        "model": "RusTV TV mode",
        "sw_version": c.version,
    })
}

fn base_entity(c: &Ctx, key: &str, name: &str) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("name".into(), name.into());
    m.insert("unique_id".into(), format!("rustv_{}_{}", c.node, key).into());
    m.insert("has_entity_name".into(), true.into());
    m.insert("availability_topic".into(), c.topics.availability().into());
    m.insert("payload_available".into(), "online".into());
    m.insert("payload_not_available".into(), "offline".into());
    m.insert("device".into(), device(c));
    m
}

fn finish(c: &Ctx, component: &'static str, key: &str, m: Map<String, Value>) -> Entity {
    Entity { component, key: key.to_string(), topic: format!("{}/{}/{}/{}/config", c.prefix, component, c.node, key), payload: Value::Object(m) }
}

fn sensor(c: &Ctx, key: &str, name: &str, state_key: &str, attrs_key: Option<&str>, extra: &[(&str, Value)]) -> Entity {
    let mut m = base_entity(c, key, name);
    m.insert("state_topic".into(), c.topics.state(state_key).into());
    if let Some(a) = attrs_key {
        m.insert("json_attributes_topic".into(), c.topics.state(a).into());
    }
    for (k, v) in extra {
        m.insert((*k).into(), v.clone());
    }
    finish(c, "sensor", key, m)
}

fn binary(c: &Ctx, key: &str, name: &str, state_key: &str, attrs_key: Option<&str>, extra: &[(&str, Value)]) -> Entity {
    let mut m = base_entity(c, key, name);
    m.insert("state_topic".into(), c.topics.state(state_key).into());
    m.insert("payload_on".into(), "ON".into());
    m.insert("payload_off".into(), "OFF".into());
    if let Some(a) = attrs_key {
        m.insert("json_attributes_topic".into(), c.topics.state(a).into());
    }
    for (k, v) in extra {
        m.insert((*k).into(), v.clone());
    }
    finish(c, "binary_sensor", key, m)
}

fn switch(c: &Ctx, key: &str, name: &str, state_key: &str, cmd: &str, icon: &str) -> Entity {
    let mut m = base_entity(c, key, name);
    m.insert("state_topic".into(), c.topics.state(state_key).into());
    m.insert("command_topic".into(), c.topics.cmd(cmd).into());
    m.insert("payload_on".into(), "ON".into());
    m.insert("payload_off".into(), "OFF".into());
    m.insert("qos".into(), 1.into());
    m.insert("icon".into(), icon.into());
    finish(c, "switch", key, m)
}

/// Toutes les entités de l'appareil, dans un ordre stable.
pub fn entities(c: &Ctx) -> Vec<Entity> {
    let mut v = vec![
        sensor(
            c,
            "state",
            "State",
            "state",
            Some("state_attrs"),
            &[("device_class", "enum".into()), ("options", json!(["off", "home", "playing", "paused", "app"])), ("icon", "mdi:television".into())],
        ),
        sensor(c, "now_playing", "Now playing", "now_playing", Some("now_playing_attrs"), &[("icon", "mdi:play-circle".into())]),
        sensor(c, "current_app", "Current app", "current_app", None, &[("icon", "mdi:application".into())]),
        sensor(c, "volume_level", "Volume", "volume", None, &[("unit_of_measurement", "%".into()), ("icon", "mdi:volume-high".into())]),
        binary(c, "tv_mode_active", "TV mode", "tv_mode", None, &[("icon", "mdi:television-classic".into())]),
        binary(c, "phone_connected", "Phone connected", "phone_connected", Some("phone_attrs"), &[("device_class", "connectivity".into())]),
        sensor(
            c,
            "uptime",
            "Uptime",
            "uptime",
            None,
            &[("device_class", "duration".into()), ("unit_of_measurement", "min".into()), ("entity_category", "diagnostic".into())],
        ),
        sensor(c, "version", "Version", "version", None, &[("entity_category", "diagnostic".into()), ("icon", "mdi:information-outline".into())]),
        switch(c, "tv_mode_switch", "TV mode", "tv_mode", "tv", "mdi:television"),
        switch(c, "mute", "Mute", "muted", "mute", "mdi:volume-off"),
    ];

    let mut n = base_entity(c, "volume_set", "Volume");
    n.insert("state_topic".into(), c.topics.state("volume").into());
    n.insert("command_topic".into(), c.topics.cmd("volume").into());
    n.insert("min".into(), 0.into());
    n.insert("max".into(), 100.into());
    n.insert("step".into(), 1.into());
    n.insert("mode".into(), "slider".into());
    n.insert("unit_of_measurement".into(), "%".into());
    n.insert("qos".into(), 1.into());
    n.insert("icon".into(), "mdi:volume-medium".into());
    v.push(finish(c, "number", "volume_set", n));

    let mut s = base_entity(c, "app_launcher", "App launcher");
    s.insert("state_topic".into(), c.topics.state("app_launcher").into());
    s.insert("command_topic".into(), c.topics.cmd("app").into());
    s.insert("options".into(), json!(c.options));
    s.insert("qos".into(), 1.into());
    s.insert("icon".into(), "mdi:apps".into());
    v.push(finish(c, "select", "app_launcher", s));

    // Pour les scripts : un identifiant de tuile brut (sans état).
    let mut t = base_entity(c, "open_tile", "Open tile (id)");
    t.insert("command_topic".into(), c.topics.cmd("open_tile").into());
    t.insert("min".into(), 1.into());
    t.insert("max".into(), 80.into());
    t.insert("qos".into(), 1.into());
    t.insert("icon".into(), "mdi:rocket-launch".into());
    t.insert("entity_category".into(), "config".into());
    v.push(finish(c, "text", "open_tile", t));

    for (id, name) in BUTTONS {
        let mut b = base_entity(c, &format!("btn_{id}"), name);
        b.insert("command_topic".into(), c.topics.cmd("button").into());
        b.insert("payload_press".into(), (*id).into());
        b.insert("qos".into(), 1.into());
        v.push(finish(c, "button", &format!("btn_{id}"), b));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn ctx() -> Ctx {
        Ctx {
            node: "ab12".into(),
            topics: Topics::new("rustv/ab12"),
            prefix: "homeassistant".into(),
            pc_name: "SALON".into(),
            version: "1.2.3".into(),
            options: vec!["Plex".into(), "YouTube".into()],
        }
    }

    #[test]
    fn payloads_are_valid_json_with_device_block() {
        let es = entities(&ctx());
        assert!(es.len() >= 26);
        for e in &es {
            let s = serde_json::to_string(&e.payload).unwrap();
            let back: Value = serde_json::from_str(&s).unwrap();
            assert_eq!(back["device"]["manufacturer"], "RusTV");
            assert_eq!(back["device"]["name"], "RusTV SALON");
            assert_eq!(back["device"]["sw_version"], "1.2.3");
            assert_eq!(back["device"]["identifiers"][0], "rustv_ab12");
            assert!(back["device"].get("configuration_url").is_none());
            assert_eq!(back["availability_topic"], "rustv/ab12/availability");
            assert!(back["unique_id"].as_str().unwrap().starts_with("rustv_ab12_"));
            assert!(e.topic.starts_with("homeassistant/") && e.topic.ends_with("/config"));
        }
    }

    #[test]
    fn unique_ids_and_topics() {
        let es = entities(&ctx());
        let ids: HashSet<_> = es.iter().map(|e| e.payload["unique_id"].as_str().unwrap().to_string()).collect();
        let tops: HashSet<_> = es.iter().map(|e| e.topic.clone()).collect();
        assert_eq!(ids.len(), es.len());
        assert_eq!(tops.len(), es.len());
    }

    #[test]
    fn entity_set_matches_brief() {
        let es = entities(&ctx());
        let has = |c: &str, k: &str| es.iter().any(|e| e.component == c && e.key == k);
        assert!(has("sensor", "state") && has("sensor", "now_playing") && has("sensor", "current_app") && has("sensor", "volume_level"));
        assert!(has("binary_sensor", "tv_mode_active") && has("binary_sensor", "phone_connected"));
        assert!(has("switch", "tv_mode_switch") && has("switch", "mute") && has("number", "volume_set") && has("select", "app_launcher"));
        assert_eq!(es.iter().filter(|e| e.component == "button").count(), BUTTONS.len());
        let sel = es.iter().find(|e| e.component == "select").unwrap();
        assert_eq!(sel.payload["options"], json!(["Plex", "YouTube"]));
        assert_eq!(sel.payload["command_topic"], "rustv/ab12/cmd/app");
        let up = es.iter().find(|e| e.key == "uptime").unwrap();
        assert_eq!(up.payload["entity_category"], "diagnostic");
    }

    #[test]
    fn every_command_topic_is_parseable() {
        for e in entities(&ctx()) {
            if let Some(t) = e.payload["command_topic"].as_str() {
                let sub = Topics::new("rustv/ab12").cmd_sub(t).unwrap().to_string();
                assert!(["tv", "mute", "volume", "app", "open_tile", "button"].contains(&sub.as_str()), "{sub}");
            }
        }
    }
}
