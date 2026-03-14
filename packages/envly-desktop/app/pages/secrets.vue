<script setup lang="ts">
import type { SecretSummary } from '~/types/tauri'

const secretsStore = useSecretsStore()
const toast = useToast()
const { t } = useI18n()

const search = ref('')
const showCreateModal = ref(false)
const editingSecret = ref<SecretSummary | null>(null)
const revealedValues = ref<Record<string, string>>({})
const bulkImportOpen = ref(false)

const filteredSecrets = computed(() => {
  const q = search.value.toLowerCase()
  if (!q) return secretsStore.secrets
  return secretsStore.secrets.filter(
    s => s.key.toLowerCase().includes(q)
      || s.description.toLowerCase().includes(q)
      || s.tags.some(t => t.toLowerCase().includes(q)),
  )
})

async function load() {
  revealedValues.value = {}
  try {
    await secretsStore.fetchSecrets()
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function toggleReveal(id: string) {
  if (revealedValues.value[id]) {
    delete revealedValues.value[id]
    return
  }
  try {
    revealedValues.value[id] = await secretsStore.revealSecretValue(id)
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function handleDelete(secret: SecretSummary) {
  try {
    await secretsStore.deleteSecret(secret.id)
    toast.add({ title: t('secrets.deleted'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

function openEdit(secret: SecretSummary) {
  editingSecret.value = secret
  showCreateModal.value = true
}

function openCreate() {
  editingSecret.value = null
  showCreateModal.value = true
}

function onSaved() {
  showCreateModal.value = false
  editingSecret.value = null
}

onMounted(load)
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-xl font-semibold">{{ $t('secrets.title') }}</h1>
      <div class="flex gap-2">
        <UButton icon="i-ph-upload-simple" :label="$t('bulkImport.title')" variant="outline" @click="bulkImportOpen = true" />
        <UButton icon="i-ph-plus" :label="$t('secrets.newSecret')" @click="openCreate" />
      </div>
    </div>

    <UInput v-model="search" icon="i-ph-magnifying-glass" :placeholder="$t('secrets.searchPlaceholder')" class="max-w-sm" />

    <div v-if="secretsStore.loading" class="flex justify-center py-8">
      <UIcon name="i-ph-spinner" class="animate-spin size-6" />
    </div>

    <div v-else-if="filteredSecrets.length === 0" class="py-12">
      <UEmpty icon="i-ph-key" :title="$t('secrets.emptyTitle')" :description="$t('secrets.emptyDescription')" />
    </div>

    <div v-else class="space-y-3">
      <UCard v-for="secret in filteredSecrets" :key="secret.id">
        <div class="flex items-start justify-between gap-4">
          <div class="space-y-1 min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span class="font-mono font-medium text-sm">{{ secret.key }}</span>
              <UBadge v-if="secret.expires_at" color="warning" variant="subtle" size="xs">
                {{ $t('secrets.expires', { date: secret.expires_at }) }}
              </UBadge>
            </div>
            <p v-if="secret.description" class="text-sm text-muted truncate">{{ secret.description }}</p>
            <div v-if="secret.tags.length" class="flex gap-1 flex-wrap">
              <UBadge v-for="tag in secret.tags" :key="tag" variant="subtle" color="neutral" size="xs">
                {{ tag }}
              </UBadge>
            </div>
            <div v-if="revealedValues[secret.id]" class="mt-2">
              <code class="text-xs bg-elevated px-2 py-1 rounded block break-all">
                {{ revealedValues[secret.id] }}
              </code>
            </div>
          </div>
          <div class="flex gap-1 shrink-0">
            <UButton
              :icon="revealedValues[secret.id] ? 'i-ph-eye-slash' : 'i-ph-eye'"
              color="neutral"
              variant="ghost"
              size="sm"
              @click="toggleReveal(secret.id)"
            />
            <UButton icon="i-ph-pencil-simple" color="neutral" variant="ghost" size="sm" @click="openEdit(secret)" />
            <UButton icon="i-ph-trash" color="error" variant="ghost" size="sm" @click="handleDelete(secret)" />
          </div>
        </div>
      </UCard>
    </div>

    <SecretFormModal
      v-model:open="showCreateModal"
      :secret="editingSecret"
      @saved="onSaved"
    />

    <BulkSecretImportModal
      v-model:open="bulkImportOpen"
      :existing-secrets="secretsStore.secrets"
      @imported="() => bulkImportOpen = false"
    />
  </div>
</template>
