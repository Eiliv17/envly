import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { EnvironmentSummary, MappingSummary } from '~/types/tauri'

export const useEnvironmentsStore = defineStore('environments', () => {
  const environmentsByProject = ref<Record<string, EnvironmentSummary[]>>({})
  const mappingsByEnv = ref<Record<string, MappingSummary[]>>({})
  const loadedProjectIds = ref<Set<string>>(new Set())
  const currentMappingEnvId = ref<string | null>(null)
  const currentMappingProjectId = ref<string | null>(null)
  const loading = ref(false)

  function environmentsFor(projectId: string): EnvironmentSummary[] {
    return environmentsByProject.value[projectId] ?? []
  }

  function mappingsFor(envId: string): MappingSummary[] {
    return mappingsByEnv.value[envId] ?? []
  }

  async function fetchEnvironments(projectId: string) {
    const envs = await invoke<EnvironmentSummary[]>('list_environments', { projectId })
    environmentsByProject.value[projectId] = envs
    loadedProjectIds.value.add(projectId)
  }

  async function fetchMappings(projectId: string, envId: string) {
    const m = await invoke<MappingSummary[]>('list_mappings', { projectId, envId })
    mappingsByEnv.value[envId] = m
    currentMappingProjectId.value = projectId
    currentMappingEnvId.value = envId
  }

  async function refetchLoaded() {
    const ids = Array.from(loadedProjectIds.value)
    await Promise.all(ids.map(pid => fetchEnvironments(pid)))
    if (currentMappingProjectId.value && currentMappingEnvId.value) {
      await fetchMappings(currentMappingProjectId.value, currentMappingEnvId.value)
    }
  }

  async function createEnvironment(args: {
    project_id: string
    name: string
    description?: string
  }): Promise<string> {
    return invoke<string>('create_environment', { args })
  }

  async function updateEnvironment(args: {
    project_id: string
    env_id: string
    name?: string
    description?: string
  }) {
    await invoke<void>('update_environment', { args })
  }

  async function deleteEnvironment(projectId: string, envId: string) {
    await invoke<void>('delete_environment', { projectId, envId })
  }

  async function activateEnvironment(projectId: string, envId: string) {
    await invoke<void>('activate_environment', { projectId, envId })
  }

  async function deactivateEnvironment(projectId: string, envId: string) {
    await invoke<void>('deactivate_environment', { projectId, envId })
  }

  async function addMapping(args: {
    project_id: string
    env_id: string
    local_key: string
    secret_id: string
    notes?: string
  }): Promise<string> {
    return invoke<string>('add_mapping', { args })
  }

  async function removeMapping(projectId: string, envId: string, mappingId: string) {
    await invoke<void>('remove_mapping', { projectId, envId, mappingId })
  }

  async function cloneEnvironment(args: {
    source_project_id: string
    source_env_id: string
    target_project_id: string
    new_name: string
  }): Promise<string> {
    return invoke<string>('clone_environment', { args })
  }

  function $reset() {
    environmentsByProject.value = {}
    mappingsByEnv.value = {}
    loadedProjectIds.value = new Set()
    currentMappingEnvId.value = null
    currentMappingProjectId.value = null
    loading.value = false
  }

  return {
    environmentsByProject,
    mappingsByEnv,
    loadedProjectIds,
    currentMappingEnvId,
    currentMappingProjectId,
    loading,
    environmentsFor,
    mappingsFor,
    fetchEnvironments,
    fetchMappings,
    refetchLoaded,
    createEnvironment,
    updateEnvironment,
    deleteEnvironment,
    activateEnvironment,
    deactivateEnvironment,
    addMapping,
    removeMapping,
    cloneEnvironment,
    $reset,
  }
})
