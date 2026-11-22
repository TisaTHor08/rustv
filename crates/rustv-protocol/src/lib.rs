//! Protocole RusTV : messages, trames, handshake HMAC et chiffrement AES-256-GCM.
//! Indépendant du transport (TCP, Bluetooth RFCOMM) : tout passe par `Read + Write`.

pub mod crypto;
pub mod frame;
pub mod messages;
pub mod pairing;
pub mod session;

pub const PROTOCOL_VERSION: u32 = 1;
pub const TCP_PORT: u16 = 47820;
pub const BEACON_PORT: u16 = 47821;
/// UUID du service RFCOMM Bluetooth.
pub const RFCOMM_UUID: &str = "7c1d3a52-5b0e-4f8a-9d63-2e41c0a8b9f7";
