//! Connexion MQTT : options, messages d'erreur lisibles, et échange ponctuel (test, suppression de la découverte).
use super::config::HaConfig;
use rumqttc::{Client, ConnectReturnCode, ConnectionError, Event, Incoming, LastWill, MqttOptions, NetworkOptions, QoS, Transport};
use std::time::{Duration, Instant};

/// Options communes (le mot de passe est passé en clair uniquement le temps de la connexion).
pub fn options(cfg: &HaConfig, client_id: &str, password: Option<String>, will: Option<(String, &str)>) -> MqttOptions {
    let mut o = MqttOptions::new(client_id, cfg.host.clone(), cfg.port);
    o.set_keep_alive(Duration::from_secs(30));
    o.set_clean_session(true);
    o.set_max_packet_size(512 * 1024, 512 * 1024);
    if !cfg.username.is_empty() {
        o.set_credentials(cfg.username.clone(), password.unwrap_or_default());
    }
    if let Some((topic, msg)) = will {
        o.set_last_will(LastWill::new(topic, msg, QoS::AtLeastOnce, true));
    }
    if cfg.tls {
        o.set_transport(Transport::tls_with_default_config());
    }
    o
}

pub fn network() -> NetworkOptions {
    let mut n = NetworkOptions::new();
    n.set_connection_timeout(8);
    n
}

/// Message en clair pour l'utilisateur (jamais de détail sensible).
pub fn friendly_error(e: &ConnectionError) -> String {
    match e {
        ConnectionError::ConnectionRefused(code) => match code {
            ConnectReturnCode::BadUserNamePassword | ConnectReturnCode::NotAuthorized => "The broker refused the user name or password.".into(),
            ConnectReturnCode::BadClientId => "The broker refused the client id: choose another one.".into(),
            ConnectReturnCode::ServiceUnavailable => "The broker is not available right now.".into(),
            _ => "The broker refused the connection.".into(),
        },
        ConnectionError::Io(err) => {
            use std::io::ErrorKind::*;
            let text = err.to_string().to_lowercase();
            match err.kind() {
                ConnectionRefused => "Nothing answers on this host and port: is the MQTT broker running?".into(),
                TimedOut | WouldBlock => "The broker did not answer in time: check the address and the port.".into(),
                _ if text.contains("lookup") || text.contains("11001") || text.contains("resolve") || text.contains("no such host") => {
                    "Host name not found: check the address.".into()
                }
                ConnectionReset | ConnectionAborted | UnexpectedEof | BrokenPipe => {
                    "The connection was closed by the broker (wrong port, or TLS expected).".into()
                }
                _ => "Network error while reaching the broker.".into(),
            }
        }
        ConnectionError::Tls(_) => "Secure connection failed: the broker certificate is not trusted, or this port does not use TLS.".into(),
        ConnectionError::NetworkTimeout | ConnectionError::FlushTimeout => "The broker did not answer in time.".into(),
        ConnectionError::NotConnAck(_) => "This port does not speak MQTT.".into(),
        ConnectionError::MqttState(_) => "The connection to the broker was interrupted.".into(),
        _ => "Connection to the broker failed.".into(),
    }
}

/// Se connecte avec un identifiant dédié (`<id>-tmp`, pour ne pas éjecter la session principale), publie les
/// messages `(topic, charge, retenu)` en QoS 1, attend les accusés puis se déconnecte.
pub fn exchange(cfg: &HaConfig, password: Option<String>, client_id: &str, msgs: Vec<(String, Vec<u8>, bool)>) -> Result<(), String> {
    let mut opts = options(cfg, client_id, password, None);
    opts.set_keep_alive(Duration::from_secs(5));
    let (client, mut conn) = Client::new(opts, 16);
    conn.eventloop.set_network_options(network());
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut pending = msgs.len();
    let mut feed = Some(msgs);
    // `iter()` (et non `recv_timeout`) : annuler une connexion en cours la recommencerait indéfiniment.
    for ev in conn.iter() {
        if Instant::now() > deadline {
            return Err("The broker did not answer in time.".into());
        }
        match ev {
            Err(e) => return Err(friendly_error(&e)),
            Ok(Event::Incoming(Incoming::ConnAck(a))) => {
                if a.code != ConnectReturnCode::Success {
                    return Err(friendly_error(&ConnectionError::ConnectionRefused(a.code)));
                }
                if pending == 0 {
                    let _ = client.try_disconnect();
                    return Ok(());
                }
                // La file du client est bornée : un fil dédié publie en bloquant pendant que cette boucle fait avancer la connexion.
                if let Some(list) = feed.take() {
                    let c = client.clone();
                    std::thread::spawn(move || {
                        for (t, p, r) in list {
                            if c.publish(t, QoS::AtLeastOnce, r, p).is_err() {
                                break;
                            }
                        }
                    });
                }
            }
            Ok(Event::Incoming(Incoming::PubAck(_))) => {
                pending = pending.saturating_sub(1);
                if pending == 0 {
                    let _ = client.try_disconnect();
                    return Ok(());
                }
            }
            Ok(_) => {}
        }
    }
    Err("The connection was closed.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error, ErrorKind};

    #[test]
    fn plain_words() {
        let e = ConnectionError::ConnectionRefused(ConnectReturnCode::BadUserNamePassword);
        assert!(friendly_error(&e).contains("user name or password"));
        let e = ConnectionError::Io(Error::new(ErrorKind::ConnectionRefused, "x"));
        assert!(friendly_error(&e).contains("broker running"));
        let e = ConnectionError::Io(Error::new(ErrorKind::Other, "failed to lookup address information"));
        assert!(friendly_error(&e).contains("not found"));
        assert!(friendly_error(&ConnectionError::NetworkTimeout).contains("in time"));
    }

    #[test]
    fn options_use_will_and_credentials() {
        let cfg = HaConfig { host: "h".into(), port: 1883, username: "u".into(), ..Default::default() };
        let o = options(&cfg, "cid", Some("pw".into()), Some(("b/availability".into(), "offline")));
        assert_eq!(o.client_id(), "cid");
        assert_eq!(o.broker_address(), ("h".to_string(), 1883));
        let w = o.last_will().unwrap();
        assert_eq!(w.topic, "b/availability");
        assert!(w.retain);
    }
}
