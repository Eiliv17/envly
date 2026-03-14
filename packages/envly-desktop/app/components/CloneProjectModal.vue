<script setup lang="ts">
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import type { ProjectSummary } from '~/types/tauri'

const props = defineProps<{
  project: ProjectSummary | null
}>()

const open = defineModel<boolean>('open', { default: false })
const emit = defineEmits<{ cloned: [] }>()

const projectsStore = useProjectsStore()
const toast = useToast()
const { t } = useI18n()

const newName = ref('')
const folderPath = ref('')
const submitting = ref(false)

watch(open, (val) => {
  if (val && props.project) {
    newName.value = `${props.project.name} (copy)`
    folderPath.value = ''
  }
})

async function pickFolder() {
  const selected = await openDialog({ directory: true, title: t('cloneProject.selectFolder') })
  if (selected) {
    folderPath.value = String(selected)
  }
}

async function onSubmit() {
  if (!newName.value.trim()) {
    toast.add({ title: t('cloneProject.nameRequired'), color: 'warning' })
    return
  }
  if (!folderPath.value.trim()) {
    toast.add({ title: t('cloneProject.pathRequired'), color: 'warning' })
    return
  }
  if (!props.project) return
  submitting.value = true
  try {
    await projectsStore.cloneProject({
      source_project_id: props.project.id,
      new_name: newName.value.trim(),
      new_folder_path: folderPath.value.trim(),
    })
    toast.add({ title: t('cloneProject.cloned'), color: 'success' })
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
  <UModal v-model:open="open" :title="$t('cloneProject.title')">
    <template #body>
      <form class="space-y-4" @submit.prevent="onSubmit">
        <UFormField :label="$t('cloneProject.newName')" required>
          <UInput v-model="newName" class="w-full" />
        </UFormField>

        <UFormField :label="$t('cloneProject.folderPath')" required>
          <UInput
            :model-value="folderPath"
            readonly
            :placeholder="$t('projectForm.folderPathPlaceholder')"
            class="w-full cursor-pointer"
            trailing-icon="i-ph-folder-open"
            @click="pickFolder"
          />
        </UFormField>

        <div class="flex gap-2 justify-end pt-2">
          <UButton :label="$t('common.cancel')" color="neutral" variant="outline" @click="open = false" />
          <UButton type="submit" :label="$t('cloneProject.clone')" icon="i-ph-copy" :loading="submitting" />
        </div>
      </form>
    </template>
  </UModal>
</template>
