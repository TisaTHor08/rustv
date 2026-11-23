//! Construction des topics MQTT à partir du topic de base (`rustv/<id>` par défaut).

#[derive(Clone, Debug, PartialEq)]
pub struct Topics {
    base: String,
}

impl Topics {
    pub fn new(base: &str) -> Self {
        Self { base: base.trim_matches('/').to_string() }
    }

    pub fn availability(&self) -> String {
        format!("{}/availability", self.base)
    }

    pub fn state(&self, key: &str) -> String {
        format!("{}/state/{}", self.base, key)
    }

    pub fn cmd(&self, sub: &str) -> String {
        format!("{}/cmd/{}", self.base, sub)
    }

    /// Filtre d'abonnement aux commandes.
    pub fn cmd_filter(&self) -> String {
        format!("{}/cmd/#", self.base)
    }

    /// `rustv/x/cmd/volume` → `Some("volume")` ; tout autre topic → `None`.
    pub fn cmd_sub<'a>(&self, topic: &'a str) -> Option<&'a str> {
        let rest = topic.strip_prefix(self.base.as_str())?.strip_prefix("/cmd/")?;
        (!rest.is_empty()).then_some(rest)
    }

    pub fn test(&self) -> String {
        format!("{}/test", self.base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn building() {
        let t = Topics::new("/rustv/abc/");
        assert_eq!(t.availability(), "rustv/abc/availability");
        assert_eq!(t.state("volume"), "rustv/abc/state/volume");
        assert_eq!(t.cmd("tv"), "rustv/abc/cmd/tv");
        assert_eq!(t.cmd_filter(), "rustv/abc/cmd/#");
    }

    #[test]
    fn cmd_sub_only_matches_own_commands() {
        let t = Topics::new("rustv/abc");
        assert_eq!(t.cmd_sub("rustv/abc/cmd/volume"), Some("volume"));
        assert_eq!(t.cmd_sub("rustv/abc/cmd/a/b"), Some("a/b"));
        assert_eq!(t.cmd_sub("rustv/abc/cmd/"), None);
        assert_eq!(t.cmd_sub("rustv/abcd/cmd/volume"), None);
        assert_eq!(t.cmd_sub("rustv/abc/state/volume"), None);
        assert_eq!(t.cmd_sub("other/cmd/volume"), None);
    }
}
