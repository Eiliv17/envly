use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::core::crypto::keychain;
use crate::core::crypto::passphrase::PassphraseCipher;
use crate::core::crypto::symmetric::SymmetricCipher;
use crate::core::crypto::CipherKind;
use crate::core::registry::VaultEntry;
use crate::state::{AppState, VaultStatus};

type CmdResult<T> = std::result::Result<T, String>;

fn map_err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// -- Registry / multi-vault commands --

#[derive(Serialize)]
pub struct VaultEntrySummary {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub cipher_kind: CipherKind,
    pub created_at: chrono::DateTime<Utc>,
    pub last_accessed: Option<chrono::DateTime<Utc>>,
}

impl From<&VaultEntry> for VaultEntrySummary {
    fn from(e: &VaultEntry) -> Self {
        Self {
            id: e.id,
            name: e.name.clone(),
            path: e.path.to_string_lossy().to_string(),
            cipher_kind: e.cipher_kind,
            created_at: e.created_at,
            last_accessed: e.last_accessed,
        }
    }
}

#[tauri::command]
pub fn list_vaults(state: State<'_, AppState>) -> Vec<VaultEntrySummary> {
    let reg = state.registry.read().unwrap();
    reg.vaults.iter().map(VaultEntrySummary::from).collect()
}

#[tauri::command]
pub fn get_active_vault_id(state: State<'_, AppState>) -> Option<Uuid> {
    state.active_vault_id()
}

#[derive(Deserialize)]
pub struct CreateVaultEntryArgs {
    pub name: String,
    pub path: String,
    pub cipher_kind: CipherKind,
    pub passphrase: Option<String>,
}

#[tauri::command]
pub async fn create_vault_entry(
    app: AppHandle,
    state: State<'_, AppState>,
    args: CreateVaultEntryArgs,
) -> CmdResult<Uuid> {
    let vault_path = PathBuf::from(&args.path);

    if vault_path.is_file() {
        return Err("A file already exists at the specified path. Choose a different location or import the existing vault.".into());
    }

    let entry = VaultEntry {
        id: Uuid::new_v4(),
        name: args.name,
        path: vault_path,
        cipher_kind: args.cipher_kind,
        created_at: Utc::now(),
        last_accessed: None,
    };
    let id = entry.id;

    {
        let mut reg = state.registry.write().unwrap();
        reg.add_entry(entry).map_err(map_err)?;
    }
    state.save_registry().map_err(map_err)?;

    // Select and initialize the new vault
    state.select_vault(id).map_err(map_err)?;

    let cipher: Box<dyn crate::core::crypto::Cipher> = match args.cipher_kind {
        CipherKind::Passphrase => {
            let pass = args
                .passphrase
                .ok_or("Passphrase is required for passphrase mode")?;
            Box::new(PassphraseCipher::new(pass))
        }
        CipherKind::Symmetric => {
            let key = SymmetricCipher::generate_key();
            keychain::store_key(&key, &id).map_err(map_err)?;
            Box::new(SymmetricCipher::new(key))
        }
    };
    state.init(cipher).map_err(map_err)?;

    let _ = app.emit("vault:status-changed", ());
    Ok(id)
}

#[tauri::command]
pub async fn delete_vault_entry(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Uuid,
    delete_file: bool,
) -> CmdResult<()> {
    if state.active_vault_id() == Some(id) && state.is_unlocked() {
        state.lock().map_err(map_err)?;
    }

    let removed = {
        let mut reg = state.registry.write().unwrap();
        reg.remove_entry(id).map_err(map_err)?
    };
    state.save_registry().map_err(map_err)?;

    if delete_file && removed.path.is_file() {
        let _ = std::fs::remove_file(&removed.path);
    }

    let _ = app.emit("vault:status-changed", ());
    Ok(())
}

#[tauri::command]
pub fn rename_vault(app: AppHandle, state: State<'_, AppState>, id: Uuid, new_name: String) -> CmdResult<()> {
    {
        let mut reg = state.registry.write().unwrap();
        reg.rename_entry(id, new_name).map_err(map_err)?;
    }
    state.save_registry().map_err(map_err)?;
    let _ = app.emit("vault:status-changed", ());
    Ok(())
}

#[derive(Deserialize)]
pub struct ImportVaultEntryArgs {
    pub name: String,
    pub path: String,
}

#[tauri::command]
pub fn import_vault_entry(
    app: AppHandle,
    state: State<'_, AppState>,
    args: ImportVaultEntryArgs,
) -> CmdResult<Uuid> {
    let vault_path = PathBuf::from(&args.path);
    if !vault_path.is_file() {
        return Err("Vault file not found".into());
    }

    let contents = std::fs::read_to_string(&vault_path).map_err(map_err)?;
    let vault_file: serde_json::Value =
        serde_json::from_str(&contents).map_err(|_| "Not a valid vault file".to_string())?;
    let cipher_kind_str = vault_file
        .get("cipher_kind")
        .and_then(|v| v.as_str())
        .ok_or("Not a valid Envly vault file")?;
    let cipher_kind: CipherKind =
        serde_json::from_value(serde_json::Value::String(cipher_kind_str.into()))
            .map_err(|_| "Unknown cipher kind".to_string())?;

    let entry = VaultEntry {
        id: Uuid::new_v4(),
        name: args.name,
        path: vault_path,
        cipher_kind,
        created_at: Utc::now(),
        last_accessed: None,
    };
    let id = entry.id;

    {
        let mut reg = state.registry.write().unwrap();
        reg.add_entry(entry).map_err(map_err)?;
    }
    state.save_registry().map_err(map_err)?;

    let _ = app.emit("vault:status-changed", ());
    Ok(id)
}

#[tauri::command]
pub fn select_vault(app: AppHandle, state: State<'_, AppState>, id: Uuid) -> CmdResult<VaultStatus> {
    let status = state.select_vault(id).map_err(map_err)?;
    let _ = app.emit("vault:status-changed", ());
    Ok(status)
}

#[tauri::command]
pub fn check_path_exists(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

// -- Existing vault commands (now vault-specific) --

#[tauri::command]
pub fn get_vault_status(state: State<'_, AppState>) -> VaultStatus {
    state.status()
}

#[tauri::command]
pub async fn unlock_vault(app: AppHandle, state: State<'_, AppState>, passphrase: Option<String>) -> CmdResult<()> {
    let cipher: Box<dyn crate::core::crypto::Cipher> = if let Some(pass) = passphrase {
        Box::new(PassphraseCipher::new(pass))
    } else {
        let vault_id = state.active_vault_id().ok_or("No vault selected")?;
        let key = keychain::retrieve_key(&vault_id).map_err(map_err)?;
        Box::new(SymmetricCipher::new(key))
    };
    state.unlock(cipher).map_err(map_err)?;
    let _ = app.emit("vault:status-changed", ());
    Ok(())
}

#[tauri::command]
pub fn lock_vault(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    state.lock().map_err(map_err)?;
    let _ = app.emit("vault:status-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn change_passphrase(
    app: AppHandle,
    state: State<'_, AppState>,
    new_passphrase: String,
) -> CmdResult<()> {
    state.require_unlocked().map_err(map_err)?;

    let new_cipher = Box::new(PassphraseCipher::new(new_passphrase));

    {
        let vault_guard = state.vault().read().unwrap();
        let vault = vault_guard.as_ref().ok_or("Vault is locked")?;
        let path = state.active_vault_path_public().map_err(map_err)?;
        crate::core::vault::save_vault(&path, new_cipher.as_ref(), vault).map_err(map_err)?;
    }

    state.lock().map_err(map_err)?;
    state.unlock(new_cipher).map_err(map_err)?;

    let _ = app.emit("vault:status-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn export_vault(
    state: State<'_, AppState>,
    destination: String,
    passphrase: String,
) -> CmdResult<()> {
    state.require_unlocked().map_err(map_err)?;

    let export_cipher = PassphraseCipher::new(passphrase);
    let dest_path = PathBuf::from(&destination);

    let vault_guard = state.vault().read().unwrap();
    let vault = vault_guard.as_ref().ok_or("Vault is locked")?;
    crate::core::vault::save_vault(&dest_path, &export_cipher, vault).map_err(map_err)?;

    Ok(())
}

#[tauri::command]
pub fn get_active_vault_cipher_kind(state: State<'_, AppState>) -> CmdResult<String> {
    let id = state.active_vault_id().ok_or("No vault selected")?;
    let reg = state.registry.read().unwrap();
    let entry = reg
        .find_entry(id)
        .ok_or("Active vault not found in registry")?;
    Ok(entry.cipher_kind.to_string())
}

#[tauri::command]
pub async fn migrate_cipher(
    app: AppHandle,
    state: State<'_, AppState>,
    target_kind: CipherKind,
    passphrase: Option<String>,
) -> CmdResult<()> {
    state.require_unlocked().map_err(map_err)?;

    let vault_id = state.active_vault_id().ok_or("No vault selected")?;

    let current_kind = {
        let reg = state.registry.read().unwrap();
        let entry = reg
            .find_entry(vault_id)
            .ok_or("Active vault not found in registry")?;
        entry.cipher_kind
    };

    if current_kind == target_kind {
        return Err("Vault already uses this cipher mode".into());
    }

    // Build two cipher instances: one to save now, one for unlock after lock.
    // lock() forces a flush_if_dirty() using the cipher in state, so we must
    // replace it before calling lock().
    let (save_cipher, unlock_cipher): (Box<dyn crate::core::crypto::Cipher>, Box<dyn crate::core::crypto::Cipher>) = match target_kind {
        CipherKind::Passphrase => {
            let pass = passphrase.ok_or("Passphrase is required when migrating to passphrase mode")?;
            (Box::new(PassphraseCipher::new(pass.clone())), Box::new(PassphraseCipher::new(pass)))
        }
        CipherKind::Symmetric => {
            let key = SymmetricCipher::generate_key();
            keychain::store_key(&key, &vault_id).map_err(map_err)?;
            (Box::new(SymmetricCipher::new(key)), Box::new(SymmetricCipher::new(key)))
        }
    };

    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;
        let path = state.active_vault_path_public().map_err(map_err)?;
        vault.cipher_kind = target_kind.to_string();
        crate::core::vault::save_vault(&path, save_cipher.as_ref(), vault).map_err(map_err)?;
    }

    // Swap the cipher in state so lock()'s flush uses the new cipher
    state.replace_cipher(save_cipher);

    {
        let mut reg = state.registry.write().unwrap();
        if let Some(entry) = reg.find_entry_mut(vault_id) {
            entry.cipher_kind = target_kind;
        }
    }
    state.save_registry().map_err(map_err)?;

    if current_kind == CipherKind::Symmetric {
        let _ = keychain::delete_key(&vault_id);
    }

    state.lock().map_err(map_err)?;
    state.unlock(unlock_cipher).map_err(map_err)?;

    let _ = app.emit("vault:status-changed", ());
    Ok(())
}

