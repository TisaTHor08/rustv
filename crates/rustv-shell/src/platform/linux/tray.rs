//! Linux : y a-t-il un hôte d'icônes de notification (StatusNotifierWatcher) ? Sans lui (GNOME sans extension),
//! l'icône de RusTV est invisible : le shell ouvre alors la fenêtre d'appairage au lieu de rester muet.
use zbus::blocking::{fdo::DBusProxy, Connection};

pub fn host_available() -> bool {
    let Ok(conn) = Connection::session() else { return false };
    let Ok(dbus) = DBusProxy::new(&conn) else { return false };
    let Ok(name) = zbus::names::BusName::try_from("org.kde.StatusNotifierWatcher") else { return false };
    dbus.name_has_owner(name).unwrap_or(false)
}
