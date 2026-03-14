use uuid::Uuid;

use crate::core::error::{EnvlyError, Result};
use crate::core::vault::models::Vault;

/// Resolve all env mappings for a given environment into concrete key=value pairs.
/// Returns a Vec of (local_key, secret_value) tuples.
pub fn resolve_environment(
    vault: &Vault,
    project_id: Uuid,
    env_id: Uuid,
) -> Result<Vec<(String, String)>> {
    let project = vault.find_project(project_id).ok_or_else(|| {
        EnvlyError::Validation(format!("Project with id '{project_id}' not found"))
    })?;

    let env = project
        .environments
        .iter()
        .find(|e| e.id == env_id)
        .ok_or_else(|| {
            EnvlyError::Validation(format!("Environment with id '{env_id}' not found"))
        })?;

    let mut resolved = Vec::with_capacity(env.mappings.len());

    for mapping in &env.mappings {
        let secret = vault.find_secret(mapping.secret_id).ok_or_else(|| {
            EnvlyError::Validation(format!(
                "Secret with id '{}' not found (referenced by mapping '{}')",
                mapping.secret_id, mapping.local_key
            ))
        })?;
        resolved.push((mapping.local_key.clone(), secret.value.to_string()));
    }

    Ok(resolved)
}

/// Format resolved key=value pairs as .env file content.
pub fn format_env_file(resolved: &[(String, String)]) -> String {
    let mut output = String::new();
    for (key, value) in resolved {
        if needs_quoting(value) {
            let escaped = value
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r");
            output.push_str(&format!("{key}=\"{escaped}\"\n"));
        } else {
            output.push_str(&format!("{key}={value}\n"));
        }
    }
    output
}

fn needs_quoting(value: &str) -> bool {
    value.is_empty()
        || value.contains(' ')
        || value.contains('\n')
        || value.contains('\r')
        || value.contains('#')
        || value.contains('"')
        || value.contains('\'')
        || value.contains('\\')
        || value.contains('=')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::vault::models::*;

    fn setup_vault() -> (Vault, Uuid, Uuid, Uuid) {
        let mut vault = Vault::new("symmetric", 1);

        let secret = Secret::new("PG_PASS".into(), "s3cret".into());
        let secret_id = secret.id;
        vault.add_secret(secret).unwrap();

        let project = Project::new("api".into(), "/tmp/api".into());
        let project_id = project.id;
        vault.add_project(project).unwrap();

        let env = Environment::new("dev".into());
        let env_id = env.id;
        vault.add_environment(project_id, env).unwrap();

        vault
            .add_mapping(
                project_id,
                env_id,
                EnvMapping::new("DATABASE_PASSWORD".into(), secret_id),
            )
            .unwrap();

        (vault, project_id, env_id, secret_id)
    }

    #[test]
    fn resolve_valid_environment() {
        let (vault, project_id, env_id, _) = setup_vault();
        let resolved = resolve_environment(&vault, project_id, env_id).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].0, "DATABASE_PASSWORD");
        assert_eq!(resolved[0].1, "s3cret");
    }

    #[test]
    fn resolve_empty_environment() {
        let mut vault = Vault::new("symmetric", 1);
        let project = Project::new("api".into(), "/tmp/api".into());
        let pid = project.id;
        vault.add_project(project).unwrap();
        let env = Environment::new("empty".into());
        let eid = env.id;
        vault.add_environment(pid, env).unwrap();

        let resolved = resolve_environment(&vault, pid, eid).unwrap();
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolve_missing_project_fails() {
        let vault = Vault::new("symmetric", 1);
        let result = resolve_environment(&vault, Uuid::new_v4(), Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn resolve_missing_environment_fails() {
        let mut vault = Vault::new("symmetric", 1);
        let project = Project::new("api".into(), "/tmp/api".into());
        let pid = project.id;
        vault.add_project(project).unwrap();

        let result = resolve_environment(&vault, pid, Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn format_simple_values() {
        let resolved = vec![
            ("KEY1".into(), "value1".into()),
            ("KEY2".into(), "value2".into()),
        ];
        let output = format_env_file(&resolved);
        assert_eq!(output, "KEY1=value1\nKEY2=value2\n");
    }

    #[test]
    fn format_quoted_values() {
        let resolved = vec![
            ("KEY".into(), "value with spaces".into()),
            ("HASH".into(), "before#after".into()),
        ];
        let output = format_env_file(&resolved);
        assert!(output.contains("KEY=\"value with spaces\""));
        assert!(output.contains("HASH=\"before#after\""));
    }

    // --- New resolver / format tests ---

    #[test]
    fn format_env_file_empty() {
        let output = format_env_file(&[]);
        assert_eq!(output, "");
    }

    #[test]
    fn format_env_file_newline_in_value() {
        let resolved = vec![("MULTI".into(), "line1\nline2".into())];
        let output = format_env_file(&resolved);
        assert_eq!(output, "MULTI=\"line1\\nline2\"\n");
    }

    #[test]
    fn format_env_file_backslash_in_value() {
        let resolved = vec![("PATH".into(), "C:\\Users\\me".into())];
        let output = format_env_file(&resolved);
        assert_eq!(output, "PATH=\"C:\\\\Users\\\\me\"\n");
    }

    #[test]
    fn format_env_file_equals_in_value() {
        let resolved = vec![("URL".into(), "postgres://host?opt=1".into())];
        let output = format_env_file(&resolved);
        assert_eq!(output, "URL=\"postgres://host?opt=1\"\n");
    }

    #[test]
    fn format_env_file_empty_value() {
        let resolved = vec![("EMPTY".into(), "".into())];
        let output = format_env_file(&resolved);
        assert_eq!(output, "EMPTY=\"\"\n");
    }

    #[test]
    fn format_env_file_double_quotes_in_value() {
        let resolved = vec![("MSG".into(), "say \"hello\"".into())];
        let output = format_env_file(&resolved);
        assert_eq!(output, "MSG=\"say \\\"hello\\\"\"\n");
    }

    #[test]
    fn format_env_file_carriage_return() {
        let resolved = vec![("CR".into(), "a\rb".into())];
        let output = format_env_file(&resolved);
        assert_eq!(output, "CR=\"a\\rb\"\n");
    }

    #[test]
    fn resolve_multiple_mappings_ordering() {
        let mut vault = Vault::new("symmetric", 1);

        let s1 = Secret::new("S_A".into(), "val_a".into());
        let s2 = Secret::new("S_B".into(), "val_b".into());
        let s3 = Secret::new("S_C".into(), "val_c".into());
        let sid1 = s1.id;
        let sid2 = s2.id;
        let sid3 = s3.id;
        vault.add_secret(s1).unwrap();
        vault.add_secret(s2).unwrap();
        vault.add_secret(s3).unwrap();

        let project = Project::new("api".into(), "/tmp/api".into());
        let pid = project.id;
        vault.add_project(project).unwrap();

        let env = Environment::new("dev".into());
        let eid = env.id;
        vault.add_environment(pid, env).unwrap();

        vault
            .add_mapping(pid, eid, EnvMapping::new("FIRST".into(), sid1))
            .unwrap();
        vault
            .add_mapping(pid, eid, EnvMapping::new("SECOND".into(), sid2))
            .unwrap();
        vault
            .add_mapping(pid, eid, EnvMapping::new("THIRD".into(), sid3))
            .unwrap();

        let resolved = resolve_environment(&vault, pid, eid).unwrap();
        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved[0].0, "FIRST");
        assert_eq!(resolved[1].0, "SECOND");
        assert_eq!(resolved[2].0, "THIRD");
    }

    #[test]
    fn resolve_with_deleted_secret_fails() {
        let mut vault = Vault::new("symmetric", 1);
        let secret = Secret::new("S1".into(), "v1".into());
        let sid = secret.id;
        vault.add_secret(secret).unwrap();

        let project = Project::new("api".into(), "/tmp/api".into());
        let pid = project.id;
        vault.add_project(project).unwrap();

        let env = Environment::new("dev".into());
        let eid = env.id;
        vault.add_environment(pid, env).unwrap();
        vault
            .add_mapping(pid, eid, EnvMapping::new("KEY".into(), sid))
            .unwrap();

        // Directly remove the secret from the vec to simulate a dangling reference
        vault.secrets.clear();

        let result = resolve_environment(&vault, pid, eid);
        assert!(result.is_err());
    }
}
