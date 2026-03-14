<script setup lang="ts">
import type { ProjectSummary } from '~/types/tauri'

const projectsStore = useProjectsStore()
const toast = useToast()
const { t } = useI18n()

const showCreateModal = ref(false)
const editingProject = ref<ProjectSummary | null>(null)
const cloneModalOpen = ref(false)
const cloningProject = ref<ProjectSummary | null>(null)

async function load() {
  try {
    await projectsStore.fetchProjects()
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function handleDelete(project: ProjectSummary) {
  try {
    await projectsStore.deleteProject(project.id)
    toast.add({ title: t('projects.deleted'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

function openEdit(project: ProjectSummary) {
  editingProject.value = project
  showCreateModal.value = true
}

function openCreate() {
  editingProject.value = null
  showCreateModal.value = true
}

function openClone(project: ProjectSummary) {
  cloningProject.value = project
  cloneModalOpen.value = true
}

function onCloned() {
  cloneModalOpen.value = false
}

function onSaved() {
  showCreateModal.value = false
  editingProject.value = null
}

onMounted(load)
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-xl font-semibold">{{ $t('projects.title') }}</h1>
      <UButton icon="i-ph-plus" :label="$t('projects.newProject')" @click="openCreate" />
    </div>

    <div v-if="projectsStore.loading" class="flex justify-center py-8">
      <UIcon name="i-ph-spinner" class="animate-spin size-6" />
    </div>

    <div v-else-if="projectsStore.projects.length === 0" class="py-12">
      <UEmpty icon="i-ph-kanban" :title="$t('projects.emptyTitle')" :description="$t('projects.emptyDescription')" />
    </div>

    <div v-else class="grid gap-4 sm:grid-cols-2">
      <UCard v-for="project in projectsStore.projects" :key="project.id" class="cursor-pointer hover:ring-2 hover:ring-primary transition-shadow" @click="navigateTo(`/projects/${project.id}`)">
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <h3 class="font-semibold">{{ project.name }}</h3>
            <div class="flex gap-1" @click.stop>
              <UButton icon="i-ph-copy" color="neutral" variant="ghost" size="xs" @click="openClone(project)" />
              <UButton icon="i-ph-pencil-simple" color="neutral" variant="ghost" size="xs" @click="openEdit(project)" />
              <UButton icon="i-ph-trash" color="error" variant="ghost" size="xs" @click="handleDelete(project)" />
            </div>
          </div>
          <p v-if="project.description" class="text-sm text-muted truncate">{{ project.description }}</p>
          <div class="flex items-center gap-3 text-xs text-muted">
            <span class="flex items-center gap-1">
              <UIcon name="i-ph-stack" class="size-3.5" />
              {{ $t('projects.envCount', project.environment_count) }}
            </span>
            <UBadge v-if="!project.path_valid" color="error" variant="subtle" size="xs">
              {{ $t('projects.pathMissing') }}
            </UBadge>
            <UBadge v-else color="success" variant="subtle" size="xs">
              {{ $t('projects.pathValid') }}
            </UBadge>
          </div>
          <p class="text-xs font-mono text-muted truncate">{{ project.folder_path }}</p>
        </div>
      </UCard>
    </div>

    <ProjectFormModal
      v-model:open="showCreateModal"
      :project="editingProject"
      @saved="onSaved"
    />

    <CloneProjectModal
      v-model:open="cloneModalOpen"
      :project="cloningProject"
      @cloned="onCloned"
    />
  </div>
</template>
