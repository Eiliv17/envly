use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

use uuid::Uuid;

use crate::core::crypto::Cipher;
use crate::core::env::symlink;
use crate::core::error::{EnvlyError, Result};
use crate::core::registry::VaultRegistry;
use crate::core::vault;
use crate::core::vault::models::Vault;

pub struct AppState {
    pub registry: RwLock<VaultRegistry>,
    pub registry_path: PathBuf,
    pub tmp_dir: PathBuf,
    active_vault_id: RwLock<Option<Uuid>>,
    vault: RwLock<Option<Vault>>,
    cipher: RwLock<Option<Box<dyn Cipher>>>,
    dirty: AtomicBool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VaultStatus {
    Uninitialized,
    Locked,
    Unlocked,
}

impl AppState {
    pub fn new(registry_path: PathBuf, registry: VaultRegistry, tmp_dir: PathBuf) -> Self {
        Self {
            registry: RwLock::new(registry),
            registry_path,
            tmp_dir,
            active_vault_id: RwLock::new(None),
            vault: RwLock::new(None),
            cipher: RwLock::new(None),
            dirty: AtomicBool::new(false),
        }
    }

    pub fn active_vault_id(&self) -> Option<Uuid> {
        *self.active_vault_id.read().unwrap()
    }

    pub fn active_vault_path_public(&self) -> Result<PathBuf> {
        self.active_vault_path()
    }

    fn active_vault_path(&self) -> Result<PathBuf> {
        let id = self
            .active_vault_id()
            .ok_or_else(|| EnvlyError::Vault("No vault selected".into()))?;
        let reg = self.registry.read().unwrap();
        let entry = reg
            .find_entry(id)
            .ok_or_else(|| EnvlyError::Vault("Selected vault not found in registry".into()))?;
        Ok(entry.path.clone())
    }

    pub fn select_vault(&self, id: Uuid) -> Result<VaultStatus> {
        {
            let reg = self.registry.read().unwrap();
            reg.find_entry(id)
                .ok_or_else(|| EnvlyError::Vault("Vault not found in registry".into()))?;
        }
        if self.is_unlocked() {
            self.lock()?;
        }
        *self.active_vault_id.write().unwrap() = Some(id);

        let path = self.active_vault_path()?;
        if vault::vault_exists(&path) {
            Ok(VaultStatus::Locked)
        } else {
            Ok(VaultStatus::Uninitialized)
        }
    }

    pub fn status(&self) -> VaultStatus {
        if self.active_vault_id().is_none() {
            return VaultStatus::Uninitialized;
        }
        let path = match self.active_vault_path() {
            Ok(p) => p,
            Err(_) => return VaultStatus::Uninitialized,
        };
        if !vault::vault_exists(&path) {
            return VaultStatus::Uninitialized;
        }
        if self.vault.read().unwrap().is_some() {
            VaultStatus::Unlocked
        } else {
            VaultStatus::Locked
        }
    }

    pub fn is_unlocked(&self) -> bool {
        self.vault.read().unwrap().is_some()
    }

    pub fn unlock(&self, cipher: Box<dyn Cipher>) -> Result<()> {
        let path = self.active_vault_path()?;
        let loaded = vault::load_vault(&path, cipher.as_ref())?;
        *self.vault.write().unwrap() = Some(loaded);
        *self.cipher.write().unwrap() = Some(cipher);

        let id = self.active_vault_id().unwrap();
        self.registry.write().unwrap().update_last_accessed(id);
        let _ = self.save_registry();

        Ok(())
    }

    pub fn init(&self, cipher: Box<dyn Cipher>) -> Result<()> {
        let path = self.active_vault_path()?;
        let created = vault::create_vault(&path, cipher.as_ref())?;
        *self.vault.write().unwrap() = Some(created);
        *self.cipher.write().unwrap() = Some(cipher);
        Ok(())
    }

    pub fn lock(&self) -> Result<()> {
        // Deactivate all environments so the saved vault reflects reality
        {
            let mut vault_guard = self.vault.write().unwrap();
            if let Some(ref mut v) = *vault_guard {
                for project in &mut v.projects {
                    for env in &mut project.environments {
                        env.is_active = false;
                    }
                }
            }
        }
        // Flush the corrected state to disk
        self.dirty.store(true, Ordering::Relaxed);
        let _ = self.flush_if_dirty();
        // Clean up all symlinks and temp files
        let _ = symlink::cleanup_all(&self.tmp_dir);
        *self.vault.write().unwrap() = None;
        *self.cipher.write().unwrap() = None;
        self.dirty.store(false, Ordering::Relaxed);
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let path = self.active_vault_path()?;
        let vault_guard = self.vault.read().unwrap();
        let vault = vault_guard
            .as_ref()
            .ok_or_else(|| EnvlyError::Vault("Vault is locked".into()))?;
        let cipher_guard = self.cipher.read().unwrap();
        let cipher = cipher_guard
            .as_ref()
            .ok_or_else(|| EnvlyError::Vault("No cipher available".into()))?;
        vault::save_vault(&path, cipher.as_ref(), vault)?;
        Ok(())
    }

    pub fn save_registry(&self) -> Result<()> {
        let reg = self.registry.read().unwrap();
        reg.save(&self.registry_path)
    }

    pub fn vault(&self) -> &RwLock<Option<Vault>> {
        &self.vault
    }

    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Relaxed);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Relaxed)
    }

    pub fn flush_if_dirty(&self) -> Result<()> {
        if self.dirty.swap(false, Ordering::Relaxed) {
            self.save()?;
        }
        Ok(())
    }

    pub fn replace_cipher(&self, cipher: Box<dyn Cipher>) {
        *self.cipher.write().unwrap() = Some(cipher);
    }

    pub fn require_unlocked(&self) -> Result<()> {
        if !self.is_unlocked() {
            return Err(EnvlyError::Vault("Vault is locked".into()));
        }
        Ok(())
    }
}
