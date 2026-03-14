use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::core::vault::models::{EnvMapping, Environment, Project};
use crate::state::AppState;

type CmdResult<T> = std::result::Result<T, String>;

fn map_err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize)]
pub struct ActiveEnvInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Serialize)]
pub struct ProjectSummary {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub folder_path: String,
    pub env_filename: String,
    pub environment_count: usize,
    pub starred: bool,
    pub active_env: Option<ActiveEnvInfo>,
    pub path_valid: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&Project> for ProjectSummary {
    fn from(p: &Project) -> Self {
        let active_env = p
            .environments
            .iter()
            .find(|e| e.is_active)
            .map(|e| ActiveEnvInfo {
                id: e.id,
                name: e.name.clone(),
            });
        Self {
            id: p.id,
            name: p.name.clone(),
            description: p.description.clone(),
            folder_path: p.folder_path.clone(),
            env_filename: p.env_filename.clone(),
            environment_count: p.environments.len(),
            starred: p.starred,
            active_env,
            path_valid: std::path::Path::new(&p.folder_path).is_dir(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> CmdResult<Vec<ProjectSummary>> {
    state.require_unlocked().map_err(map_err)?;
    let vault_guard = state.vault().read().unwrap();
    let vault = vault_guard.as_ref().ok_or("Vault is locked")?;
    Ok(vault.projects.iter().map(ProjectSummary::from).collect())
}

#[derive(Deserialize)]
pub struct CreateProjectArgs {
    pub name: String,
    pub folder_path: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_env_filename")]
    pub env_filename: String,
}

fn default_env_filename() -> String {
    ".env".to_string()
}

#[tauri::command]
pub fn create_project(
    app: AppHandle,
    state: State<'_, AppState>,
    args: CreateProjectArgs,
) -> CmdResult<Uuid> {
    state.require_unlocked().map_err(map_err)?;

    let mut project = Project::new(args.name, args.folder_path);
    project.description = args.description;
    if !args.env_filename.is_empty() {
        project.env_filename = args.env_filename;
    }
    let id = project.id;

    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;
        vault.add_project(project).map_err(map_err)?;
    }
    state.mark_dirty();
    let _ = app.emit("vault:projects-changed", ());
    Ok(id)
}

#[derive(Deserialize)]
pub struct UpdateProjectArgs {
    pub id: Uuid,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub folder_path: Option<String>,
    #[serde(default)]
    pub env_filename: Option<String>,
}

#[tauri::command]
pub fn update_project(
    app: AppHandle,
    state: State<'_, AppState>,
    args: UpdateProjectArgs,
) -> CmdResult<()> {
    state.require_unlocked().map_err(map_err)?;

    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;

        if let Some(ref name) = args.name {
            vault
                .ensure_project_name_unique(name, args.id)
                .map_err(map_err)?;
        }

        let project = vault
            .find_project_mut(args.id)
            .ok_or_else(|| format!("Project with id '{}' not found", args.id))?;

        if let Some(name) = args.name {
            project.name = name;
        }
        if let Some(desc) = args.description {
            project.description = desc;
        }
        if let Some(path) = args.folder_path {
            project.folder_path = path;
        }
        if let Some(filename) = args.env_filename {
            project.env_filename = filename;
        }
        project.updated_at = chrono::Utc::now();
    }
    state.mark_dirty();
    let _ = app.emit("vault:projects-changed", ());
    Ok(())
}

#[tauri::command]
pub fn delete_project(app: AppHandle, state: State<'_, AppState>, id: Uuid) -> CmdResult<()> {
    state.require_unlocked().map_err(map_err)?;

    {
        let vault_guard = state.vault().read().unwrap();
        let vault = vault_guard.as_ref().ok_or("Vault is locked")?;
        if let Some(project) = vault.find_project(id) {
            let has_active = project.environments.iter().any(|e| e.is_active);
            if has_active {
                let folder_path = project.folder_path.clone();
                let env_filename = project.env_filename.clone();
                drop(vault_guard);
                let _ = crate::core::env::symlink::deactivate(
                    std::path::Path::new(&folder_path),
                    &env_filename,
                    &state.tmp_dir,
                );
            }
        }
    }

    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;
        vault.remove_project(id).map_err(map_err)?;
    }
    state.mark_dirty();
    let _ = app.emit("vault:projects-changed", ());
    Ok(())
}

#[tauri::command]
pub fn validate_project_path(path: String) -> bool {
    std::path::Path::new(&path).is_dir()
}

#[tauri::command]
pub fn toggle_project_starred(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Uuid,
) -> CmdResult<bool> {
    state.require_unlocked().map_err(map_err)?;

    let new_starred;
    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;
        let project = vault
            .find_project_mut(id)
            .ok_or_else(|| format!("Project with id '{}' not found", id))?;
        project.starred = !project.starred;
        new_starred = project.starred;
    }
    state.mark_dirty();
    let _ = app.emit("vault:projects-changed", ());
    Ok(new_starred)
}

#[derive(Deserialize)]
pub struct CloneProjectArgs {
    pub source_project_id: Uuid,
    pub new_name: String,
    pub new_folder_path: String,
}

#[tauri::command]
pub fn clone_project(
    app: AppHandle,
    state: State<'_, AppState>,
    args: CloneProjectArgs,
) -> CmdResult<Uuid> {
    state.require_unlocked().map_err(map_err)?;

    let new_project_id;
    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;

        let source = vault
            .find_project(args.source_project_id)
            .ok_or_else(|| format!("Source project not found"))?;

        let cloned_envs: Vec<Environment> = source
            .environments
            .iter()
            .map(|env| {
                let new_mappings: Vec<EnvMapping> = env
                    .mappings
                    .iter()
                    .map(|m| {
                        let mut cloned = EnvMapping::new(m.local_key.clone(), m.secret_id);
                        cloned.notes = m.notes.clone();
                        cloned
                    })
                    .collect();
                let mut new_env = Environment::new(env.name.clone());
                new_env.description = env.description.clone();
                new_env.mappings = new_mappings;
                new_env
            })
            .collect();

        let mut new_project = Project::new(args.new_name, args.new_folder_path);
        new_project.description = source.description.clone();
        new_project.env_filename = source.env_filename.clone();
        new_project.environments = cloned_envs;
        new_project_id = new_project.id;

        vault.add_project(new_project).map_err(map_err)?;
    }
    state.mark_dirty();
    let _ = app.emit("vault:projects-changed", ());
    Ok(new_project_id)
}
