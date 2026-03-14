import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { SecretSummary, ParsedEnvEntry, BulkCreateResult } from '~/types/tauri'

export const useSecretsStore = defineStore('secrets', () => {
  const secrets = ref<SecretSummary[]>([])
  const loading = ref(false)

  const secretOptions = computed(() =>
    secrets.value.map(s => ({ label: s.key, value: s.id })),
  )

  function secretById(id: string) {
    return secrets.value.find(s => s.id === id)
  }

  async function fetchSecrets() {
    loading.value = true
    try {
      secrets.value = await invoke<SecretSummary[]>('list_secrets')
    } finally {
      loading.value = false
    }
  }

  async function createSecret(args: {
    key: string
    value: string
    description?: string
    expires_at?: string | null
    tags?: string[]
  }): Promise<string> {
    return invoke<string>('create_secret', { args })
  }

  async function updateSecret(args: {
    id: string
    key?: string
    value?: string
    description?: string
    expires_at?: string | null
    tags?: string[]
  }) {
    await invoke<void>('update_secret', { args })
  }

  async function deleteSecret(id: string) {
    await invoke<void>('delete_secret', { id })
  }

  function revealSecretValue(id: string): Promise<string> {
    return invoke<string>('reveal_secret_value', { id })
  }

  function parseEnvFile(path: string): Promise<ParsedEnvEntry[]> {
    return invoke<ParsedEnvEntry[]>('parse_env_file', { path })
  }

  async function bulkCreateSecrets(entries: { key: string; value: string }[]): Promise<BulkCreateResult> {
    return invoke<BulkCreateResult>('bulk_create_secrets', { entries })
  }

  function $reset() {
    secrets.value = []
    loading.value = false
  }

  return {
    secrets,
    loading,
    secretOptions,
    secretById,
    fetchSecrets,
    createSecret,
    updateSecret,
    deleteSecret,
    revealSecretValue,
    parseEnvFile,
    bulkCreateSecrets,
    $reset,
  }
})
