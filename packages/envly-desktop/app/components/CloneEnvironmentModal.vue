<script setup lang="ts">
import type { EnvironmentSummary } from '~/types/tauri'

const props = defineProps<{
  environment: EnvironmentSummary | null
  projectId: string
}>()

const open = defineModel<boolean>('open', { default: false })
const emit = defineEmits<{ cloned: [] }>()

const environmentsStore = useEnvironmentsStore()
const projectsStore = useProjectsStore()
const toast = useToast()
const { t } = useI18n()

const newName = ref('')
const targetProjectId = ref('')
const submitting = ref(false)

const projectOptions = computed(() =>
  projectsStore.projects.map(p => ({ label: p.name, value: p.id })),
)

watch(open, async (val) => {
  if (val && props.environment) {
    newName.value = `${props.environment.name} (copy)`
    targetProjectId.value = props.projectId
    if (projectsStore.projects.length === 0) {
      await projectsStore.fetchProjects()
    }
  }
})

async function onSubmit() {
  if (!newName.value.trim()) {
    toast.add({ title: t('cloneEnv.nameRequired'), color: 'warning' })
    return
  }
  if (!props.environment) return
  submitting.value = true
  try {
    await environmentsStore.cloneEnvironment({
      source_project_id: props.projectId,
      source_env_id: props.environment.id,
      target_project_id: targetProjectId.value,
      new_name: newName.value.trim(),
    })
    toast.add({ title: t('cloneEnv.cloned'), color: 'success' })
    open.value = false
    emit('cloned')
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <UModal v-model:open="open" :title="$t('cloneEnv.title')">
    <template #body>
      <form class="space-y-4" @submit.prevent="onSubmit">
        <UFormField :label="$t('cloneEnv.newName')" required>
          <UInput v-model="newName" class="w-full" />
        </UFormField>

        <UFormField :label="$t('cloneEnv.targetProject')">
          <USelectMenu
            v-model="targetProjectId"
            :items="projectOptions"
            value-key="value"
            class="w-full"
          />
        </UFormField>

        <div class="flex gap-2 justify-end pt-2">
          <UButton :label="$t('common.cancel')" color="neutral" variant="outline" @click="open = false" />
          <UButton type="submit" :label="$t('cloneEnv.clone')" icon="i-ph-copy" :loading="submitting" />
        </div>
      </form>
    </template>
  </UModal>
</template>
