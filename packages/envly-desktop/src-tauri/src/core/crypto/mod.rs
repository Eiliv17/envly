pub mod keychain;
pub mod passphrase;
pub mod symmetric;

use serde::{Deserialize, Serialize};

use crate::core::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CipherKind {
    Passphrase,
    Symmetric,
}

impl std::fmt::Display for CipherKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Passphrase => write!(f, "passphrase"),
            Self::Symmetric => write!(f, "symmetric"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub ciphertext: String,
    pub nonce: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub salt: Option<String>,
}

pub trait Cipher: Send + Sync {
    fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedPayload>;
    fn decrypt(&self, payload: &EncryptedPayload) -> Result<Vec<u8>>;
    fn kind(&self) -> CipherKind;
}
