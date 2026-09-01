use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    /// SSH private key 원문. sandbox storage에만 영속화되고 프론트로는 내려가지 않음
    pub private_key: String,
    pub has_passphrase: bool,
}
