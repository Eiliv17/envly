import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { VaultEntrySummary, VaultStatus } from '~/types/tauri'

export const useVaultStore = defineStore('vault', () => {
  const status = ref<VaultStatus>('uninitialized')
  const activeVaultId = ref<string | null>(null)
  const vaults = ref<VaultEntrySummary[]>([])
  const loading = ref(false)

  async function fetchVaults() {
    vaults.value = await invoke<VaultEntrySummary[]>('list_vaults')
  }

  async function fetchStatus() {
    status.value = await invoke<VaultStatus>('get_vault_status')
    activeVaultId.value = await invoke<string | null>('get_active_vault_id')
  }

  async function selectVault(id: string): Promise<VaultStatus> {
    const newStatus = await invoke<VaultStatus>('select_vault', { id })
    status.value = newStatus
    activeVaultId.value = id
    return newStatus
  }

  async function unlockVault(passphrase?: string) {
    await invoke<void>('unlock_vault', { passphrase: passphrase ?? null })
    status.value = 'unlocked'
  }

  async function lockVault() {
    await invoke<void>('lock_vault')
    status.value = 'locked'

    const secretsStore = useSecretsStore()
    const projectsStore = useProjectsStore()
    const environmentsStore = useEnvironmentsStore()
    secretsStore.$reset()
    projectsStore.$reset()
    environmentsStore.$reset()
  }

  async function createVaultEntry(args: {
    name: string
    path: string
    cipher_kind: 'passphrase' | 'symmetric'
    passphrase?: string
  }): Promise<string> {
    const id = await invoke<string>('create_vault_entry', { args })
    await fetchVaults()
    status.value = 'unlocked'
    activeVaultId.value = id
    return id
  }

  async function deleteVaultEntry(id: string, deleteFile: boolean) {
    await invoke<void>('delete_vault_entry', { id, deleteFile })
    await fetchVaults()
    if (activeVaultId.value === id) {
      activeVaultId.value = null
      status.value = 'uninitialized'
    }
  }

  async function renameVault(id: string, newName: string) {
    await invoke<void>('rename_vault', { id, newName })
    await fetchVaults()
  }

  async function importVaultEntry(args: { name: string; path: string }): Promise<string> {
    const id = await invoke<string>('import_vault_entry', { args })
    await fetchVaults()
    return id
  }

  async function changePassphrase(newPassphrase: string) {
    await invoke<void>('change_passphrase', { newPassphrase })
  }

  async function exportVault(destination: string, passphrase: string) {
    await invoke<void>('export_vault', { destination, passphrase })
  }

  async function getActiveVaultCipherKind(): Promise<'passphrase' | 'symmetric'> {
    return invoke<'passphrase' | 'symmetric'>('get_active_vault_cipher_kind')
  }

  async function migrateCipher(targetKind: 'passphrase' | 'symmetric', passphrase?: string) {
    await invoke<void>('migrate_cipher', { targetKind, passphrase: passphrase ?? null })
  }

  function checkPathExists(path: string): Promise<boolean> {
    return invoke<boolean>('check_path_exists', { path })
  }

  function currentVault() {
    return vaults.value.find(v => v.id === activeVaultId.value) ?? null
  }

  return {
    status,
    activeVaultId,
    vaults,
    loading,
    fetchVaults,
    fetchStatus,
    selectVault,
    unlockVault,
    lockVault,
    createVaultEntry,
    deleteVaultEntry,
    renameVault,
    importVaultEntry,
    changePassphrase,
    exportVault,
    getActiveVaultCipherKind,
    migrateCipher,
    checkPathExists,
    currentVault,
  }
})
