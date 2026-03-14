<script setup lang="ts">
import { save as saveDialog } from '@tauri-apps/plugin-dialog'

const vaultStore = useVaultStore()
const toast = useToast()
const { t, locale, locales, setLocale } = useI18n()

const availableLocales = computed(() =>
  (locales.value as Array<{ code: string; name: string }>).map(l => ({
    label: l.name,
    value: l.code,
  })),
)

const vaultName = ref('')
const vaultPath = ref('')

const editingName = ref(false)
const editName = ref('')

const newPassphrase = ref('')
const confirmPassphrase = ref('')
const changingPassphrase = ref(false)
const passphraseErrors = reactive({ newPassphrase: '', confirmPassphrase: '' })

const exportModalOpen = ref(false)
const exportPassphrase = ref('')
const exportConfirmPassphrase = ref('')
const exporting = ref(false)
const exportDestination = ref('')
const exportErrors = reactive({ passphrase: '', confirmPassphrase: '' })

const deleteModalOpen = ref(false)
const deleteConfirmPassphrase = ref('')
const deleting = ref(false)
const deleteErrors = reactive({ passphrase: '' })

const cipherKind = ref<'passphrase' | 'symmetric' | null>(null)
const migrateModalOpen = ref(false)
const migratePassphrase = ref('')
const migrateConfirmPassphrase = ref('')
const migrating = ref(false)
const migrateErrors = reactive({ passphrase: '', confirmPassphrase: '' })

const chosenLocale = computed({
  get: () => locale.value,
  set: (val: 'en' | 'it' | 'es') => setLocale(val),
})

watch(newPassphrase, () => { passphraseErrors.newPassphrase = '' })
watch(confirmPassphrase, () => { passphraseErrors.confirmPassphrase = '' })
watch(exportPassphrase, () => { exportErrors.passphrase = '' })
watch(exportConfirmPassphrase, () => { exportErrors.confirmPassphrase = '' })
watch(deleteConfirmPassphrase, () => { deleteErrors.passphrase = '' })
watch(migratePassphrase, () => { migrateErrors.passphrase = '' })
watch(migrateConfirmPassphrase, () => { migrateErrors.confirmPassphrase = '' })

onMounted(async () => {
  try {
    await vaultStore.fetchStatus()
    await vaultStore.fetchVaults()
    const current = vaultStore.currentVault()
    if (current) {
      vaultName.value = current.name
      vaultPath.value = current.path
    }
    if (vaultStore.activeVaultId) {
      cipherKind.value = await vaultStore.getActiveVaultCipherKind()
    }
  } catch {
    // ignored
  }
})

function startEditName() {
  editName.value = vaultName.value
  editingName.value = true
}

async function saveName() {
  if (!editName.value.trim() || !vaultStore.activeVaultId) return
  try {
    await vaultStore.renameVault(vaultStore.activeVaultId, editName.value.trim())
    vaultName.value = editName.value.trim()
    editingName.value = false
    toast.add({ title: t('settings.renamed'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function handleChangePassphrase() {
  passphraseErrors.newPassphrase = ''
  passphraseErrors.confirmPassphrase = ''
  let valid = true
  if (!newPassphrase.value) {
    passphraseErrors.newPassphrase = t('common.passphraseRequired')
    valid = false
  }
  if (newPassphrase.value && newPassphrase.value !== confirmPassphrase.value) {
    passphraseErrors.confirmPassphrase = t('common.passphraseMismatch')
    valid = false
  }
  if (!valid) return
  changingPassphrase.value = true
  try {
    await vaultStore.changePassphrase(newPassphrase.value)
    newPassphrase.value = ''
    confirmPassphrase.value = ''
    toast.add({ title: t('settings.passphraseChanged'), description: t('settings.passphraseChangedDescription'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  } finally {
    changingPassphrase.value = false
  }
}

function openMigrateModal() {
  migratePassphrase.value = ''
  migrateConfirmPassphrase.value = ''
  migrateErrors.passphrase = ''
  migrateErrors.confirmPassphrase = ''
  migrateModalOpen.value = true
}

async function handleMigrateToKeychain() {
  migrating.value = true
  try {
    await vaultStore.migrateCipher('symmetric')
    cipherKind.value = 'symmetric'
    migrateModalOpen.value = false
    toast.add({ title: t('settings.migrateCipherSuccess'), description: t('settings.migrateCipherSuccessDescription'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('settings.migrateCipherFailed'), description: String(e), color: 'error' })
  } finally {
    migrating.value = false
  }
}

async function handleMigrateToPassphrase() {
  migrateErrors.passphrase = ''
  migrateErrors.confirmPassphrase = ''
  let valid = true
  if (!migratePassphrase.value) {
    migrateErrors.passphrase = t('common.passphraseRequired')
    valid = false
  }
  if (migratePassphrase.value && migratePassphrase.value !== migrateConfirmPassphrase.value) {
    migrateErrors.confirmPassphrase = t('common.passphraseMismatch')
    valid = false
  }
  if (!valid) return

  migrating.value = true
  try {
    await vaultStore.migrateCipher('passphrase', migratePassphrase.value)
    cipherKind.value = 'passphrase'
    migrateModalOpen.value = false
    toast.add({ title: t('settings.migrateCipherSuccess'), description: t('settings.migrateCipherSuccessDescription'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('settings.migrateCipherFailed'), description: String(e), color: 'error' })
  } finally {
    migrating.value = false
  }
}

async function openExportDialog() {
  try {
    const dest = await saveDialog({
      title: t('settings.exportDialogTitle'),
      defaultPath: `${vaultName.value || 'vault'}.envly`,
      filters: [{ name: t('common.envlyVaultFilter'), extensions: ['envly'] }],
    })
    if (!dest) return
    exportDestination.value = typeof dest === 'string' ? dest : String(dest)
    exportPassphrase.value = ''
    exportConfirmPassphrase.value = ''
    exportModalOpen.value = true
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function handleExport() {
  exportErrors.passphrase = ''
  exportErrors.confirmPassphrase = ''
  let valid = true
  if (!exportPassphrase.value) {
    exportErrors.passphrase = t('common.passphraseRequired')
    valid = false
  }
  if (exportPassphrase.value && exportPassphrase.value !== exportConfirmPassphrase.value) {
    exportErrors.confirmPassphrase = t('common.passphraseMismatch')
    valid = false
  }
  if (!valid) return
  exporting.value = true
  try {
    await vaultStore.exportVault(exportDestination.value, exportPassphrase.value)
    exportModalOpen.value = false
    toast.add({ title: t('settings.exported'), description: t('settings.exportedDescription'), color: 'success' })
  } catch (e) {
    toast.add({ title: t('settings.exportFailed'), description: String(e), color: 'error' })
  } finally {
    exporting.value = false
  }
}

async function handleDelete() {
  deleteErrors.passphrase = ''
  if (!deleteConfirmPassphrase.value) {
    deleteErrors.passphrase = t('settings.confirmPassphraseRequired')
    return
  }
  if (!vaultStore.activeVaultId) return
  deleting.value = true
  try {
    await vaultStore.lockVault()
    await vaultStore.deleteVaultEntry(vaultStore.activeVaultId, true)
    deleteModalOpen.value = false
    toast.add({ title: t('settings.deleted'), color: 'success' })
    await navigateTo('/vaults')
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  } finally {
    deleting.value = false
  }
}
</script>

<template>
  <div class="space-y-6">
    <h1 class="text-xl font-semibold">{{ $t('settings.title') }}</h1>

    <!-- Vault Information -->
    <UCard>
      <template #header>
        <h2 class="font-semibold text-sm">{{ $t('settings.vaultInfo') }}</h2>
      </template>
      <div class="space-y-4">
        <div class="flex items-center justify-between">
          <div v-if="!editingName" class="flex items-center gap-2">
            <span class="text-sm text-muted">{{ $t('settings.nameLabel') }}</span>
            <span class="font-medium">{{ vaultName || $t('settings.unknown') }}</span>
            <UButton icon="i-ph-pencil-simple" color="neutral" variant="ghost" size="xs" @click="startEditName" />
          </div>
          <form v-else class="flex items-center gap-2" @submit.prevent="saveName">
            <UInput v-model="editName" size="sm" autofocus class="w-64" />
            <UButton type="submit" icon="i-ph-check" color="primary" variant="ghost" size="xs" />
            <UButton icon="i-ph-x" color="neutral" variant="ghost" size="xs" @click="editingName = false" />
          </form>
        </div>
        <div class="flex items-center gap-2">
          <span class="text-sm text-muted">{{ $t('settings.pathLabel') }}</span>
          <span class="text-sm font-mono truncate max-w-md">{{ vaultPath }}</span>
        </div>
      </div>
    </UCard>

    <!-- Protection Mode -->
    <UCard v-if="cipherKind">
      <template #header>
        <h2 class="font-semibold text-sm">{{ $t('settings.protectionMode') }}</h2>
      </template>
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <span class="text-sm text-muted">{{ $t('settings.currentMode') }}</span>
          <UBadge :color="cipherKind === 'passphrase' ? 'primary' : 'neutral'" variant="subtle">
            {{ cipherKind === 'passphrase' ? $t('settings.modePassphrase') : $t('settings.modeKeychain') }}
          </UBadge>
        </div>
        <UButton
          :icon="cipherKind === 'passphrase' ? 'i-ph-fingerprint' : 'i-ph-key'"
          :label="cipherKind === 'passphrase' ? $t('settings.migrateToKeychain') : $t('settings.migrateToPassphrase')"
          color="neutral"
          variant="outline"
          @click="openMigrateModal"
        />
      </div>
    </UCard>

    <!-- Migrate Cipher Modal -->
    <UModal v-model:open="migrateModalOpen">
      <template #content>
        <UCard>
          <template #header>
            <h3 class="font-semibold">
              {{ cipherKind === 'passphrase' ? $t('settings.migrateToKeychain') : $t('settings.migrateToPassphrase') }}
            </h3>
          </template>

          <!-- Migrate to Keychain (no passphrase needed) -->
          <div v-if="cipherKind === 'passphrase'" class="space-y-3">
            <p class="text-sm text-muted">{{ $t('settings.migrateToKeychainDescription') }}</p>
            <div class="flex justify-end gap-2 pt-2">
              <UButton :label="$t('common.cancel')" color="neutral" variant="outline" @click="migrateModalOpen = false" />
              <UButton :label="$t('settings.migrateConfirmKeychain')" :loading="migrating" @click="handleMigrateToKeychain" />
            </div>
          </div>

          <!-- Migrate to Passphrase (needs new passphrase) -->
          <form v-else class="space-y-3" @submit.prevent="handleMigrateToPassphrase">
            <p class="text-sm text-muted">{{ $t('settings.migrateToPassphraseDescription') }}</p>
            <UFormField :label="$t('settings.newPassphrase')" :error="migrateErrors.passphrase || undefined">
              <UInput v-model="migratePassphrase" type="password" :placeholder="$t('settings.newPassphrasePlaceholder')" class="w-full" />
            </UFormField>
            <UFormField :label="$t('settings.confirmPassphrase')" :error="migrateErrors.confirmPassphrase || undefined">
              <UInput v-model="migrateConfirmPassphrase" type="password" :placeholder="$t('settings.confirmPassphrasePlaceholder')" class="w-full" />
            </UFormField>
            <div class="flex justify-end gap-2 pt-2">
              <UButton :label="$t('common.cancel')" color="neutral" variant="outline" @click="migrateModalOpen = false" />
              <UButton type="submit" :label="$t('settings.migrateToPassphrase')" :loading="migrating" />
            </div>
          </form>
        </UCard>
      </template>
    </UModal>

    <!-- Change Passphrase -->
    <UCard v-if="cipherKind === 'passphrase'">
      <template #header>
        <h2 class="font-semibold text-sm">{{ $t('settings.changePassphrase') }}</h2>
      </template>
      <form class="space-y-3" @submit.prevent="handleChangePassphrase">
        <UFormField :label="$t('settings.newPassphrase')" :error="passphraseErrors.newPassphrase || undefined">
          <UInput v-model="newPassphrase" type="password" :placeholder="$t('settings.newPassphrasePlaceholder')" class="w-full" />
        </UFormField>
        <UFormField :label="$t('settings.confirmPassphrase')" :error="passphraseErrors.confirmPassphrase || undefined">
          <UInput v-model="confirmPassphrase" type="password" :placeholder="$t('settings.confirmPassphrasePlaceholder')" class="w-full" />
        </UFormField>
        <UButton type="submit" :label="$t('settings.changePassphraseBtn')" :loading="changingPassphrase" />
      </form>
    </UCard>

    <!-- Export Vault -->
    <UCard>
      <template #header>
        <h2 class="font-semibold text-sm">{{ $t('settings.exportVault') }}</h2>
      </template>
      <div class="flex items-center justify-between">
        <p class="text-sm text-muted">{{ $t('settings.exportDescription') }}</p>
        <UButton icon="i-ph-download-simple" :label="$t('common.export')" color="neutral" variant="outline" @click="openExportDialog" />
      </div>
    </UCard>

    <!-- Export Passphrase Modal -->
    <UModal v-model:open="exportModalOpen">
      <template #content>
        <UCard>
          <template #header>
            <h3 class="font-semibold">{{ $t('settings.setExportPassphrase') }}</h3>
          </template>
          <form class="space-y-3" @submit.prevent="handleExport">
            <p class="text-sm text-muted">{{ $t('settings.exportPassphraseHint') }}</p>
            <UFormField :label="$t('settings.exportPassphrase')" :error="exportErrors.passphrase || undefined">
              <UInput v-model="exportPassphrase" type="password" :placeholder="$t('settings.exportPassphrasePlaceholder')" class="w-full" />
            </UFormField>
            <UFormField :label="$t('settings.confirmPassphrase')" :error="exportErrors.confirmPassphrase || undefined">
              <UInput v-model="exportConfirmPassphrase" type="password" :placeholder="$t('settings.exportConfirmPassphrasePlaceholder')" class="w-full" />
            </UFormField>
            <div class="flex justify-end gap-2 pt-2">
              <UButton :label="$t('common.cancel')" color="neutral" variant="outline" @click="exportModalOpen = false" />
              <UButton type="submit" :label="$t('common.export')" :loading="exporting" />
            </div>
          </form>
        </UCard>
      </template>
    </UModal>

    <!-- Language -->
    <UCard>
      <template #header>
        <h2 class="font-semibold text-sm">{{ $t('settings.language') }}</h2>
      </template>
      <div class="flex items-center justify-between">
        <p class="text-sm text-muted">{{ $t('settings.languageDescription') }}</p>
        <USelectMenu
          v-model="chosenLocale"
          :items="availableLocales"
          value-key="value"
          class="w-44"
          size="sm"
        />
      </div>
    </UCard>

    <!-- Danger Zone -->
    <UCard class="ring-error/50 ring-1">
      <template #header>
        <h2 class="font-semibold text-sm text-error">{{ $t('settings.dangerZone') }}</h2>
      </template>
      <div class="flex items-center justify-between">
        <div>
          <p class="font-medium text-sm">{{ $t('settings.deleteVault') }}</p>
          <p class="text-sm text-muted">{{ $t('settings.deleteDescription') }}</p>
        </div>
        <UButton
          icon="i-ph-trash"
          :label="$t('settings.deleteVault')"
          color="error"
          variant="outline"
          @click="deleteConfirmPassphrase = ''; deleteModalOpen = true"
        />
      </div>
    </UCard>

    <!-- Delete Confirmation Modal -->
    <UModal v-model:open="deleteModalOpen">
      <template #content>
        <UCard>
          <template #header>
            <h3 class="font-semibold text-error">{{ $t('settings.deleteVault') }}</h3>
          </template>
          <form class="space-y-3" @submit.prevent="handleDelete">
            <p class="text-sm text-muted">
              {{ $t('settings.deleteConfirmText', { name: vaultName }) }}
            </p>
            <UFormField :label="$t('settings.currentPassphrase')" :error="deleteErrors.passphrase || undefined">
              <UInput v-model="deleteConfirmPassphrase" type="password" :placeholder="$t('settings.currentPassphrasePlaceholder')" class="w-full" />
            </UFormField>
            <div class="flex justify-end gap-2 pt-2">
              <UButton :label="$t('common.cancel')" color="neutral" variant="outline" @click="deleteModalOpen = false" />
              <UButton type="submit" :label="$t('settings.deletePermanently')" color="error" :loading="deleting" />
            </div>
          </form>
        </UCard>
      </template>
    </UModal>
  </div>
</template>
