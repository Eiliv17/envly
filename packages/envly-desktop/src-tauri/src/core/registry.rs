use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::crypto::CipherKind;
use crate::core::error::{EnvlyError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub id: Uuid,
    pub name: String,
    pub path: PathBuf,
    pub cipher_kind: CipherKind,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultRegistry {
    pub vaults: Vec<VaultEntry>,
}

impl VaultRegistry {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.is_file() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(path)?;
        let registry: Self = serde_json::from_str(&contents)
            .map_err(|_| EnvlyError::Vault("Corrupt registry file".into()))?;
        Ok(registry)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let encoded = serde_json::to_vec_pretty(self)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, encoded)?;
        Ok(())
    }

    pub fn find_entry(&self, id: Uuid) -> Option<&VaultEntry> {
        self.vaults.iter().find(|v| v.id == id)
    }

    pub fn find_entry_mut(&mut self, id: Uuid) -> Option<&mut VaultEntry> {
        self.vaults.iter_mut().find(|v| v.id == id)
    }

    pub fn add_entry(&mut self, entry: VaultEntry) -> Result<()> {
        if self.vaults.iter().any(|v| v.name == entry.name) {
            return Err(EnvlyError::Validation(format!(
                "A vault named '{}' already exists",
                entry.name
            )));
        }
        if self.vaults.iter().any(|v| v.path == entry.path) {
            return Err(EnvlyError::Validation(format!(
                "A vault at path '{}' is already registered",
                entry.path.display()
            )));
        }
        self.vaults.push(entry);
        Ok(())
    }

    pub fn remove_entry(&mut self, id: Uuid) -> Result<VaultEntry> {
        let idx = self
            .vaults
            .iter()
            .position(|v| v.id == id)
            .ok_or_else(|| EnvlyError::Vault("Vault entry not found".into()))?;
        Ok(self.vaults.remove(idx))
    }

    pub fn rename_entry(&mut self, id: Uuid, new_name: String) -> Result<()> {
        if self.vaults.iter().any(|v| v.name == new_name && v.id != id) {
            return Err(EnvlyError::Validation(format!(
                "A vault named '{new_name}' already exists"
            )));
        }
        let entry = self
            .find_entry_mut(id)
            .ok_or_else(|| EnvlyError::Vault("Vault entry not found".into()))?;
        entry.name = new_name;
        Ok(())
    }

    pub fn prune_stale(&mut self) -> Vec<VaultEntry> {
        let (keep, stale): (Vec<_>, Vec<_>) = std::mem::take(&mut self.vaults)
            .into_iter()
            .partition(|e| e.path.is_file());
        self.vaults = keep;
        stale
    }

    pub fn update_last_accessed(&mut self, id: Uuid) {
        if let Some(entry) = self.find_entry_mut(id) {
            entry.last_accessed = Some(Utc::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_entry(name: &str) -> VaultEntry {
        VaultEntry {
            id: Uuid::new_v4(),
            name: name.to_string(),
            path: PathBuf::from(format!("/tmp/{name}.envly")),
            cipher_kind: CipherKind::Passphrase,
            created_at: Utc::now(),
            last_accessed: None,
        }
    }

    #[test]
    fn test_add_and_find() {
        let mut reg = VaultRegistry::default();
        let entry = sample_entry("work");
        let id = entry.id;
        reg.add_entry(entry).unwrap();
        assert!(reg.find_entry(id).is_some());
    }

    #[test]
    fn test_duplicate_name_rejected() {
        let mut reg = VaultRegistry::default();
        reg.add_entry(sample_entry("work")).unwrap();
        let result = reg.add_entry(sample_entry("work"));
        assert!(result.is_err());
    }

    #[test]
    fn test_remove() {
        let mut reg = VaultRegistry::default();
        let entry = sample_entry("temp");
        let id = entry.id;
        reg.add_entry(entry).unwrap();
        reg.remove_entry(id).unwrap();
        assert!(reg.find_entry(id).is_none());
    }

    #[test]
    fn test_rename() {
        let mut reg = VaultRegistry::default();
        let entry = sample_entry("old");
        let id = entry.id;
        reg.add_entry(entry).unwrap();
        reg.rename_entry(id, "new".to_string()).unwrap();
        assert_eq!(reg.find_entry(id).unwrap().name, "new");
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let mut reg = VaultRegistry::default();
        reg.add_entry(sample_entry("alpha")).unwrap();
        reg.add_entry(sample_entry("beta")).unwrap();
        reg.save(&path).unwrap();

        let loaded = VaultRegistry::load(&path).unwrap();
        assert_eq!(loaded.vaults.len(), 2);
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let reg = VaultRegistry::load(Path::new("/nonexistent/registry.json")).unwrap();
        assert!(reg.vaults.is_empty());
    }

    #[test]
    fn test_update_last_accessed() {
        let mut reg = VaultRegistry::default();
        let entry = sample_entry("main");
        let id = entry.id;
        reg.add_entry(entry).unwrap();
        assert!(reg.find_entry(id).unwrap().last_accessed.is_none());
        reg.update_last_accessed(id);
        assert!(reg.find_entry(id).unwrap().last_accessed.is_some());
    }

    // --- New registry tests ---

    #[test]
    fn test_add_entry_duplicate_path_rejected() {
        let mut reg = VaultRegistry::default();
        let mut e1 = sample_entry("alpha");
        e1.path = PathBuf::from("/tmp/shared.envly");
        reg.add_entry(e1).unwrap();

        let mut e2 = sample_entry("beta");
        e2.path = PathBuf::from("/tmp/shared.envly");
        let result = reg.add_entry(e2);
        assert!(result.is_err());
    }

    #[test]
    fn test_prune_stale_removes_missing_files() {
        let dir = tempdir().unwrap();
        let mut reg = VaultRegistry::default();

        // Create a real file for one entry
        let real_path = dir.path().join("real.envly");
        std::fs::write(&real_path, "data").unwrap();
        let mut e1 = sample_entry("real");
        e1.path = real_path;
        reg.add_entry(e1).unwrap();

        // A fake path that doesn't exist
        let mut e2 = sample_entry("gone");
        e2.path = PathBuf::from("/nonexistent/gone.envly");
        reg.add_entry(e2).unwrap();

        assert_eq!(reg.vaults.len(), 2);
        let stale = reg.prune_stale();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].name, "gone");
        assert_eq!(reg.vaults.len(), 1);
        assert_eq!(reg.vaults[0].name, "real");
    }

    #[test]
    fn test_rename_to_duplicate_name_rejected() {
        let mut reg = VaultRegistry::default();
        let e1 = sample_entry("alpha");
        let e2 = sample_entry("beta");
        let e2_id = e2.id;
        reg.add_entry(e1).unwrap();
        reg.add_entry(e2).unwrap();

        let result = reg.rename_entry(e2_id, "alpha".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_nonexistent_entry_fails() {
        let mut reg = VaultRegistry::default();
        let result = reg.rename_entry(Uuid::new_v4(), "new_name".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_nonexistent_entry_fails() {
        let mut reg = VaultRegistry::default();
        let result = reg.remove_entry(Uuid::new_v4());
        assert!(result.is_err());
    }
}
