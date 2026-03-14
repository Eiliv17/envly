use std::collections::HashSet;
use std::fs;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::core::env::{resolver, symlink};
use crate::core::vault::models::Secret;
use crate::state::AppState;

type CmdResult<T> = std::result::Result<T, String>;

fn map_err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Refresh all active environment symlinks that reference any of the given secret IDs.
fn refresh_active_symlinks_for_secrets(
    state: &AppState,
    secret_ids: &HashSet<Uuid>,
) -> CmdResult<()> {
    if secret_ids.is_empty() {
        return Ok(());
    }

    let pending: Vec<(String, String, String)>;
    {
        let vault_guard = state.vault().read().unwrap();
        let vault = vault_guard.as_ref().ok_or("Vault is locked")?;

        pending = vault
            .projects
            .iter()
            .flat_map(|project| {
                project.environments.iter().filter_map(move |env| {
                    if !env.is_active {
                        return None;
                    }
                    if !env
                        .mappings
                        .iter()
                        .any(|m| secret_ids.contains(&m.secret_id))
                    {
                        return None;
                    }
                    let resolved = resolver::resolve_environment(vault, project.id, env.id).ok()?;
                    let content = resolver::format_env_file(&resolved);
                    Some((
                        project.folder_path.clone(),
                        project.env_filename.clone(),
                        content,
                    ))
                })
            })
            .collect();
    }

    for (folder_path, env_filename, content) in &pending {
        symlink::activate(
            std::path::Path::new(folder_path),
            env_filename,
            content,
            &state.tmp_dir,
        )
        .map_err(map_err)?;
    }
    Ok(())
}

/// Secret response with value stripped for list operations.
#[derive(Serialize)]
pub struct SecretSummary {
    pub id: Uuid,
    pub key: String,
    pub description: String,
    pub expires_at: Option<NaiveDate>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&Secret> for SecretSummary {
    fn from(s: &Secret) -> Self {
        Self {
            id: s.id,
            key: s.key.clone(),
            description: s.description.clone(),
            expires_at: s.expires_at,
            tags: s.tags.clone(),
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

#[tauri::command]
pub fn list_secrets(state: State<'_, AppState>) -> CmdResult<Vec<SecretSummary>> {
    state.require_unlocked().map_err(map_err)?;
    let vault_guard = state.vault().read().unwrap();
    let vault = vault_guard.as_ref().ok_or("Vault is locked")?;
    Ok(vault.secrets.iter().map(SecretSummary::from).collect())
}

#[tauri::command]
pub fn reveal_secret_value(state: State<'_, AppState>, id: Uuid) -> CmdResult<String> {
    state.require_unlocked().map_err(map_err)?;
    let vault_guard = state.vault().read().unwrap();
    let vault = vault_guard.as_ref().ok_or("Vault is locked")?;
    let secret = vault
        .find_secret(id)
        .ok_or_else(|| format!("Secret with id '{id}' not found"))?;
    Ok(secret.value.to_string())
}

#[derive(Deserialize)]
pub struct CreateSecretArgs {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub expires_at: Option<NaiveDate>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[tauri::command]
pub fn create_secret(
    app: AppHandle,
    state: State<'_, AppState>,
    args: CreateSecretArgs,
) -> CmdResult<Uuid> {
    state.require_unlocked().map_err(map_err)?;
    let mut secret = Secret::new(args.key, args.value);
    secret.description = args.description;
    secret.expires_at = args.expires_at;
    secret.tags = args.tags;
    let id = secret.id;

    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;
        vault.add_secret(secret).map_err(map_err)?;
    }
    state.mark_dirty();
    let _ = app.emit("vault:secrets-changed", ());
    Ok(id)
}

#[derive(Deserialize)]
pub struct UpdateSecretArgs {
    pub id: Uuid,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub expires_at: Option<Option<NaiveDate>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[tauri::command]
pub fn update_secret(
    app: AppHandle,
    state: State<'_, AppState>,
    args: UpdateSecretArgs,
) -> CmdResult<()> {
    state.require_unlocked().map_err(map_err)?;

    let value_changed;
    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;

        if let Some(ref key) = args.key {
            vault
                .ensure_secret_key_unique(key, args.id)
                .map_err(map_err)?;
        }

        let secret = vault
            .find_secret_mut(args.id)
            .ok_or_else(|| format!("Secret with id '{}' not found", args.id))?;

        if let Some(key) = args.key {
            secret.key = key;
        }
        value_changed = args.value.is_some();
        if let Some(value) = args.value {
            secret.value = crate::core::vault::models::Zeroizing::new(value);
        }
        if let Some(desc) = args.description {
            secret.description = desc;
        }
        if let Some(exp) = args.expires_at {
            secret.expires_at = exp;
        }
        if let Some(tags) = args.tags {
            secret.tags = tags;
        }
        secret.updated_at = chrono::Utc::now();
    }
    state.mark_dirty();
    if value_changed {
        refresh_active_symlinks_for_secrets(&state, &HashSet::from([args.id]))?;
    }
    let _ = app.emit("vault:secrets-changed", ());
    Ok(())
}

#[tauri::command]
pub fn delete_secret(app: AppHandle, state: State<'_, AppState>, id: Uuid) -> CmdResult<()> {
    state.require_unlocked().map_err(map_err)?;

    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;
        vault.remove_secret(id).map_err(map_err)?;
    }
    state.mark_dirty();
    let _ = app.emit("vault:secrets-changed", ());
    Ok(())
}

// --- Bulk import ---

#[derive(Serialize, Clone)]
pub struct ParsedEnvEntry {
    pub key: String,
    pub value: String,
}

fn parse_env_text(text: &str) -> Vec<ParsedEnvEntry> {
    let mut entries = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let line_body = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
        if let Some(eq_pos) = line_body.find('=') {
            let key = line_body[..eq_pos].trim().to_string();
            let raw_value = line_body[eq_pos + 1..].trim();
            let value =
                if raw_value.starts_with('"') && raw_value.ends_with('"') && raw_value.len() >= 2 {
                    let inner = &raw_value[1..raw_value.len() - 1];
                    inner
                        .replace("\\\"", "\"")
                        .replace("\\n", "\n")
                        .replace("\\r", "\r")
                        .replace("\\\\", "\\")
                } else if raw_value.starts_with('\'')
                    && raw_value.ends_with('\'')
                    && raw_value.len() >= 2
                {
                    raw_value[1..raw_value.len() - 1].to_string()
                } else {
                    // Strip inline comments for unquoted values
                    let without_comment = if let Some(hash_pos) = raw_value.find(" #") {
                        raw_value[..hash_pos].trim_end()
                    } else {
                        raw_value
                    };
                    without_comment.to_string()
                };
            if !key.is_empty() {
                entries.push(ParsedEnvEntry { key, value });
            }
        }
    }
    entries
}

#[tauri::command]
pub fn parse_env_file(path: String) -> CmdResult<Vec<ParsedEnvEntry>> {
    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {e}"))?;
    Ok(parse_env_text(&content))
}

#[derive(Deserialize)]
pub struct BulkSecretEntry {
    pub key: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct BulkCreateResult {
    pub created: usize,
    pub updated: usize,
}

#[tauri::command]
pub fn bulk_create_secrets(
    app: AppHandle,
    state: State<'_, AppState>,
    entries: Vec<BulkSecretEntry>,
) -> CmdResult<BulkCreateResult> {
    state.require_unlocked().map_err(map_err)?;

    let mut created = 0usize;
    let mut updated = 0usize;
    let mut updated_secret_ids = HashSet::new();

    {
        let mut vault_guard = state.vault().write().unwrap();
        let vault = vault_guard.as_mut().ok_or("Vault is locked")?;

        for entry in entries {
            if let Some(existing) = vault.secrets.iter_mut().find(|s| s.key == entry.key) {
                existing.value = crate::core::vault::models::Zeroizing::new(entry.value);
                existing.updated_at = chrono::Utc::now();
                updated_secret_ids.insert(existing.id);
                updated += 1;
            } else {
                let secret = Secret::new(entry.key, entry.value);
                vault.secrets.push(secret);
                created += 1;
            }
        }
    }

    if created > 0 || updated > 0 {
        state.mark_dirty();
        let _ = app.emit("vault:secrets-changed", ());
    }
    refresh_active_symlinks_for_secrets(&state, &updated_secret_ids)?;
    Ok(BulkCreateResult { created, updated })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_key_value() {
        let entries = parse_env_text("DB_HOST=localhost\nDB_PORT=5432");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "DB_HOST");
        assert_eq!(entries[0].value, "localhost");
        assert_eq!(entries[1].key, "DB_PORT");
        assert_eq!(entries[1].value, "5432");
    }

    #[test]
    fn parse_double_quoted_value() {
        let entries = parse_env_text("KEY=\"value with spaces\"");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, "value with spaces");
    }

    #[test]
    fn parse_single_quoted_value() {
        let entries = parse_env_text("KEY='single quoted'");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, "single quoted");
    }

    #[test]
    fn parse_export_prefix() {
        let entries = parse_env_text("export API_KEY=secret123");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "API_KEY");
        assert_eq!(entries[0].value, "secret123");
    }

    #[test]
    fn parse_skips_comments() {
        let text = "# This is a comment\nKEY=value\n# Another comment";
        let entries = parse_env_text(text);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "KEY");
    }

    #[test]
    fn parse_skips_empty_lines() {
        let text = "\n\n  \nKEY=value\n\n";
        let entries = parse_env_text(text);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn parse_equals_in_value() {
        let entries = parse_env_text("URL=postgres://host?opt=1&b=2");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "URL");
        assert_eq!(entries[0].value, "postgres://host?opt=1&b=2");
    }

    #[test]
    fn parse_empty_value() {
        let entries = parse_env_text("EMPTY=");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "EMPTY");
        assert_eq!(entries[0].value, "");
    }

    #[test]
    fn parse_inline_comment_stripped() {
        let entries = parse_env_text("KEY=value # this is a comment");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, "value");
    }

    #[test]
    fn parse_escaped_quotes_in_double_quoted() {
        let entries = parse_env_text(r#"KEY="say \"hello\"""#);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, "say \"hello\"");
    }

    #[test]
    fn parse_escaped_newline_in_double_quoted() {
        let entries = parse_env_text(r#"KEY="line1\nline2""#);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, "line1\nline2");
    }

    #[test]
    fn parse_no_value_line_skipped() {
        let entries = parse_env_text("NOT_AN_ENV_LINE");
        assert_eq!(entries.len(), 0);
    }
}
