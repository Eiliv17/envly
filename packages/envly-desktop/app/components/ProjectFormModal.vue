<script setup lang="ts">
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import type { ProjectSummary } from '~/types/tauri'

const props = defineProps<{
  project: ProjectSummary | null
}>()

const open = defineModel<boolean>('open', { default: false })
const emit = defineEmits<{ saved: [] }>()

const projectsStore = useProjectsStore()
const toast = useToast()
const { t } = useI18n()

const form = reactive({
  name: '',
  description: '',
  folder_path: '',
  env_filename: '.env',
})
const errors = reactive({ name: '', folder_path: '' })
const submitting = ref(false)

const isEditing = computed(() => !!props.project)
const title = computed(() => isEditing.value ? t('projectForm.editTitle') : t('projectForm.newTitle'))

watch(() => form.name, () => { errors.name = '' })
watch(() => form.folder_path, () => { errors.folder_path = '' })

watch(open, (val) => {
  if (val && props.project) {
    form.name = props.project.name
    form.description = props.project.description
    form.folder_path = props.project.folder_path
    form.env_filename = props.project.env_filename
  } else if (val) {
    form.name = ''
    form.description = ''
    form.folder_path = ''
    form.env_filename = '.env'
  }
})

async function pickFolder() {
  const selected = await openDialog({ directory: true, title: t('projectForm.selectFolder') })
  if (selected) {
    form.folder_path = String(selected)
  }
}

async function onSubmit() {
  errors.name = ''
  errors.folder_path = ''
  let valid = true
  if (!form.name.trim()) {
    errors.name = t('projectForm.nameRequired')
    valid = false
  }
  if (!form.folder_path.trim()) {
    errors.folder_path = t('projectForm.pathRequired')
    valid = false
  }
  if (!valid) return
  submitting.value = true
  try {
    if (isEditing.value && props.project) {
      await projectsStore.updateProject({
        id: props.project.id,
        name: form.name.trim(),
        description: form.description,
        folder_path: form.folder_path.trim(),
        env_filename: form.env_filename || '.env',
      })
      toast.add({ title: t('projectForm.updated'), color: 'success' })
    } else {
      await projectsStore.createProject({
        name: form.name.trim(),
        folder_path: form.folder_path.trim(),
        description: form.description,
        env_filename: form.env_filename || '.env',
      })
      toast.add({ title: t('projectForm.created'), color: 'success' })
    }
    emit('saved')
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <UModal v-model:open="open" :title="title">
    <template #body>
      <form class="space-y-4" @submit.prevent="onSubmit">
        <UFormField :label="$t('projectForm.projectName')" :error="errors.name || undefined" required>
          <UInput v-model="form.name" :placeholder="$t('projectForm.projectNamePlaceholder')" class="w-full" />
        </UFormField>

        <UFormField :label="$t('projectForm.description')">
          <UTextarea v-model="form.description" :placeholder="$t('projectForm.descriptionPlaceholder')" :rows="2" class="w-full" />
        </UFormField>

        <UFormField :label="$t('projectForm.folderPath')" :error="errors.folder_path || undefined" required>
          <UInput :model-value="form.folder_path" readonly :placeholder="$t('projectForm.folderPathPlaceholder')" class="w-full cursor-pointer" trailing-icon="i-ph-folder-open" @click="pickFolder" />
        </UFormField>

        <UFormField :label="$t('projectForm.envFilename')">
          <UInput v-model="form.env_filename" :placeholder="$t('projectForm.envFilenamePlaceholder')" class="w-full" />
        </UFormField>

        <div class="flex gap-2 justify-end pt-2">
          <UButton :label="$t('common.cancel')" color="neutral" variant="outline" @click="open = false" />
          <UButton type="submit" :label="isEditing ? $t('common.save') : $t('common.create')" :loading="submitting" />
        </div>
      </form>
    </template>
  </UModal>
</template>
