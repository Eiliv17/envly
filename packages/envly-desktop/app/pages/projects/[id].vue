<script setup lang="ts">
import type { EnvironmentSummary } from '~/types/tauri'

const route = useRoute()
const projectId = route.params.id as string

const projectsStore = useProjectsStore()
const environmentsStore = useEnvironmentsStore()
const secretsStore = useSecretsStore()
const toast = useToast()
const { t } = useI18n()

const activeEnvId = ref<string | null>(null)
const loading = ref(true)

const envModalOpen = ref(false)
const editingEnv = ref<EnvironmentSummary | null>(null)
const cloneEnvModalOpen = ref(false)
const cloningEnv = ref<EnvironmentSummary | null>(null)

const newMappingKey = ref('')
const newMappingSecretId = ref('')
const newMappingNotes = ref('')

const project = computed(() => projectsStore.projectById(projectId))
const environments = computed(() => environmentsStore.environmentsFor(projectId))
const mappings = computed(() =>
  activeEnvId.value ? environmentsStore.mappingsFor(activeEnvId.value) : [],
)

function openCreateEnvModal() {
  editingEnv.value = null
  envModalOpen.value = true
}

function openEditEnvModal(env: EnvironmentSummary) {
  editingEnv.value = env
  envModalOpen.value = true
}

function openCloneEnvModal(env: EnvironmentSummary) {
  cloningEnv.value = env
  cloneEnvModalOpen.value = true
}

function onEnvCloned() {
  cloneEnvModalOpen.value = false
}

function onEnvSaved() {
  envModalOpen.value = false
}

async function load() {
  loading.value = true
  try {
    await projectsStore.fetchProjects()
    if (!project.value) {
      toast.add({ title: t('projectDetail.notFound'), color: 'error' })
      return navigateTo('/projects')
    }
    await environmentsStore.fetchEnvironments(projectId)
    await secretsStore.fetchSecrets()

    const envs = environments.value
    const active = envs.find(e => e.is_active)
    if (active) {
      activeEnvId.value = active.id
      await environmentsStore.fetchMappings(projectId, active.id)
    } else if (envs.length > 0) {
      activeEnvId.value = envs[0]!.id
      await environmentsStore.fetchMappings(projectId, envs[0]!.id)
    }
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  } finally {
    loading.value = false
  }
}

async function handleDeleteEnv(envId: string) {
  try {
    await environmentsStore.deleteEnvironment(projectId, envId)
    toast.add({ title: t('projectDetail.envDeleted'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function handleActivate(envId: string) {
  try {
    await environmentsStore.activateEnvironment(projectId, envId)
    toast.add({ title: t('projectDetail.envActivated'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function handleDeactivate(envId: string) {
  try {
    await environmentsStore.deactivateEnvironment(projectId, envId)
    toast.add({ title: t('projectDetail.envDeactivated'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function handleAddMapping() {
  if (!newMappingKey.value.trim() || !newMappingSecretId.value || !activeEnvId.value) return
  try {
    await environmentsStore.addMapping({
      project_id: projectId,
      env_id: activeEnvId.value,
      local_key: newMappingKey.value.trim(),
      secret_id: newMappingSecretId.value,
      notes: newMappingNotes.value,
    })
    newMappingKey.value = ''
    newMappingSecretId.value = ''
    newMappingNotes.value = ''
    toast.add({ title: t('projectDetail.mappingAdded'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function handleRemoveMapping(mappingId: string) {
  if (!activeEnvId.value) return
  try {
    await environmentsStore.removeMapping(projectId, activeEnvId.value, mappingId)
    toast.add({ title: t('projectDetail.mappingRemoved'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

function selectEnv(envId: string) {
  activeEnvId.value = envId
  environmentsStore.fetchMappings(projectId, envId)
}

onMounted(load)
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center gap-3">
      <UButton icon="i-ph-arrow-left" color="neutral" variant="ghost" to="/projects" />
      <div>
        <h1 class="text-xl font-semibold">{{ project?.name }}</h1>
        <p v-if="project?.description" class="text-sm text-muted">{{ project.description }}</p>
      </div>
    </div>

    <div v-if="loading" class="flex justify-center py-8">
      <UIcon name="i-ph-spinner" class="animate-spin size-6" />
    </div>

    <template v-else>
      <!-- Environments -->
      <UCard>
        <template #header>
          <div class="flex items-center justify-between">
            <h2 class="font-semibold text-sm">{{ $t('projectDetail.environments') }}</h2>
            <UButton icon="i-ph-plus" size="xs" variant="outline" :label="$t('common.add')" @click="openCreateEnvModal" />
          </div>
        </template>

        <div v-if="environments.length === 0" class="text-sm text-muted py-4 text-center">
          {{ $t('projectDetail.emptyEnvs') }}
        </div>

        <div v-else class="space-y-2">
          <div
            v-for="env in environments"
            :key="env.id"
            class="flex items-center justify-between p-3 rounded-lg cursor-pointer transition-colors"
            :class="activeEnvId === env.id ? 'bg-elevated ring-1 ring-primary' : 'hover:bg-elevated'"
            @click="selectEnv(env.id)"
          >
            <div class="flex items-center gap-2">
              <span class="font-medium text-sm">{{ env.name }}</span>
              <UBadge v-if="env.is_active" color="success" variant="subtle" size="xs">{{ $t('projectDetail.active') }}</UBadge>
              <span class="text-xs text-muted">{{ $t('projectDetail.mappingCount', { n: env.mapping_count }) }}</span>
            </div>
            <div class="flex gap-1" @click.stop>
              <UButton
                v-if="!env.is_active"
                icon="i-ph-play"
                color="success"
                variant="ghost"
                size="xs"
                @click="handleActivate(env.id)"
              />
              <UButton
                v-else
                icon="i-ph-stop"
                color="warning"
                variant="ghost"
                size="xs"
                @click="handleDeactivate(env.id)"
              />
              <UButton icon="i-ph-copy" color="neutral" variant="ghost" size="xs" @click="openCloneEnvModal(env)" />
              <UButton icon="i-ph-pencil-simple" color="neutral" variant="ghost" size="xs" @click="openEditEnvModal(env)" />
              <UButton icon="i-ph-trash" color="error" variant="ghost" size="xs" @click="handleDeleteEnv(env.id)" />
            </div>
          </div>
        </div>
      </UCard>

      <EnvironmentFormModal
        v-model:open="envModalOpen"
        :environment="editingEnv"
        :project-id="projectId"
        @saved="onEnvSaved"
      />

      <CloneEnvironmentModal
        v-model:open="cloneEnvModalOpen"
        :environment="cloningEnv"
        :project-id="projectId"
        @cloned="onEnvCloned"
      />

      <!-- Mappings -->
      <UCard v-if="activeEnvId">
        <template #header>
          <h2 class="font-semibold text-sm">
            {{ $t('projectDetail.mappingsFor', { name: environments.find(e => e.id === activeEnvId)?.name }) }}
          </h2>
        </template>

        <div class="space-y-3 mb-4 p-3 rounded-lg bg-elevated">
          <div class="grid grid-cols-1 sm:grid-cols-3 gap-2">
            <UFormField :label="$t('projectDetail.localKey')">
              <UInput v-model="newMappingKey" :placeholder="$t('projectDetail.localKeyPlaceholder')" class="w-full" />
            </UFormField>
            <UFormField :label="$t('projectDetail.secret')">
              <USelectMenu
                v-model="newMappingSecretId"
                :items="secretsStore.secretOptions"
                :placeholder="$t('projectDetail.secretPlaceholder')"
                value-key="value"
                class="w-full"
              />
            </UFormField>
            <UFormField :label="$t('projectDetail.notes')">
              <UInput v-model="newMappingNotes" :placeholder="$t('projectDetail.notesPlaceholder')" class="w-full" />
            </UFormField>
          </div>
          <UButton :label="$t('projectDetail.addMapping')" size="sm" @click="handleAddMapping" />
        </div>

        <div v-if="mappings.length === 0" class="text-sm text-muted text-center py-4">
          {{ $t('projectDetail.emptyMappings') }}
        </div>

        <div v-else class="divide-y divide-default">
          <div v-for="mapping in mappings" :key="mapping.id" class="flex items-center justify-between py-3">
            <div class="space-y-0.5 min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <code class="text-sm font-mono">{{ mapping.local_key }}</code>
                <UIcon name="i-ph-arrow-right" class="size-3.5 text-muted" />
                <code class="text-sm font-mono text-primary">{{ mapping.secret_key }}</code>
              </div>
              <p v-if="mapping.notes" class="text-xs text-muted">{{ mapping.notes }}</p>
            </div>
            <UButton icon="i-ph-trash" color="error" variant="ghost" size="xs" @click="handleRemoveMapping(mapping.id)" />
          </div>
        </div>
      </UCard>
    </template>
  </div>
</template>
