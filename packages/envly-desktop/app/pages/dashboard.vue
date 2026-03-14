<script setup lang="ts">
import type { ProjectSummary } from '~/types/tauri'

const projectsStore = useProjectsStore()
const environmentsStore = useEnvironmentsStore()
const toast = useToast()
const { t } = useI18n()

const selectedEnvs = ref<Record<string, string>>({})
const loading = ref(true)

const sortedProjects = computed(() => {
  return [...projectsStore.projects].sort((a, b) => {
    if (a.starred !== b.starred) return a.starred ? -1 : 1
    return a.name.localeCompare(b.name)
  })
})

async function load() {
  loading.value = true
  try {
    await projectsStore.fetchProjects()

    const selMap: Record<string, string> = {}

    await Promise.all(
      projectsStore.projects.map(async (p) => {
        await environmentsStore.fetchEnvironments(p.id)
        const envs = environmentsStore.environmentsFor(p.id)
        const active = envs.find(e => e.is_active)
        if (active) {
          selMap[p.id] = active.id
        } else if (envs.length > 0) {
          selMap[p.id] = envs[0]!.id
        }
      }),
    )

    selectedEnvs.value = selMap
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  } finally {
    loading.value = false
  }
}

function isActive(project: ProjectSummary): boolean {
  return !!project.active_env
}

async function handleToggleActive(project: ProjectSummary, value: boolean) {
  const envId = selectedEnvs.value[project.id]
  if (!envId) {
    toast.add({ title: t('dashboard.noEnvSelected'), color: 'warning' })
    return
  }

  try {
    if (value) {
      await environmentsStore.activateEnvironment(project.id, envId)
      toast.add({ title: t('dashboard.activated'), color: 'success' })
    } else {
      const activeId = project.active_env?.id
      if (activeId) {
        await environmentsStore.deactivateEnvironment(project.id, activeId)
        toast.add({ title: t('dashboard.deactivated'), color: 'success' })
      }
    }
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function handleEnvChange(project: ProjectSummary, newEnvId: string) {
  selectedEnvs.value[project.id] = newEnvId

  if (!isActive(project)) return

  try {
    await environmentsStore.activateEnvironment(project.id, newEnvId)
    toast.add({ title: t('dashboard.switched'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function handleToggleStar(project: ProjectSummary) {
  try {
    await projectsStore.toggleProjectStarred(project.id)
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

function envOptions(projectId: string) {
  return environmentsStore.environmentsFor(projectId).map(e => ({
    label: e.name,
    value: e.id,
  }))
}

onMounted(load)
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-xl font-semibold">{{ $t('dashboard.title') }}</h1>
      <UButton to="/projects" icon="i-ph-kanban" :label="$t('dashboard.manageProjects')" variant="outline" color="neutral" size="sm" />
    </div>

    <div v-if="loading" class="flex justify-center py-12">
      <UIcon name="i-ph-spinner" class="animate-spin size-6" />
    </div>

    <div v-else-if="projectsStore.projects.length === 0" class="py-16">
      <UEmpty
        icon="i-ph-kanban"
        :title="$t('dashboard.emptyTitle')"
        :description="$t('dashboard.emptyDescription')"
      >
        <template #actions>
          <UButton to="/projects" :label="$t('dashboard.goToProjects')" />
        </template>
      </UEmpty>
    </div>

    <div v-else class="space-y-3">
      <UCard v-for="project in sortedProjects" :key="project.id">
        <div class="flex items-center gap-4">
          <UButton
            :icon="project.starred ? 'i-ph-star' : 'i-ph-star'"
            :color="project.starred ? 'warning' : 'neutral'"
            :variant="project.starred ? 'solid' : 'ghost'"
            size="xs"
            @click="handleToggleStar(project)"
          />

          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <NuxtLink :to="`/projects/${project.id}`" class="font-semibold text-sm hover:underline">
                {{ project.name }}
              </NuxtLink>
              <UBadge v-if="!project.path_valid" color="error" variant="subtle" size="xs">{{ $t('dashboard.pathMissing') }}</UBadge>
            </div>
            <p class="text-xs text-muted font-mono truncate">{{ project.folder_path }}</p>
          </div>

          <div class="w-44 shrink-0">
            <USelectMenu
              v-if="envOptions(project.id).length > 0"
              :model-value="selectedEnvs[project.id]"
              :items="envOptions(project.id)"
              value-key="value"
              :placeholder="$t('dashboard.noEnvsPlaceholder')"
              size="sm"
              @update:model-value="(val: string) => handleEnvChange(project, val)"
            />
            <span v-else class="text-xs text-muted">{{ $t('dashboard.noEnvironments') }}</span>
          </div>

          <USwitch
            :model-value="isActive(project)"
            :disabled="!envOptions(project.id).length"
            size="sm"
            @update:model-value="(val: boolean) => handleToggleActive(project, val)"
          />
        </div>
      </UCard>
    </div>
  </div>
</template>
