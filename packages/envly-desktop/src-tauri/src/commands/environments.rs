use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::core::env::{resolver, symlink};
use crate::core::vault::models::{EnvMapping, Environment};
use crate::state::AppState;

type CmdResult<T> = std::result::Result<T, String>;

fn map_err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// If the given environment is active, re-resolve its mappings and rewrite the symlink.
fn refresh_active_symlink(state: &AppState, project_id: Uuid, env_id: Uuid) -> CmdResult<()> {
    let vault_guard = state.vault().read().unwrap();
    let vault = vault_guard.as_ref().ok_or("Vault is locked")?;
    let project = vault.find_project(project_id).ok_or("Project not found")?;
    let is_active = project
        .environments
        .iter()
        .any(|e| e.id == env_id && e.is_active);
    if !is_active {
        return Ok(());
    }
    let resolved = resolver::resolve_environment(vault, project_id, env_id).map_err(map_err)?;
    let content = resolver::format_env_file(&resolved);
    let folder_path = project.folder_path.clone();
    let env_filename = project.env_filename.clone();
    drop(vault_guard);

    symlink::activate(
        std::path::Path::new(&folder_path),
        &env_filename,
        &content,
        &state.tmp_dir,
    )
    .map_err(map_err)?;
    Ok(())
}

#[derive(Serialize)]
pub struct EnvironmentSummary {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub is_active: bool,
    pub mapping_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&Environment> for EnvironmentSummary {
    fn from(e: &Environment) -> Self {
        Self {
            id: e.id,
            name: e.name.clone(),
            description: e.description.clone(),
            is_active: e.is_active,
            mapping_count: e.mappings.len(),
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

#[derive(Serialize)]
pub struct MappingSummary {
    pub id: Uuid,
    pub local_key: String,
    pub secret_id: Uuid,
    pub secret_key: String,
    pub notes: String,
}

#[tauri::command]
pub fn list_environments(
    state: State<'_, AppState>,
    project_id: Uuid,
) -> CmdResult<Vec<EnvironmentSummary>> {
    state.require_unlocked().map_err(map_err)?;
    let vault_guard = state.vault().read().unwrap();
    let vault = vault_guard.as_ref().ok_or("Vault is locked")?;
    let project = vault
        .find_project(project_id)
        .ok_or_else(|| format!("Project with id '{project_id}' not found"))?;
    Ok(project
        .environments
        .iter()
        .map(EnvironmentSummary::from)
        .collect())
}

#[tauri::command]
pub fn list_mappings(
    state: State<'_, AppState>,
    project_id: Uuid,
    env_id: Uuid,
) -> CmdResult<Vec<MappingSummary>> {
    state.require_unlocked().map_err(map_err)?;
    let vault_guard = state.vault().read().unwrap();
    let vault = vault_guard.as_ref().ok_or("Vault is locked")?;
    let project = vault
        .find_project(project_id)
        .ok_or_else(|| format!("Project with id '{project_id}' not found"))?;
    let env = project
        .environments
        .iter()
        .find(|e| e.id == env_id)
        .ok_or_else(|| format!("Environment with id '{env_id}' not found"))?;

    let summaries = env
        .mappings
        .iter()
        .map(|m| {
            let secret_key = vault
                .find_secret(m.secret_id)
                .map(|s| s.key.clone())
                .unwrap_or_else(|| "[deleted]".into());
            MappingSummary {
                id: m.id,
                local_key: m.local_key.clone(),
                secret_id: m.secret_id,
                secret_key,
                notes: m.notes.clone(),
            }
        })
        .collect();
    Ok(summaries)
}

#[derive(Deserialize)]
pub struct CreateEnvironmentArgs {
    pub project_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[tauri::command]
pub fn create_environment(
    app: AppHandle,
    state: State<'_, AppState>,
    args: CreateEnvironmentArgs,
) -> CmdResult<Uuid> {
    state.require_unlocked().map_err(map_err)?;

    let mut env = Environment::new(args.name);
    env.description = args.description;
    let id = env.id;

    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;
        vault
            .add_environment(args.project_id, env)
            .map_err(map_err)?;
    }
    state.mark_dirty();
    let _ = app.emit("vault:environments-changed", ());
    Ok(id)
}

#[derive(Deserialize)]
pub struct UpdateEnvironmentArgs {
    pub project_id: Uuid,
    pub env_id: Uuid,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[tauri::command]
pub fn update_environment(
    app: AppHandle,
    state: State<'_, AppState>,
    args: UpdateEnvironmentArgs,
) -> CmdResult<()> {
    state.require_unlocked().map_err(map_err)?;

    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;

        if let Some(ref name) = args.name {
            vault
                .ensure_env_name_unique(args.project_id, name, args.env_id)
                .map_err(map_err)?;
        }

        let project = vault
            .find_project_mut(args.project_id)
            .ok_or_else(|| format!("Project not found"))?;
        let env = project
            .environments
            .iter_mut()
            .find(|e| e.id == args.env_id)
            .ok_or_else(|| format!("Environment not found"))?;

        if let Some(name) = args.name {
            env.name = name;
        }
        if let Some(desc) = args.description {
            env.description = desc;
        }
        env.updated_at = chrono::Utc::now();
    }
    state.mark_dirty();
    let _ = app.emit("vault:environments-changed", ());
    Ok(())
}

#[tauri::command]
pub fn delete_environment(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: Uuid,
    env_id: Uuid,
) -> CmdResult<()> {
    state.require_unlocked().map_err(map_err)?;

    let cleanup_info: Option<(String, String)> = {
        let vault_guard = state.vault().read().unwrap();
        let vault = vault_guard.as_ref().ok_or("Vault is locked")?;
        let project = vault
            .find_project(project_id)
            .ok_or_else(|| format!("Project with id '{project_id}' not found"))?;
        let env = project.environments.iter().find(|e| e.id == env_id);
        match env {
            Some(e) if e.is_active => {
                Some((project.folder_path.clone(), project.env_filename.clone()))
            }
            _ => None,
        }
    };

    if let Some((folder_path, env_filename)) = cleanup_info {
        let _ = symlink::deactivate(
            std::path::Path::new(&folder_path),
            &env_filename,
            &state.tmp_dir,
        );
    }

    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;
        vault
            .remove_environment(project_id, env_id)
            .map_err(map_err)?;
    }
    state.mark_dirty();
    let _ = app.emit("vault:environments-changed", ());
    Ok(())
}

#[tauri::command]
pub fn activate_environment(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: Uuid,
    env_id: Uuid,
) -> CmdResult<()> {
    state.require_unlocked().map_err(map_err)?;

    let (project_path, env_filename, env_content) = {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;
        vault
            .activate_environment(project_id, env_id)
            .map_err(map_err)?;

        let resolved = resolver::resolve_environment(vault, project_id, env_id).map_err(map_err)?;
        let content = resolver::format_env_file(&resolved);

        let project = vault.find_project(project_id).ok_or("Project not found")?;
        (
            project.folder_path.clone(),
            project.env_filename.clone(),
            content,
        )
    };

    symlink::activate(
        std::path::Path::new(&project_path),
        &env_filename,
        &env_content,
        &state.tmp_dir,
    )
    .map_err(map_err)?;

    state.mark_dirty();
    let _ = app.emit("vault:environments-changed", ());
    Ok(())
}

#[tauri::command]
pub fn deactivate_environment(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: Uuid,
    env_id: Uuid,
) -> CmdResult<()> {
    state.require_unlocked().map_err(map_err)?;

    let (project_path, env_filename) = {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;
        vault
            .deactivate_environment(project_id, env_id)
            .map_err(map_err)?;

        let project = vault.find_project(project_id).ok_or("Project not found")?;
        (project.folder_path.clone(), project.env_filename.clone())
    };

    symlink::deactivate(
        std::path::Path::new(&project_path),
        &env_filename,
        &state.tmp_dir,
    )
    .map_err(map_err)?;

    state.mark_dirty();
    let _ = app.emit("vault:environments-changed", ());
    Ok(())
}

#[derive(Deserialize)]
pub struct AddMappingArgs {
    pub project_id: Uuid,
    pub env_id: Uuid,
    pub local_key: String,
    pub secret_id: Uuid,
    #[serde(default)]
    pub notes: String,
}

#[tauri::command]
pub fn add_mapping(
    app: AppHandle,
    state: State<'_, AppState>,
    args: AddMappingArgs,
) -> CmdResult<Uuid> {
    state.require_unlocked().map_err(map_err)?;

    let mut mapping = EnvMapping::new(args.local_key, args.secret_id);
    mapping.notes = args.notes;
    let id = mapping.id;

    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;
        vault
            .add_mapping(args.project_id, args.env_id, mapping)
            .map_err(map_err)?;
    }
    refresh_active_symlink(&state, args.project_id, args.env_id)?;
    state.mark_dirty();
    let _ = app.emit("vault:environments-changed", ());
    Ok(id)
}

#[tauri::command]
pub fn remove_mapping(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: Uuid,
    env_id: Uuid,
    mapping_id: Uuid,
) -> CmdResult<()> {
    state.require_unlocked().map_err(map_err)?;

    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;
        vault
            .remove_mapping(project_id, env_id, mapping_id)
            .map_err(map_err)?;
    }
    refresh_active_symlink(&state, project_id, env_id)?;
    state.mark_dirty();
    let _ = app.emit("vault:environments-changed", ());
    Ok(())
}

#[derive(Deserialize)]
pub struct CloneEnvironmentArgs {
    pub source_project_id: Uuid,
    pub source_env_id: Uuid,
    pub target_project_id: Uuid,
    pub new_name: String,
}

#[tauri::command]
pub fn clone_environment(
    app: AppHandle,
    state: State<'_, AppState>,
    args: CloneEnvironmentArgs,
) -> CmdResult<Uuid> {
    state.require_unlocked().map_err(map_err)?;

    let new_env_id;
    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;

        let source_project = vault
            .find_project(args.source_project_id)
            .ok_or_else(|| format!("Source project not found"))?;
        let source_env = source_project
            .environments
            .iter()
            .find(|e| e.id == args.source_env_id)
            .ok_or_else(|| format!("Source environment not found"))?;

        let now = chrono::Utc::now();
        let new_mappings: Vec<EnvMapping> = source_env
            .mappings
            .iter()
            .map(|m| {
                let mut cloned = EnvMapping::new(m.local_key.clone(), m.secret_id);
                cloned.notes = m.notes.clone();
                cloned
            })
            .collect();

        let mut new_env = Environment::new(args.new_name);
        new_env.description = source_env.description.clone();
        new_env.mappings = new_mappings;
        new_env.created_at = now;
        new_env.updated_at = now;
        new_env_id = new_env.id;

        vault
            .add_environment(args.target_project_id, new_env)
            .map_err(map_err)?;
    }
    state.mark_dirty();
    let _ = app.emit("vault:environments-changed", ());
    Ok(new_env_id)
}
