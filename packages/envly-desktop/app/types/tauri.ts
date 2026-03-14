export interface VaultEntrySummary {
  id: string
  name: string
  path: string
  cipher_kind: 'passphrase' | 'symmetric'
  created_at: string
  last_accessed: string | null
}

export interface SecretSummary {
  id: string
  key: string
  description: string
  expires_at: string | null
  tags: string[]
  created_at: string
  updated_at: string
}

export interface ActiveEnvInfo {
  id: string
  name: string
}

export interface ProjectSummary {
  id: string
  name: string
  description: string
  folder_path: string
  env_filename: string
  environment_count: number
  starred: boolean
  active_env: ActiveEnvInfo | null
  path_valid: boolean
  created_at: string
  updated_at: string
}

export interface EnvironmentSummary {
  id: string
  name: string
  description: string
  is_active: boolean
  mapping_count: number
  created_at: string
  updated_at: string
}

export interface MappingSummary {
  id: string
  local_key: string
  secret_id: string
  secret_key: string
  notes: string
}

export interface ParsedEnvEntry {
  key: string
  value: string
}

export interface BulkCreateResult {
  created: number
  updated: number
}

export type VaultStatus = 'uninitialized' | 'locked' | 'unlocked'
