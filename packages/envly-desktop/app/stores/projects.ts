import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { ProjectSummary } from '~/types/tauri'

export const useProjectsStore = defineStore('projects', () => {
  const projects = ref<ProjectSummary[]>([])
  const loading = ref(false)

  const starredProjects = computed(() =>
    projects.value.filter(p => p.starred),
  )

  function projectById(id: string) {
    return projects.value.find(p => p.id === id)
  }

  async function fetchProjects() {
    loading.value = true
    try {
      projects.value = await invoke<ProjectSummary[]>('list_projects')
    } finally {
      loading.value = false
    }
  }

  async function createProject(args: {
    name: string
    folder_path: string
    description?: string
    env_filename?: string
  }): Promise<string> {
    return invoke<string>('create_project', { args })
  }

  async function updateProject(args: {
    id: string
    name?: string
    description?: string
    folder_path?: string
    env_filename?: string
  }) {
    await invoke<void>('update_project', { args })
  }

  async function deleteProject(id: string) {
    await invoke<void>('delete_project', { id })
  }

  function validateProjectPath(path: string): Promise<boolean> {
    return invoke<boolean>('validate_project_path', { path })
  }

  async function toggleProjectStarred(id: string): Promise<boolean> {
    return invoke<boolean>('toggle_project_starred', { id })
  }

  async function cloneProject(args: {
    source_project_id: string
    new_name: string
    new_folder_path: string
  }): Promise<string> {
    return invoke<string>('clone_project', { args })
  }

  function $reset() {
    projects.value = []
    loading.value = false
  }

  return {
    projects,
    loading,
    starredProjects,
    projectById,
    fetchProjects,
    createProject,
    updateProject,
    deleteProject,
    validateProjectPath,
    toggleProjectStarred,
    cloneProject,
    $reset,
  }
})
