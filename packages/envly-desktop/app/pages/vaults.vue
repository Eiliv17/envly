<script setup lang="ts">
import type { VaultEntrySummary } from '~/types/tauri'
import { open as openDialog } from '@tauri-apps/plugin-dialog'

definePageMeta({ layout: 'auth' })

const vaultStore = useVaultStore()
const toast = useToast()
const { t, locale, locales, setLocale } = useI18n()

const availableLocales = computed(() =>
  (locales.value as Array<{ code: string; name: string }>).map(l => ({
    label: l.name,
    value: l.code,
  })),
)

const loading = ref(true)

async function loadVaults() {
  loading.value = true
  try {
    await vaultStore.fetchVaults()
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  } finally {
    loading.value = false
  }
}

async function handleSelect(vault: VaultEntrySummary) {
  try {
    const status = await vaultStore.selectVault(vault.id)
    if (status === 'uninitialized') {
      toast.add({ title: t('vaults.fileMissing'), description: t('vaults.fileMissingDescription'), color: 'warning' })
      return
    }
    await navigateTo('/unlock')
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function handleDelete(vault: VaultEntrySummary) {
  try {
    await vaultStore.deleteVaultEntry(vault.id, false)
    toast.add({ title: t('vaults.removed'), description: t('vaults.removedDescription', { name: vault.name }), color: 'success' })
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

async function handleOpenFromFile() {
  try {
    const selected = await openDialog({
      title: t('vaults.openDialogTitle'),
      filters: [{ name: t('common.envlyVaultFilter'), extensions: ['envly'] }],
    })
    if (!selected) return

    const filePath = typeof selected === 'string' ? selected : String(selected)
    const fileName = filePath.split('/').pop()?.replace('.envly', '') || 'Imported Vault'

    await vaultStore.importVaultEntry({ name: fileName, path: filePath })
    toast.add({ title: t('vaults.imported'), description: t('vaults.importedDescription', { name: fileName }), color: 'success' })
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  }
}

onMounted(loadVaults)
</script>

<template>
  <div class="flex w-full min-h-screen">
    <!-- Left sidebar -->
    <aside class="w-64 shrink-0 border-r border-default bg-elevated flex flex-col">
      <div class="p-5 space-y-2">
        <div class="flex items-center gap-2">
          <UIcon name="i-ph-shield-check" class="size-5 text-primary" />
          <span class="font-bold text-lg">{{ $t('vaults.brand') }}</span>
        </div>
        <p class="text-xs text-muted">{{ $t('vaults.tagline') }}</p>
      </div>

      <nav class="flex-1 px-3 space-y-1">
        <UButton
          to="/setup"
          icon="i-ph-plus"
          :label="$t('vaults.createNew')"
          color="primary"
          variant="ghost"
          block
          class="justify-start"
        />
        <UButton
          icon="i-ph-folder-open"
          :label="$t('vaults.openFromFile')"
          color="neutral"
          variant="ghost"
          block
          class="justify-start"
          @click="handleOpenFromFile"
        />
      </nav>

      <div class="p-4 border-t border-default space-y-3">
        <p class="text-xs text-muted">{{ $t('vaults.encryptedNote') }}</p>
        <div class="flex items-center gap-2">
          <UIcon name="i-ph-translate" class="size-3.5 text-muted shrink-0" />
          <USelectMenu
            :model-value="locale"
            :items="availableLocales"
            value-key="value"
            class="flex-1"
            size="xs"
            @update:model-value="(val: string) => setLocale(val)"
          />
        </div>
      </div>
    </aside>

    <!-- Right panel -->
    <main class="flex-1 flex flex-col min-h-screen">
      <div class="p-6 border-b border-default">
        <h1 class="text-xl font-semibold">{{ $t('vaults.title') }}</h1>
        <p class="text-sm text-muted mt-1">{{ $t('vaults.subtitle') }}</p>
      </div>

      <div class="flex-1 overflow-y-auto p-6">
        <div v-if="loading" class="flex items-center justify-center h-40">
          <UIcon name="i-ph-spinner" class="animate-spin size-6" />
        </div>

        <div v-else-if="vaultStore.vaults.length === 0" class="flex flex-col items-center justify-center h-full text-center space-y-4 py-20">
          <UIcon name="i-ph-vault" class="size-12 text-muted" />
          <div class="space-y-1">
            <p class="font-medium">{{ $t('vaults.emptyTitle') }}</p>
            <p class="text-sm text-muted">{{ $t('vaults.emptyDescription') }}</p>
          </div>
        </div>

        <div v-else class="space-y-3 max-w-2xl">
          <VaultCard
            v-for="vault in vaultStore.vaults"
            :key="vault.id"
            :vault="vault"
            @select="handleSelect"
            @delete="handleDelete"
          />
        </div>
      </div>
    </main>
  </div>
</template>
