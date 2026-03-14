<script setup lang="ts">
import type { SecretSummary } from '~/types/tauri'

const props = defineProps<{
  secret: SecretSummary | null
}>()

const open = defineModel<boolean>('open', { default: false })
const emit = defineEmits<{ saved: [] }>()

const secretsStore = useSecretsStore()
const toast = useToast()
const { t } = useI18n()

const form = reactive({
  key: '',
  value: '',
  description: '',
  tags: [] as string[],
  expires_at: null as Date | null,
})
const submitting = ref(false)

const isEditing = computed(() => !!props.secret)
const title = computed(() => isEditing.value ? t('secretForm.editTitle') : t('secretForm.newTitle'))

watch(open, async (val) => {
  if (val && props.secret) {
    form.key = props.secret.key
    form.description = props.secret.description
    form.tags = [...props.secret.tags]
    form.expires_at = props.secret.expires_at ? new Date(props.secret.expires_at) : null
    try {
      form.value = await secretsStore.revealSecretValue(props.secret.id)
    } catch {
      form.value = ''
    }
  } else if (val) {
    form.key = ''
    form.value = ''
    form.description = ''
    form.tags = []
    form.expires_at = null
  }
})

async function onSubmit() {
  if (!form.key.trim() || !form.value.trim()) {
    toast.add({ title: t('secretForm.keyValueRequired'), color: 'warning' })
    return
  }
  submitting.value = true
  try {
    const expiresStr = form.expires_at ? form.expires_at.toISOString().split('T')[0] : null
    if (isEditing.value && props.secret) {
      await secretsStore.updateSecret({
        id: props.secret.id,
        key: form.key.trim(),
        value: form.value,
        description: form.description,
        tags: form.tags,
        expires_at: expiresStr,
      })
      toast.add({ title: t('secretForm.updated'), color: 'success' })
    } else {
      await secretsStore.createSecret({
        key: form.key.trim(),
        value: form.value,
        description: form.description,
        tags: form.tags,
        expires_at: expiresStr,
      })
      toast.add({ title: t('secretForm.created'), color: 'success' })
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
        <UFormField :label="$t('secretForm.key')" required>
          <UInput v-model="form.key" :placeholder="$t('secretForm.keyPlaceholder')" class="w-full" />
        </UFormField>

        <UFormField :label="$t('secretForm.value')" required>
          <UTextarea v-model="form.value" :placeholder="$t('secretForm.valuePlaceholder')" :rows="3" class="w-full" />
        </UFormField>

        <UFormField :label="$t('secretForm.description')">
          <UInput v-model="form.description" :placeholder="$t('secretForm.descriptionPlaceholder')" class="w-full" />
        </UFormField>

        <UFormField :label="$t('secretForm.tags')">
          <UInputTags v-model="form.tags" :placeholder="$t('secretForm.tagsPlaceholder')" class="w-full" />
        </UFormField>

        <UFormField :label="$t('secretForm.expiresAt')">
          <UInputDate v-model="form.expires_at" class="w-full" />
        </UFormField>

        <div class="flex gap-2 justify-end pt-2">
          <UButton :label="$t('common.cancel')" color="neutral" variant="outline" @click="open = false" />
          <UButton type="submit" :label="isEditing ? $t('common.save') : $t('common.create')" :loading="submitting" />
        </div>
      </form>
    </template>
  </UModal>
</template>
