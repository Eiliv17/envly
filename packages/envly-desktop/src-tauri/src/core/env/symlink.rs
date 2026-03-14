use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::error::{EnvlyError, Result};

#[derive(Debug, Serialize, Deserialize)]
struct ManifestEntry {
    id: String,
    temp_path: PathBuf,
    symlink_path: PathBuf,
    pid: u32,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Manifest {
    entries: Vec<ManifestEntry>,
}

const MANIFEST_FILE: &str = "active_envs.json";
const STALE_TTL_HOURS: i64 = 24;

fn manifest_path(tmp_dir: &Path) -> PathBuf {
    tmp_dir.join(MANIFEST_FILE)
}

fn load_manifest(tmp_dir: &Path) -> Manifest {
    let path = manifest_path(tmp_dir);
    if !path.is_file() {
        return Manifest::default();
    }
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Manifest::default(),
    }
}

fn save_manifest(tmp_dir: &Path, manifest: &Manifest) -> Result<()> {
    let path = manifest_path(tmp_dir);
    let encoded = serde_json::to_string_pretty(manifest)?;
    fs::write(path, encoded)?;
    Ok(())
}

/// Check whether the current OS/user can create symlinks.
/// On Unix this is always true. On Windows it requires Developer Mode or admin.
#[allow(dead_code)]
pub fn can_create_symlinks() -> bool {
    #[cfg(unix)]
    {
        true
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_file;
        let dir = std::env::temp_dir();
        let target = dir.join(".envly_symlink_test_target");
        let link = dir.join(".envly_symlink_test_link");
        let _ = fs::write(&target, "");
        let result = symlink_file(&target, &link).is_ok();
        let _ = fs::remove_file(&link);
        let _ = fs::remove_file(&target);
        result
    }
}

/// Describes what currently exists at the target path.
#[derive(Debug, PartialEq)]
pub enum ExistingFile {
    None,
    EnvlySymlink,
    ForeignSymlink,
    RegularFile,
}

/// Detect what kind of file exists at the given path.
pub fn detect_existing(symlink_path: &Path, tmp_dir: &Path) -> ExistingFile {
    if !symlink_path.exists() && !symlink_path.symlink_metadata().is_ok() {
        return ExistingFile::None;
    }

    match fs::read_link(symlink_path) {
        Ok(target) => {
            if target.starts_with(tmp_dir) {
                ExistingFile::EnvlySymlink
            } else {
                ExistingFile::ForeignSymlink
            }
        }
        Err(_) => ExistingFile::RegularFile,
    }
}

/// Create a temp file with the resolved env content and symlink it into the project.
/// Returns the path to the created temp file.
pub fn activate(
    project_path: &Path,
    env_filename: &str,
    env_content: &str,
    tmp_dir: &Path,
) -> Result<PathBuf> {
    fs::create_dir_all(tmp_dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o700);
        fs::set_permissions(tmp_dir, perms)?;
    }

    let temp_name = format!("env-{}", Uuid::new_v4());
    let temp_path = tmp_dir.join(&temp_name);

    fs::write(&temp_path, env_content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&temp_path, perms)?;
    }

    let symlink_path = project_path.join(env_filename);

    // Handle existing file at the symlink target
    match detect_existing(&symlink_path, tmp_dir) {
        ExistingFile::None => {}
        ExistingFile::EnvlySymlink => {
            fs::remove_file(&symlink_path)?;
        }
        ExistingFile::RegularFile => {
            let backup = project_path.join(format!("{env_filename}.bak"));
            fs::rename(&symlink_path, &backup)?;
        }
        ExistingFile::ForeignSymlink => {
            return Err(EnvlyError::Validation(format!(
                "An unrecognized symlink already exists at '{}'. Please remove it manually.",
                symlink_path.display()
            )));
        }
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(&temp_path, &symlink_path)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&temp_path, &symlink_path)?;

    let mut manifest = load_manifest(tmp_dir);
    manifest.entries.push(ManifestEntry {
        id: temp_name,
        temp_path: temp_path.clone(),
        symlink_path,
        pid: std::process::id(),
        created_at: Utc::now(),
    });
    save_manifest(tmp_dir, &manifest)?;

    Ok(temp_path)
}

/// Remove a symlink and its backing temp file for a project.
pub fn deactivate(project_path: &Path, env_filename: &str, tmp_dir: &Path) -> Result<()> {
    let symlink_path = project_path.join(env_filename);

    if let ExistingFile::EnvlySymlink = detect_existing(&symlink_path, tmp_dir) {
        if let Ok(target) = fs::read_link(&symlink_path) {
            let _ = fs::remove_file(&target);
        }
        fs::remove_file(&symlink_path)?;
    }

    // Update manifest
    let mut manifest = load_manifest(tmp_dir);
    manifest.entries.retain(|e| e.symlink_path != symlink_path);
    save_manifest(tmp_dir, &manifest)?;

    Ok(())
}

/// Remove ALL temp files and symlinks tracked in the manifest.
pub fn cleanup_all(tmp_dir: &Path) -> Result<()> {
    let manifest = load_manifest(tmp_dir);

    for entry in &manifest.entries {
        let _ = fs::remove_file(&entry.temp_path);
        if entry.symlink_path.symlink_metadata().is_ok() {
            let _ = fs::remove_file(&entry.symlink_path);
        }
    }

    save_manifest(tmp_dir, &Manifest::default())?;
    Ok(())
}

/// Clean up temp files and symlinks from dead processes or expired TTL.
pub fn cleanup_stale(tmp_dir: &Path) -> Result<()> {
    let mut manifest = load_manifest(tmp_dir);
    let now = Utc::now();

    manifest.entries.retain(|entry| {
        let is_stale =
            !is_pid_alive(entry.pid) || (now - entry.created_at).num_hours() >= STALE_TTL_HOURS;

        if is_stale {
            let _ = fs::remove_file(&entry.temp_path);
            if entry.symlink_path.symlink_metadata().is_ok() {
                let _ = fs::remove_file(&entry.symlink_path);
            }
            return false;
        }
        true
    });

    save_manifest(tmp_dir, &manifest)?;
    Ok(())
}

fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn activate_creates_symlink_and_temp() {
        let project_dir = tempdir().unwrap();
        let tmp_dir = tempdir().unwrap();

        let content = "KEY=value\n";
        let temp_path = activate(project_dir.path(), ".env", content, tmp_dir.path()).unwrap();

        assert!(temp_path.is_file());
        let symlink_path = project_dir.path().join(".env");
        assert!(symlink_path.symlink_metadata().is_ok());

        let read_content = fs::read_to_string(&symlink_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn deactivate_removes_symlink_and_temp() {
        let project_dir = tempdir().unwrap();
        let tmp_dir = tempdir().unwrap();

        let temp_path = activate(project_dir.path(), ".env", "K=V\n", tmp_dir.path()).unwrap();
        deactivate(project_dir.path(), ".env", tmp_dir.path()).unwrap();

        assert!(!temp_path.exists());
        assert!(!project_dir.path().join(".env").exists());
    }

    #[test]
    fn activate_backs_up_existing_regular_file() {
        let project_dir = tempdir().unwrap();
        let tmp_dir = tempdir().unwrap();

        let env_path = project_dir.path().join(".env");
        fs::write(&env_path, "OLD=content\n").unwrap();

        activate(project_dir.path(), ".env", "NEW=content\n", tmp_dir.path()).unwrap();

        let backup_path = project_dir.path().join(".env.bak");
        assert!(backup_path.is_file());
        assert_eq!(fs::read_to_string(&backup_path).unwrap(), "OLD=content\n");
    }

    #[test]
    fn activate_replaces_existing_envly_symlink() {
        let project_dir = tempdir().unwrap();
        let tmp_dir = tempdir().unwrap();

        activate(project_dir.path(), ".env", "V1=1\n", tmp_dir.path()).unwrap();
        activate(project_dir.path(), ".env", "V2=2\n", tmp_dir.path()).unwrap();

        let content = fs::read_to_string(project_dir.path().join(".env")).unwrap();
        assert_eq!(content, "V2=2\n");
    }

    #[test]
    fn detect_existing_variants() {
        let project_dir = tempdir().unwrap();
        let tmp_dir = tempdir().unwrap();

        let env_path = project_dir.path().join(".env");

        // None
        assert_eq!(
            detect_existing(&env_path, tmp_dir.path()),
            ExistingFile::None
        );

        // RegularFile
        fs::write(&env_path, "content").unwrap();
        assert_eq!(
            detect_existing(&env_path, tmp_dir.path()),
            ExistingFile::RegularFile
        );
        fs::remove_file(&env_path).unwrap();

        // EnvlySymlink
        let target = tmp_dir.path().join("env-test");
        fs::write(&target, "data").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &env_path).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&target, &env_path).unwrap();
        assert_eq!(
            detect_existing(&env_path, tmp_dir.path()),
            ExistingFile::EnvlySymlink
        );
        fs::remove_file(&env_path).unwrap();

        // ForeignSymlink
        let foreign_target = project_dir.path().join("other_file");
        fs::write(&foreign_target, "x").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&foreign_target, &env_path).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&foreign_target, &env_path).unwrap();
        assert_eq!(
            detect_existing(&env_path, tmp_dir.path()),
            ExistingFile::ForeignSymlink
        );
    }

    #[test]
    fn manifest_tracks_entries() {
        let project_dir = tempdir().unwrap();
        let tmp_dir = tempdir().unwrap();

        activate(project_dir.path(), ".env", "K=V\n", tmp_dir.path()).unwrap();

        let manifest = load_manifest(tmp_dir.path());
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0].pid, std::process::id());

        deactivate(project_dir.path(), ".env", tmp_dir.path()).unwrap();

        let manifest = load_manifest(tmp_dir.path());
        assert!(manifest.entries.is_empty());
    }

    #[test]
    fn can_create_symlinks_returns_true_on_unix() {
        #[cfg(unix)]
        assert!(can_create_symlinks());
    }

    // --- New symlink tests ---

    #[test]
    fn cleanup_all_removes_everything() {
        let project_dir = tempdir().unwrap();
        let tmp_dir = tempdir().unwrap();

        let temp1 = activate(project_dir.path(), ".env", "K1=V1\n", tmp_dir.path()).unwrap();
        assert!(temp1.is_file());

        cleanup_all(tmp_dir.path()).unwrap();

        assert!(!temp1.exists());
        assert!(!project_dir.path().join(".env").exists());

        let manifest = load_manifest(tmp_dir.path());
        assert!(manifest.entries.is_empty());
    }

    #[test]
    fn cleanup_stale_keeps_fresh_entries() {
        let project_dir = tempdir().unwrap();
        let tmp_dir = tempdir().unwrap();

        let temp_path = activate(project_dir.path(), ".env", "K=V\n", tmp_dir.path()).unwrap();
        assert!(temp_path.is_file());

        // Current PID is alive, so cleanup_stale should keep the entry
        cleanup_stale(tmp_dir.path()).unwrap();

        assert!(temp_path.is_file());
        let manifest = load_manifest(tmp_dir.path());
        assert_eq!(manifest.entries.len(), 1);
    }

    #[test]
    fn double_deactivate_is_noop() {
        let project_dir = tempdir().unwrap();
        let tmp_dir = tempdir().unwrap();

        activate(project_dir.path(), ".env", "K=V\n", tmp_dir.path()).unwrap();
        deactivate(project_dir.path(), ".env", tmp_dir.path()).unwrap();

        // Second deactivate should succeed without error
        deactivate(project_dir.path(), ".env", tmp_dir.path()).unwrap();
    }

    #[test]
    fn activate_updates_content() {
        let project_dir = tempdir().unwrap();
        let tmp_dir = tempdir().unwrap();

        activate(project_dir.path(), ".env", "OLD=1\n", tmp_dir.path()).unwrap();
        activate(project_dir.path(), ".env", "NEW=2\n", tmp_dir.path()).unwrap();

        let content = fs::read_to_string(project_dir.path().join(".env")).unwrap();
        assert_eq!(content, "NEW=2\n");
    }

    #[test]
    fn deactivate_nonexistent_is_noop() {
        let project_dir = tempdir().unwrap();
        let tmp_dir = tempdir().unwrap();

        // No activation happened; deactivate should succeed
        deactivate(project_dir.path(), ".env", tmp_dir.path()).unwrap();
        assert!(!project_dir.path().join(".env").exists());
    }

    #[test]
    fn manifest_round_trip() {
        let tmp_dir = tempdir().unwrap();
        let project_dir = tempdir().unwrap();

        activate(project_dir.path(), ".env", "K=V\n", tmp_dir.path()).unwrap();

        let m1 = load_manifest(tmp_dir.path());
        assert_eq!(m1.entries.len(), 1);

        // Re-load from disk and verify
        let m2 = load_manifest(tmp_dir.path());
        assert_eq!(m2.entries.len(), 1);
        assert_eq!(m1.entries[0].id, m2.entries[0].id);
    }
}
