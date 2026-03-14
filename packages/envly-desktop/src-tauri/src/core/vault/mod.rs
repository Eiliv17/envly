pub mod models;

use std::fs;
use std::path::Path;

use crate::core::crypto::{Cipher, CipherKind, EncryptedPayload};
use crate::core::error::{EnvlyError, Result};

use models::Vault;

#[derive(serde::Serialize, serde::Deserialize)]
struct VaultFile {
    cipher_version: u32,
    cipher_kind: CipherKind,
    payload: EncryptedPayload,
}

pub fn vault_exists(path: &Path) -> bool {
    path.is_file()
}

pub fn create_vault(path: &Path, cipher: &dyn Cipher) -> Result<Vault> {
    let vault = Vault::new(&cipher.kind().to_string(), 1);
    save_vault(path, cipher, &vault)?;
    Ok(vault)
}

pub fn load_vault(path: &Path, cipher: &dyn Cipher) -> Result<Vault> {
    let contents = fs::read_to_string(path)?;
    let vault_file: VaultFile = serde_json::from_str(&contents)
        .map_err(|_| EnvlyError::Vault("Corrupt vault file".into()))?;

    let plaintext = cipher.decrypt(&vault_file.payload)?;
    let vault: Vault = serde_json::from_slice(&plaintext)
        .map_err(|_| EnvlyError::Vault("Failed to deserialize vault contents".into()))?;

    Ok(vault)
}

pub fn save_vault(path: &Path, cipher: &dyn Cipher, vault: &Vault) -> Result<()> {
    let plaintext = serde_json::to_vec(vault)?;
    let payload = cipher.encrypt(&plaintext)?;

    let vault_file = VaultFile {
        cipher_version: vault.cipher_version,
        cipher_kind: cipher.kind(),
        payload,
    };

    let encoded = serde_json::to_vec_pretty(&vault_file)?;

    // Backup existing vault before overwriting
    if path.is_file() {
        let backup = path.with_extension("envly.bak");
        fs::copy(path, &backup)?;
    }

    // Atomic write: write to temp file then rename
    let parent = path
        .parent()
        .ok_or_else(|| EnvlyError::Vault("Vault path has no parent directory".into()))?;
    let tmp_path = parent.join(".vault.envly.tmp");
    fs::write(&tmp_path, &encoded)?;
    fs::rename(&tmp_path, path)?;

    // Clean up backup after successful write
    let backup = path.with_extension("envly.bak");
    let _ = fs::remove_file(&backup);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::crypto::symmetric::SymmetricCipher;
    use tempfile::tempdir;

    fn test_cipher() -> SymmetricCipher {
        SymmetricCipher::new(SymmetricCipher::generate_key())
    }

    #[test]
    fn create_and_load_round_trip() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("vault.envly");
        let cipher = test_cipher();

        let mut vault = create_vault(&vault_path, &cipher).unwrap();
        vault
            .add_secret(models::Secret::new("KEY".into(), "VALUE".into()))
            .unwrap();
        vault
            .add_project(models::Project::new("app".into(), "/tmp/app".into()))
            .unwrap();
        save_vault(&vault_path, &cipher, &vault).unwrap();

        let loaded = load_vault(&vault_path, &cipher).unwrap();
        assert_eq!(loaded.secrets.len(), 1);
        assert_eq!(loaded.secrets[0].key, "KEY");
        assert_eq!(*loaded.secrets[0].value, "VALUE");
        assert_eq!(loaded.projects.len(), 1);
        assert_eq!(loaded.projects[0].name, "app");
    }

    #[test]
    fn backup_cleaned_up_after_successful_save() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("vault.envly");
        let cipher = test_cipher();

        create_vault(&vault_path, &cipher).unwrap();
        let backup_path = vault_path.with_extension("envly.bak");
        assert!(
            !backup_path.exists(),
            "No backup should exist after first create"
        );

        // Second save creates a backup during write but removes it on success
        let vault = load_vault(&vault_path, &cipher).unwrap();
        save_vault(&vault_path, &cipher, &vault).unwrap();
        assert!(
            !backup_path.exists(),
            "Backup should be cleaned up after a successful write"
        );

        // Vault file itself should still be valid
        let reloaded = load_vault(&vault_path, &cipher).unwrap();
        assert_eq!(reloaded.cipher_version, vault.cipher_version);
    }

    #[test]
    fn wrong_key_fails_to_load() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("vault.envly");
        let cipher1 = test_cipher();
        let cipher2 = test_cipher();

        create_vault(&vault_path, &cipher1).unwrap();
        let result = load_vault(&vault_path, &cipher2);
        assert!(result.is_err());
    }

    #[test]
    fn corrupt_file_fails_gracefully() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("vault.envly");
        std::fs::write(&vault_path, "not valid json at all").unwrap();

        let cipher = test_cipher();
        let result = load_vault(&vault_path, &cipher);
        assert!(result.is_err());
    }

    #[test]
    fn vault_exists_check() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("vault.envly");
        assert!(!vault_exists(&vault_path));

        let cipher = test_cipher();
        create_vault(&vault_path, &cipher).unwrap();
        assert!(vault_exists(&vault_path));
    }

    #[test]
    fn atomic_write_no_partial_file() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("vault.envly");
        let cipher = test_cipher();

        create_vault(&vault_path, &cipher).unwrap();

        // The temp file should not remain after a successful write
        let tmp_path = dir.path().join(".vault.envly.tmp");
        assert!(!tmp_path.exists());
    }

    // --- New vault file tests ---

    #[test]
    fn save_vault_to_nonexistent_parent_fails() {
        let vault_path = std::path::PathBuf::from("/nonexistent/deep/path/vault.envly");
        let cipher = test_cipher();
        let vault = Vault::new(&cipher.kind().to_string(), 1);
        let result = save_vault(&vault_path, &cipher, &vault);
        assert!(result.is_err());
    }

    #[test]
    fn create_vault_is_loadable() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("test.envly");
        let cipher = test_cipher();

        create_vault(&vault_path, &cipher).unwrap();
        assert!(vault_exists(&vault_path));

        let loaded = load_vault(&vault_path, &cipher).unwrap();
        assert!(loaded.secrets.is_empty());
        assert!(loaded.projects.is_empty());
        assert_eq!(loaded.cipher_version, 1);
    }

    #[test]
    fn save_preserves_mutations() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("test.envly");
        let cipher = test_cipher();

        let mut vault = create_vault(&vault_path, &cipher).unwrap();
        vault
            .add_secret(models::Secret::new("A".into(), "1".into()))
            .unwrap();
        vault
            .add_secret(models::Secret::new("B".into(), "2".into()))
            .unwrap();
        vault
            .add_project(models::Project::new("proj".into(), "/tmp".into()))
            .unwrap();
        save_vault(&vault_path, &cipher, &vault).unwrap();

        let loaded = load_vault(&vault_path, &cipher).unwrap();
        assert_eq!(loaded.secrets.len(), 2);
        assert_eq!(loaded.projects.len(), 1);
    }
}
