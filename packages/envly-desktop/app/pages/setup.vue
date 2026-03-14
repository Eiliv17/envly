<script setup lang="ts">
import { open as openDialog } from '@tauri-apps/plugin-dialog'

definePageMeta({ layout: 'auth' })

const vaultStore = useVaultStore()
const toast = useToast()
const { t } = useI18n()

const form = reactive({
  name: '',
  path: '',
  cipher_kind: 'passphrase' as 'passphrase' | 'symmetric',
  passphrase: '',
  confirmPassphrase: '',
})
const errors = reactive({ name: '', path: '', passphrase: '', confirmPassphrase: '' })
const submitting = ref(false)
const selectedFolder = ref('')

function sanitizeName(name: string): string {
  return name
    .trim()
    .toLowerCase()
    .replace(/\s+/g, '-')
    .replace(/[^a-z0-9-]/g, '')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '')
}

function buildPath(folder: string, name: string): string {
  const sanitized = sanitizeName(name)
  const filename = sanitized || 'vault'
  return `${folder}/${filename}.envly`
}

watch(() => form.name, () => {
  errors.name = ''
  if (selectedFolder.value) {
    form.path = buildPath(selectedFolder.value, form.name)
  }
})
watch(() => form.path, () => { errors.path = '' })
watch(() => form.passphrase, () => { errors.passphrase = '' })
watch(() => form.confirmPassphrase, () => { errors.confirmPassphrase = '' })

async function pickFolder() {
  const selected = await openDialog({ directory: true, title: t('setup.chooseLocation') })
  if (selected) {
    selectedFolder.value = selected
    form.path = buildPath(selected, form.name)
  }
}

async function onSubmit() {
  errors.name = ''
  errors.path = ''
  errors.passphrase = ''
  errors.confirmPassphrase = ''

  let valid = true
  if (!form.name.trim()) {
    errors.name = t('setup.nameRequired')
    valid = false
  }
  if (!form.path.trim()) {
    errors.path = t('setup.pathRequired')
    valid = false
  }
  if (form.cipher_kind === 'passphrase') {
    if (!form.passphrase) {
      errors.passphrase = t('common.passphraseRequired')
      valid = false
    }
    if (form.passphrase && form.passphrase !== form.confirmPassphrase) {
      errors.confirmPassphrase = t('common.passphraseMismatch')
      valid = false
    }
  }
  if (!valid) return

  submitting.value = true
  try {
    const exists = await vaultStore.checkPathExists(form.path.trim())
    if (exists) {
      errors.path = t('setup.fileExists')
      return
    }

    await vaultStore.createVaultEntry({
      name: form.name.trim(),
      path: form.path.trim(),
      cipher_kind: form.cipher_kind,
      passphrase: form.cipher_kind === 'passphrase' ? form.passphrase : undefined,
    })
    toast.add({ title: t('setup.vaultCreated'), description: t('setup.vaultCreatedDescription', { name: form.name }), color: 'success' })
    await navigateTo('/dashboard')
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="flex-1 flex items-center justify-center p-6">
    <div class="w-full max-w-md space-y-6">
      <div class="text-center space-y-2">
        <h1 class="text-2xl font-bold">{{ $t('setup.title') }}</h1>
        <p class="text-sm text-muted">{{ $t('setup.subtitle') }}</p>
      </div>

      <UCard>
        <form class="space-y-4" @submit.prevent="onSubmit">
          <UFormField :label="$t('setup.vaultName')" :error="errors.name || undefined">
            <UInput v-model="form.name" :placeholder="$t('setup.vaultNamePlaceholder')" class="w-full" />
          </UFormField>

          <UFormField :label="$t('setup.vaultFilePath')" :error="errors.path || undefined">
            <UInput :model-value="form.path" readonly :placeholder="$t('setup.vaultFilePathPlaceholder')" class="w-full cursor-pointer" trailing-icon="i-ph-folder-open" @click="pickFolder" />
          </UFormField>

          <UFormField :label="$t('setup.protectionMethod')">
            <div class="grid grid-cols-2 gap-2">
              <button
                type="button"
                class="p-3 rounded-lg border text-left transition-colors"
                :class="form.cipher_kind === 'passphrase'
                  ? 'border-primary bg-primary/5 ring-1 ring-primary'
                  : 'border-default hover:border-primary/50'"
                @click="form.cipher_kind = 'passphrase'"
              >
                <div class="flex items-center gap-2 mb-1">
                  <UIcon name="i-ph-key" class="size-4" />
                  <span class="text-sm font-medium">{{ $t('setup.usePassphrase') }}</span>
                </div>
                <p class="text-xs text-muted">{{ $t('setup.usePassphraseDescription') }}</p>
              </button>
              <button
                type="button"
                class="p-3 rounded-lg border text-left transition-colors"
                :class="form.cipher_kind === 'symmetric'
                  ? 'border-primary bg-primary/5 ring-1 ring-primary'
                  : 'border-default hover:border-primary/50'"
                @click="form.cipher_kind = 'symmetric'"
              >
                <div class="flex items-center gap-2 mb-1">
                  <UIcon name="i-ph-fingerprint" class="size-4" />
                  <span class="text-sm font-medium">{{ $t('setup.useKeychain') }}</span>
                </div>
                <p class="text-xs text-muted">{{ $t('setup.useKeychainDescription') }}</p>
              </button>
            </div>
          </UFormField>

          <template v-if="form.cipher_kind === 'passphrase'">
            <UFormField :label="$t('setup.passphrase')" :error="errors.passphrase || undefined">
              <UInput v-model="form.passphrase" type="password" :placeholder="$t('setup.passphrasePlaceholder')" class="w-full" />
            </UFormField>

            <UFormField :label="$t('setup.confirmPassphrase')" :error="errors.confirmPassphrase || undefined">
              <UInput v-model="form.confirmPassphrase" type="password" :placeholder="$t('setup.confirmPassphrasePlaceholder')" class="w-full" />
            </UFormField>
          </template>

          <div class="flex gap-2 pt-2">
            <UButton to="/vaults" color="neutral" variant="outline" :label="$t('common.back')" class="flex-1" />
            <UButton type="submit" :label="$t('setup.createVault')" :loading="submitting" class="flex-1" />
          </div>
        </form>
      </UCard>
    </div>
  </div>
</template>
