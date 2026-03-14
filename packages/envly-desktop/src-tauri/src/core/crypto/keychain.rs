use base64::{engine::general_purpose::STANDARD, Engine};
use uuid::Uuid;

use crate::core::error::{EnvlyError, Result};

const SERVICE_NAME: &str = "com.envly.envly";

fn key_entry(vault_id: &Uuid) -> String {
    format!("vault-master-key-{vault_id}")
}

pub fn store_key(key: &[u8; 32], vault_id: &Uuid) -> Result<()> {
    let entry_name = key_entry(vault_id);
    let entry = keyring::Entry::new(SERVICE_NAME, &entry_name)
        .map_err(|e| EnvlyError::Keychain(e.to_string()))?;
    let encoded = STANDARD.encode(key);
    entry
        .set_password(&encoded)
        .map_err(|e| EnvlyError::Keychain(e.to_string()))?;
    Ok(())
}

pub fn retrieve_key(vault_id: &Uuid) -> Result<[u8; 32]> {
    let entry_name = key_entry(vault_id);
    let entry = keyring::Entry::new(SERVICE_NAME, &entry_name)
        .map_err(|e| EnvlyError::Keychain(e.to_string()))?;
    let encoded = entry
        .get_password()
        .map_err(|e| EnvlyError::Keychain(e.to_string()))?;
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|e| EnvlyError::Keychain(e.to_string()))?;
    if bytes.len() != 32 {
        return Err(EnvlyError::Keychain("Stored key has invalid length".into()));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

pub fn delete_key(vault_id: &Uuid) -> Result<()> {
    let entry_name = key_entry(vault_id);
    let entry = keyring::Entry::new(SERVICE_NAME, &entry_name)
        .map_err(|e| EnvlyError::Keychain(e.to_string()))?;
    entry
        .delete_credential()
        .map_err(|e| EnvlyError::Keychain(e.to_string()))?;
    Ok(())
}

#[allow(dead_code)]
pub fn has_key(vault_id: &Uuid) -> bool {
    let entry_name = key_entry(vault_id);
    let entry = match keyring::Entry::new(SERVICE_NAME, &entry_name) {
        Ok(e) => e,
        Err(_) => return false,
    };
    entry.get_password().is_ok()
}
