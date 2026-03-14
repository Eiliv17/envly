use chrono::NaiveDate;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;
use zeroize::Zeroize;

use crate::core::error::{EnvlyError, Result};

fn serialize_zeroizing<S>(
    val: &Zeroizing<String>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(val.as_str())
}

fn deserialize_zeroizing<'de, D>(
    deserializer: D,
) -> std::result::Result<Zeroizing<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(Zeroizing::new(s))
}

#[derive(Clone)]
pub struct Zeroizing<T: Zeroize>(T);

impl<T: Zeroize> Zeroizing<T> {
    pub fn new(val: T) -> Self {
        Self(val)
    }
}

impl<T: Zeroize> std::ops::Deref for Zeroizing<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Zeroize> Drop for Zeroizing<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl<T: Zeroize + PartialEq> PartialEq for Zeroizing<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Zeroize + std::fmt::Debug> std::fmt::Debug for Zeroizing<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vault {
    pub cipher_version: u32,
    pub cipher_kind: String,
    pub secrets: Vec<Secret>,
    pub projects: Vec<Project>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Secret {
    pub id: Uuid,
    pub key: String,
    #[serde(
        serialize_with = "serialize_zeroizing",
        deserialize_with = "deserialize_zeroizing"
    )]
    pub value: Zeroizing<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub expires_at: Option<NaiveDate>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Secret")
            .field("id", &self.id)
            .field("key", &self.key)
            .field("value", &"[REDACTED]")
            .field("description", &self.description)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub folder_path: String,
    #[serde(default = "default_env_filename")]
    pub env_filename: String,
    #[serde(default)]
    pub starred: bool,
    pub environments: Vec<Environment>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_env_filename() -> String {
    ".env".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub is_active: bool,
    pub mappings: Vec<EnvMapping>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvMapping {
    pub id: Uuid,
    pub local_key: String,
    pub secret_id: Uuid,
    #[serde(default)]
    pub notes: String,
}

// --- Vault CRUD helpers ---

impl Vault {
    pub fn new(cipher_kind: &str, cipher_version: u32) -> Self {
        Self {
            cipher_version,
            cipher_kind: cipher_kind.to_string(),
            secrets: Vec::new(),
            projects: Vec::new(),
        }
    }

    // -- Secrets --

    pub fn find_secret(&self, id: Uuid) -> Option<&Secret> {
        self.secrets.iter().find(|s| s.id == id)
    }

    pub fn find_secret_mut(&mut self, id: Uuid) -> Option<&mut Secret> {
        self.secrets.iter_mut().find(|s| s.id == id)
    }

    pub fn add_secret(&mut self, secret: Secret) -> Result<()> {
        Self::validate_secret_key(&secret.key)?;
        if self.secrets.iter().any(|s| s.key == secret.key) {
            return Err(EnvlyError::Validation(format!(
                "Secret with key '{}' already exists",
                secret.key
            )));
        }
        self.secrets.push(secret);
        Ok(())
    }

    pub fn validate_secret_key(key: &str) -> Result<()> {
        if key.trim().is_empty() {
            return Err(EnvlyError::Validation("Secret key cannot be empty".into()));
        }
        Ok(())
    }

    pub fn ensure_secret_key_unique(&self, key: &str, exclude_id: Uuid) -> Result<()> {
        if self
            .secrets
            .iter()
            .any(|s| s.key == key && s.id != exclude_id)
        {
            return Err(EnvlyError::Validation(format!(
                "Secret with key '{key}' already exists"
            )));
        }
        Ok(())
    }

    pub fn remove_secret(&mut self, id: Uuid) -> Result<()> {
        for project in &self.projects {
            for env in &project.environments {
                for mapping in &env.mappings {
                    if mapping.secret_id == id {
                        return Err(EnvlyError::Validation(format!(
                            "Cannot delete secret: referenced by mapping '{}' in environment '{}' of project '{}'",
                            mapping.local_key, env.name, project.name
                        )));
                    }
                }
            }
        }

        let len_before = self.secrets.len();
        self.secrets.retain(|s| s.id != id);
        if self.secrets.len() == len_before {
            return Err(EnvlyError::Validation(format!(
                "Secret with id '{id}' not found"
            )));
        }
        Ok(())
    }

    // -- Projects --

    pub fn find_project(&self, id: Uuid) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }

    pub fn find_project_mut(&mut self, id: Uuid) -> Option<&mut Project> {
        self.projects.iter_mut().find(|p| p.id == id)
    }

    pub fn add_project(&mut self, project: Project) -> Result<()> {
        Self::validate_name(&project.name, "Project")?;
        if self.projects.iter().any(|p| p.name == project.name) {
            return Err(EnvlyError::Validation(format!(
                "Project with name '{}' already exists",
                project.name
            )));
        }
        self.projects.push(project);
        Ok(())
    }

    pub fn ensure_project_name_unique(&self, name: &str, exclude_id: Uuid) -> Result<()> {
        if self
            .projects
            .iter()
            .any(|p| p.name == name && p.id != exclude_id)
        {
            return Err(EnvlyError::Validation(format!(
                "Project with name '{name}' already exists"
            )));
        }
        Ok(())
    }

    pub fn remove_project(&mut self, id: Uuid) -> Result<()> {
        let len_before = self.projects.len();
        self.projects.retain(|p| p.id != id);
        if self.projects.len() == len_before {
            return Err(EnvlyError::Validation(format!(
                "Project with id '{id}' not found"
            )));
        }
        Ok(())
    }

    // -- Environments (nested under project) --

    pub fn add_environment(&mut self, project_id: Uuid, env: Environment) -> Result<()> {
        Self::validate_name(&env.name, "Environment")?;
        let project = self.find_project_mut(project_id).ok_or_else(|| {
            EnvlyError::Validation(format!("Project with id '{project_id}' not found"))
        })?;
        if project.environments.iter().any(|e| e.name == env.name) {
            return Err(EnvlyError::Validation(format!(
                "Environment '{}' already exists in project '{}'",
                env.name, project.name
            )));
        }
        project.environments.push(env);
        Ok(())
    }

    pub fn ensure_env_name_unique(
        &self,
        project_id: Uuid,
        name: &str,
        exclude_env_id: Uuid,
    ) -> Result<()> {
        let project = self.find_project(project_id).ok_or_else(|| {
            EnvlyError::Validation(format!("Project with id '{project_id}' not found"))
        })?;
        if project
            .environments
            .iter()
            .any(|e| e.name == name && e.id != exclude_env_id)
        {
            return Err(EnvlyError::Validation(format!(
                "Environment '{name}' already exists in project '{}'",
                project.name
            )));
        }
        Ok(())
    }

    pub fn validate_name(name: &str, entity: &str) -> Result<()> {
        if name.trim().is_empty() {
            return Err(EnvlyError::Validation(format!(
                "{entity} name cannot be empty"
            )));
        }
        Ok(())
    }

    pub fn validate_local_key(key: &str) -> Result<()> {
        if key.trim().is_empty() {
            return Err(EnvlyError::Validation(
                "Mapping local_key cannot be empty".into(),
            ));
        }
        Ok(())
    }

    pub fn remove_environment(&mut self, project_id: Uuid, env_id: Uuid) -> Result<()> {
        let project = self.find_project_mut(project_id).ok_or_else(|| {
            EnvlyError::Validation(format!("Project with id '{project_id}' not found"))
        })?;
        let len_before = project.environments.len();
        project.environments.retain(|e| e.id != env_id);
        if project.environments.len() == len_before {
            return Err(EnvlyError::Validation(format!(
                "Environment with id '{env_id}' not found"
            )));
        }
        Ok(())
    }

    /// Deactivate all environments in a project, then activate the specified one.
    pub fn activate_environment(&mut self, project_id: Uuid, env_id: Uuid) -> Result<()> {
        let project = self.find_project_mut(project_id).ok_or_else(|| {
            EnvlyError::Validation(format!("Project with id '{project_id}' not found"))
        })?;
        if !project.environments.iter().any(|e| e.id == env_id) {
            return Err(EnvlyError::Validation(format!(
                "Environment with id '{env_id}' not found"
            )));
        }
        for env in &mut project.environments {
            env.is_active = env.id == env_id;
        }
        Ok(())
    }

    pub fn deactivate_environment(&mut self, project_id: Uuid, env_id: Uuid) -> Result<()> {
        let project = self.find_project_mut(project_id).ok_or_else(|| {
            EnvlyError::Validation(format!("Project with id '{project_id}' not found"))
        })?;
        let env = project
            .environments
            .iter_mut()
            .find(|e| e.id == env_id)
            .ok_or_else(|| {
                EnvlyError::Validation(format!("Environment with id '{env_id}' not found"))
            })?;
        env.is_active = false;
        Ok(())
    }

    // -- Mappings (nested under environment) --

    pub fn add_mapping(
        &mut self,
        project_id: Uuid,
        env_id: Uuid,
        mapping: EnvMapping,
    ) -> Result<()> {
        Self::validate_local_key(&mapping.local_key)?;
        if self.find_secret(mapping.secret_id).is_none() {
            return Err(EnvlyError::Validation(format!(
                "Secret with id '{}' not found",
                mapping.secret_id
            )));
        }

        let project = self.find_project_mut(project_id).ok_or_else(|| {
            EnvlyError::Validation(format!("Project with id '{project_id}' not found"))
        })?;
        let env = project
            .environments
            .iter_mut()
            .find(|e| e.id == env_id)
            .ok_or_else(|| {
                EnvlyError::Validation(format!("Environment with id '{env_id}' not found"))
            })?;

        if env
            .mappings
            .iter()
            .any(|m| m.local_key == mapping.local_key)
        {
            return Err(EnvlyError::Validation(format!(
                "Mapping with local_key '{}' already exists in this environment",
                mapping.local_key
            )));
        }

        env.mappings.push(mapping);
        Ok(())
    }

    pub fn remove_mapping(
        &mut self,
        project_id: Uuid,
        env_id: Uuid,
        mapping_id: Uuid,
    ) -> Result<()> {
        let project = self.find_project_mut(project_id).ok_or_else(|| {
            EnvlyError::Validation(format!("Project with id '{project_id}' not found"))
        })?;
        let env = project
            .environments
            .iter_mut()
            .find(|e| e.id == env_id)
            .ok_or_else(|| {
                EnvlyError::Validation(format!("Environment with id '{env_id}' not found"))
            })?;
        let len_before = env.mappings.len();
        env.mappings.retain(|m| m.id != mapping_id);
        if env.mappings.len() == len_before {
            return Err(EnvlyError::Validation(format!(
                "Mapping with id '{mapping_id}' not found"
            )));
        }
        Ok(())
    }
}

// -- Builder helpers for tests/commands --

impl Secret {
    pub fn new(key: String, value: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            key,
            value: Zeroizing::new(value),
            description: String::new(),
            expires_at: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl Project {
    pub fn new(name: String, folder_path: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description: String::new(),
            folder_path,
            env_filename: default_env_filename(),
            starred: false,
            environments: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl Environment {
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description: String::new(),
            is_active: false,
            mappings: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl EnvMapping {
    pub fn new(local_key: String, secret_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            local_key,
            secret_id,
            notes: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vault() -> Vault {
        Vault::new("passphrase", 1)
    }

    #[test]
    fn add_and_find_secret() {
        let mut vault = test_vault();
        let secret = Secret::new("DB_HOST".into(), "localhost".into());
        let id = secret.id;
        vault.add_secret(secret).unwrap();
        assert!(vault.find_secret(id).is_some());
        assert_eq!(vault.find_secret(id).unwrap().key, "DB_HOST");
    }

    #[test]
    fn duplicate_secret_key_rejected() {
        let mut vault = test_vault();
        vault
            .add_secret(Secret::new("KEY".into(), "v1".into()))
            .unwrap();
        let result = vault.add_secret(Secret::new("KEY".into(), "v2".into()));
        assert!(result.is_err());
    }

    #[test]
    fn remove_secret_success() {
        let mut vault = test_vault();
        let secret = Secret::new("KEY".into(), "val".into());
        let id = secret.id;
        vault.add_secret(secret).unwrap();
        vault.remove_secret(id).unwrap();
        assert!(vault.find_secret(id).is_none());
    }

    #[test]
    fn remove_secret_not_found() {
        let mut vault = test_vault();
        let result = vault.remove_secret(Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn remove_referenced_secret_rejected() {
        let mut vault = test_vault();
        let secret = Secret::new("KEY".into(), "val".into());
        let secret_id = secret.id;
        vault.add_secret(secret).unwrap();

        let mut project = Project::new("myapp".into(), "/tmp/myapp".into());
        let project_id = project.id;
        let mut env = Environment::new("dev".into());
        let env_id = env.id;
        env.mappings
            .push(EnvMapping::new("DATABASE_URL".into(), secret_id));
        project.environments.push(env);
        vault.add_project(project).unwrap();

        let result = vault.remove_secret(secret_id);
        assert!(result.is_err());

        // After removing the mapping, deletion should succeed
        vault
            .remove_mapping(
                project_id,
                env_id,
                vault.find_project(project_id).unwrap().environments[0].mappings[0].id,
            )
            .unwrap();
        vault.remove_secret(secret_id).unwrap();
    }

    #[test]
    fn add_and_find_project() {
        let mut vault = test_vault();
        let project = Project::new("api".into(), "/code/api".into());
        let id = project.id;
        vault.add_project(project).unwrap();
        assert!(vault.find_project(id).is_some());
    }

    #[test]
    fn duplicate_project_name_rejected() {
        let mut vault = test_vault();
        vault
            .add_project(Project::new("api".into(), "/a".into()))
            .unwrap();
        let result = vault.add_project(Project::new("api".into(), "/b".into()));
        assert!(result.is_err());
    }

    #[test]
    fn environment_crud() {
        let mut vault = test_vault();
        let project = Project::new("api".into(), "/code/api".into());
        let project_id = project.id;
        vault.add_project(project).unwrap();

        let env = Environment::new("staging".into());
        let env_id = env.id;
        vault.add_environment(project_id, env).unwrap();

        let project = vault.find_project(project_id).unwrap();
        assert_eq!(project.environments.len(), 1);
        assert_eq!(project.environments[0].name, "staging");

        vault.remove_environment(project_id, env_id).unwrap();
        let project = vault.find_project(project_id).unwrap();
        assert!(project.environments.is_empty());
    }

    #[test]
    fn activate_deactivate_environment() {
        let mut vault = test_vault();
        let project = Project::new("api".into(), "/code/api".into());
        let pid = project.id;
        vault.add_project(project).unwrap();

        let env1 = Environment::new("dev".into());
        let eid1 = env1.id;
        let env2 = Environment::new("staging".into());
        let eid2 = env2.id;
        vault.add_environment(pid, env1).unwrap();
        vault.add_environment(pid, env2).unwrap();

        vault.activate_environment(pid, eid1).unwrap();
        let p = vault.find_project(pid).unwrap();
        assert!(
            p.environments
                .iter()
                .find(|e| e.id == eid1)
                .unwrap()
                .is_active
        );
        assert!(
            !p.environments
                .iter()
                .find(|e| e.id == eid2)
                .unwrap()
                .is_active
        );

        vault.activate_environment(pid, eid2).unwrap();
        let p = vault.find_project(pid).unwrap();
        assert!(
            !p.environments
                .iter()
                .find(|e| e.id == eid1)
                .unwrap()
                .is_active
        );
        assert!(
            p.environments
                .iter()
                .find(|e| e.id == eid2)
                .unwrap()
                .is_active
        );

        vault.deactivate_environment(pid, eid2).unwrap();
        let p = vault.find_project(pid).unwrap();
        assert!(
            !p.environments
                .iter()
                .find(|e| e.id == eid2)
                .unwrap()
                .is_active
        );
    }

    #[test]
    fn mapping_crud() {
        let mut vault = test_vault();
        let secret = Secret::new("PG_PASS".into(), "secret123".into());
        let secret_id = secret.id;
        vault.add_secret(secret).unwrap();

        let project = Project::new("api".into(), "/code/api".into());
        let pid = project.id;
        vault.add_project(project).unwrap();

        let env = Environment::new("dev".into());
        let eid = env.id;
        vault.add_environment(pid, env).unwrap();

        let mapping = EnvMapping::new("DATABASE_PASSWORD".into(), secret_id);
        let mid = mapping.id;
        vault.add_mapping(pid, eid, mapping).unwrap();

        let env = &vault.find_project(pid).unwrap().environments[0];
        assert_eq!(env.mappings.len(), 1);
        assert_eq!(env.mappings[0].local_key, "DATABASE_PASSWORD");

        vault.remove_mapping(pid, eid, mid).unwrap();
        let env = &vault.find_project(pid).unwrap().environments[0];
        assert!(env.mappings.is_empty());
    }

    #[test]
    fn mapping_with_nonexistent_secret_rejected() {
        let mut vault = test_vault();
        let project = Project::new("api".into(), "/code/api".into());
        let pid = project.id;
        vault.add_project(project).unwrap();
        let env = Environment::new("dev".into());
        let eid = env.id;
        vault.add_environment(pid, env).unwrap();

        let mapping = EnvMapping::new("KEY".into(), Uuid::new_v4());
        let result = vault.add_mapping(pid, eid, mapping);
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_mapping_local_key_rejected() {
        let mut vault = test_vault();
        let secret = Secret::new("S1".into(), "v1".into());
        let sid = secret.id;
        vault.add_secret(secret).unwrap();

        let project = Project::new("api".into(), "/code/api".into());
        let pid = project.id;
        vault.add_project(project).unwrap();
        let env = Environment::new("dev".into());
        let eid = env.id;
        vault.add_environment(pid, env).unwrap();

        vault
            .add_mapping(pid, eid, EnvMapping::new("KEY".into(), sid))
            .unwrap();
        let result = vault.add_mapping(pid, eid, EnvMapping::new("KEY".into(), sid));
        assert!(result.is_err());
    }

    #[test]
    fn serialization_round_trip() {
        let mut vault = test_vault();
        vault
            .add_secret(Secret::new("KEY".into(), "secret_value".into()))
            .unwrap();
        vault
            .add_project(Project::new("app".into(), "/tmp/app".into()))
            .unwrap();

        let json = serde_json::to_string(&vault).unwrap();
        let loaded: Vault = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.secrets.len(), 1);
        assert_eq!(loaded.secrets[0].key, "KEY");
        assert_eq!(*loaded.secrets[0].value, "secret_value");
        assert_eq!(loaded.projects.len(), 1);
    }

    #[test]
    fn secret_debug_redacts_value() {
        let secret = Secret::new("KEY".into(), "super_secret".into());
        let debug = format!("{:?}", secret);
        assert!(!debug.contains("super_secret"));
        assert!(debug.contains("REDACTED"));
    }

    // --- New tests: uniqueness on update ---

    #[test]
    fn ensure_secret_key_unique_rejects_duplicate() {
        let mut vault = test_vault();
        let s1 = Secret::new("KEY_A".into(), "v1".into());
        let s2 = Secret::new("KEY_B".into(), "v2".into());
        let s2_id = s2.id;
        vault.add_secret(s1).unwrap();
        vault.add_secret(s2).unwrap();

        let result = vault.ensure_secret_key_unique("KEY_A", s2_id);
        assert!(result.is_err());
    }

    #[test]
    fn ensure_secret_key_unique_allows_same_id() {
        let mut vault = test_vault();
        let s1 = Secret::new("KEY_A".into(), "v1".into());
        let s1_id = s1.id;
        vault.add_secret(s1).unwrap();

        let result = vault.ensure_secret_key_unique("KEY_A", s1_id);
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_project_name_unique_rejects_duplicate() {
        let mut vault = test_vault();
        let p1 = Project::new("api".into(), "/a".into());
        let p2 = Project::new("web".into(), "/b".into());
        let p2_id = p2.id;
        vault.add_project(p1).unwrap();
        vault.add_project(p2).unwrap();

        let result = vault.ensure_project_name_unique("api", p2_id);
        assert!(result.is_err());
    }

    #[test]
    fn ensure_project_name_unique_allows_same_id() {
        let mut vault = test_vault();
        let p1 = Project::new("api".into(), "/a".into());
        let p1_id = p1.id;
        vault.add_project(p1).unwrap();

        let result = vault.ensure_project_name_unique("api", p1_id);
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_env_name_unique_rejects_duplicate() {
        let mut vault = test_vault();
        let project = Project::new("api".into(), "/a".into());
        let pid = project.id;
        vault.add_project(project).unwrap();

        let env1 = Environment::new("dev".into());
        let env2 = Environment::new("staging".into());
        let env2_id = env2.id;
        vault.add_environment(pid, env1).unwrap();
        vault.add_environment(pid, env2).unwrap();

        let result = vault.ensure_env_name_unique(pid, "dev", env2_id);
        assert!(result.is_err());
    }

    #[test]
    fn ensure_env_name_unique_allows_same_id() {
        let mut vault = test_vault();
        let project = Project::new("api".into(), "/a".into());
        let pid = project.id;
        vault.add_project(project).unwrap();

        let env1 = Environment::new("dev".into());
        let env1_id = env1.id;
        vault.add_environment(pid, env1).unwrap();

        let result = vault.ensure_env_name_unique(pid, "dev", env1_id);
        assert!(result.is_ok());
    }

    // --- New tests: empty name/key validation ---

    #[test]
    fn add_secret_empty_key_rejected() {
        let mut vault = test_vault();
        let result = vault.add_secret(Secret::new("".into(), "val".into()));
        assert!(result.is_err());
    }

    #[test]
    fn add_secret_whitespace_only_key_rejected() {
        let mut vault = test_vault();
        let result = vault.add_secret(Secret::new("   ".into(), "val".into()));
        assert!(result.is_err());
    }

    #[test]
    fn add_project_empty_name_rejected() {
        let mut vault = test_vault();
        let result = vault.add_project(Project::new("".into(), "/a".into()));
        assert!(result.is_err());
    }

    #[test]
    fn add_environment_empty_name_rejected() {
        let mut vault = test_vault();
        let project = Project::new("api".into(), "/a".into());
        let pid = project.id;
        vault.add_project(project).unwrap();
        let result = vault.add_environment(pid, Environment::new("".into()));
        assert!(result.is_err());
    }

    #[test]
    fn add_mapping_empty_local_key_rejected() {
        let mut vault = test_vault();
        let secret = Secret::new("S1".into(), "v1".into());
        let sid = secret.id;
        vault.add_secret(secret).unwrap();
        let project = Project::new("api".into(), "/a".into());
        let pid = project.id;
        vault.add_project(project).unwrap();
        let env = Environment::new("dev".into());
        let eid = env.id;
        vault.add_environment(pid, env).unwrap();

        let result = vault.add_mapping(pid, eid, EnvMapping::new("".into(), sid));
        assert!(result.is_err());
    }

    // --- New tests: edge cases ---

    #[test]
    fn add_mapping_same_secret_different_keys_succeeds() {
        let mut vault = test_vault();
        let secret = Secret::new("DB_PASS".into(), "s3cret".into());
        let sid = secret.id;
        vault.add_secret(secret).unwrap();

        let project = Project::new("api".into(), "/code/api".into());
        let pid = project.id;
        vault.add_project(project).unwrap();
        let env = Environment::new("dev".into());
        let eid = env.id;
        vault.add_environment(pid, env).unwrap();

        vault
            .add_mapping(pid, eid, EnvMapping::new("DB_PASSWORD".into(), sid))
            .unwrap();
        vault
            .add_mapping(pid, eid, EnvMapping::new("DATABASE_PASSWORD".into(), sid))
            .unwrap();

        let envs = &vault.find_project(pid).unwrap().environments[0];
        assert_eq!(envs.mappings.len(), 2);
    }

    #[test]
    fn multiple_projects_independent_environments() {
        let mut vault = test_vault();
        let p1 = Project::new("api".into(), "/a".into());
        let p2 = Project::new("web".into(), "/b".into());
        let pid1 = p1.id;
        let pid2 = p2.id;
        vault.add_project(p1).unwrap();
        vault.add_project(p2).unwrap();

        // Same env name in different projects should succeed
        vault
            .add_environment(pid1, Environment::new("dev".into()))
            .unwrap();
        vault
            .add_environment(pid2, Environment::new("dev".into()))
            .unwrap();

        assert_eq!(vault.find_project(pid1).unwrap().environments.len(), 1);
        assert_eq!(vault.find_project(pid2).unwrap().environments.len(), 1);
    }

    #[test]
    fn activate_nonexistent_environment_fails() {
        let mut vault = test_vault();
        let project = Project::new("api".into(), "/a".into());
        let pid = project.id;
        vault.add_project(project).unwrap();

        let result = vault.activate_environment(pid, Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn deactivate_already_inactive_is_ok() {
        let mut vault = test_vault();
        let project = Project::new("api".into(), "/a".into());
        let pid = project.id;
        vault.add_project(project).unwrap();
        let env = Environment::new("dev".into());
        let eid = env.id;
        vault.add_environment(pid, env).unwrap();

        // Env starts inactive; deactivating should still succeed
        vault.deactivate_environment(pid, eid).unwrap();
        let p = vault.find_project(pid).unwrap();
        assert!(!p.environments[0].is_active);
    }

    #[test]
    fn remove_environment_nonexistent_project_fails() {
        let mut vault = test_vault();
        let result = vault.remove_environment(Uuid::new_v4(), Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn remove_mapping_nonexistent_mapping_fails() {
        let mut vault = test_vault();
        let project = Project::new("api".into(), "/a".into());
        let pid = project.id;
        vault.add_project(project).unwrap();
        let env = Environment::new("dev".into());
        let eid = env.id;
        vault.add_environment(pid, env).unwrap();

        let result = vault.remove_mapping(pid, eid, Uuid::new_v4());
        assert!(result.is_err());
    }
}
