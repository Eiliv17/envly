use std::fmt;

#[derive(Debug)]
pub enum EnvlyError {
    Crypto(String),
    Vault(String),
    Io(std::io::Error),
    Serialization(serde_json::Error),
    Keychain(String),
    Validation(String),
}

impl fmt::Display for EnvlyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Crypto(msg) => write!(f, "Crypto error: {msg}"),
            Self::Vault(msg) => write!(f, "Vault error: {msg}"),
            Self::Io(err) => write!(f, "IO error: {err}"),
            Self::Serialization(err) => write!(f, "Serialization error: {err}"),
            Self::Keychain(msg) => write!(f, "Keychain error: {msg}"),
            Self::Validation(msg) => write!(f, "Validation error: {msg}"),
        }
    }
}

impl std::error::Error for EnvlyError {}

impl From<std::io::Error> for EnvlyError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for EnvlyError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err)
    }
}

pub type Result<T> = std::result::Result<T, EnvlyError>;
