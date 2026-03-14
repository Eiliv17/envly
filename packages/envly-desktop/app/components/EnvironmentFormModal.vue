<script setup lang="ts">
import type { EnvironmentSummary } from '~/types/tauri'

const props = defineProps<{
  environment: EnvironmentSummary | null
  projectId: string
}>()

const open = defineModel<boolean>('open', { default: false })
const emit = defineEmits<{
  saved: []
}>()

const environmentsStore = useEnvironmentsStore()
const toast = useToast()
const { t } = useI18n()

const form = reactive({
  name: '',
  description: '',
})
const submitting = ref(false)

const isEditing = computed(() => !!props.environment)
const title = computed(() => isEditing.value ? t('envForm.editTitle') : t('envForm.newTitle'))

watch(open, (val) => {
  if (val && props.environment) {
    form.name = props.environment.name
    form.description = props.environment.description
  } else if (val) {
    form.name = ''
    form.description = ''
  }
})

async function onSubmit() {
  if (!form.name.trim()) {
    toast.add({ title: t('envForm.nameRequired'), color: 'warning' })
    return
  }
  submitting.value = true
  try {
    if (isEditing.value && props.environment) {
      await environmentsStore.updateEnvironment({
        project_id: props.projectId,
        env_id: props.environment.id,
        name: form.name.trim(),
        description: form.description,
      })
      toast.add({ title: t('envForm.updated'), color: 'success' })
    } else {
      await environmentsStore.createEnvironment({
        project_id: props.projectId,
        name: form.name.trim(),
        description: form.description,
      })
      toast.add({ title: t('envForm.created'), color: 'success' })
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
        <UFormField :label="$t('envForm.name')" required>
          <UInput v-model="form.name" :placeholder="$t('envForm.namePlaceholder')" class="w-full" />
        </UFormField>

        <UFormField :label="$t('envForm.description')">
          <UTextarea v-model="form.description" :placeholder="$t('envForm.descriptionPlaceholder')" :rows="2" class="w-full" />
        </UFormField>

        <div class="flex gap-2 justify-end pt-2">
          <UButton :label="$t('common.cancel')" color="neutral" variant="outline" @click="open = false" />
          <UButton type="submit" :label="isEditing ? $t('common.save') : $t('common.create')" :loading="submitting" />
        </div>
      </form>
    </template>
  </UModal>
</template>
