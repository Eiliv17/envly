<script setup lang="ts">
definePageMeta({ layout: 'auth' })

const vaultStore = useVaultStore()
const toast = useToast()
const { t } = useI18n()

const passphrase = ref('')
const submitting = ref(false)
const vaultName = ref('')
const cipherKind = ref<'passphrase' | 'symmetric'>('passphrase')

onMounted(async () => {
  try {
    await vaultStore.fetchStatus()
    if (vaultStore.status === 'unlocked') {
      return navigateTo('/dashboard')
    }
    if (vaultStore.status === 'uninitialized') {
      toast.add({ title: t('vaults.fileMissing'), description: t('vaults.fileMissingDescription'), color: 'warning' })
      return navigateTo('/vaults')
    }

    if (!vaultStore.activeVaultId) {
      return navigateTo('/vaults')
    }

    await vaultStore.fetchVaults()
    const current = vaultStore.currentVault()
    if (current) {
      vaultName.value = current.name
      cipherKind.value = current.cipher_kind
    }
  } catch {
    return navigateTo('/vaults')
  }
})

async function unlockKeychain() {
  submitting.value = true
  try {
    await vaultStore.unlockVault()
    await navigateTo('/dashboard')
  } catch (e) {
    toast.add({ title: t('unlock.keychainFailed'), description: String(e), color: 'error' })
  } finally {
    submitting.value = false
  }
}

async function onSubmit() {
  if (!passphrase.value) return

  submitting.value = true
  try {
    await vaultStore.unlockVault(passphrase.value)
    await navigateTo('/dashboard')
  } catch (e) {
    toast.add({ title: t('unlock.failed'), description: String(e), color: 'error' })
    passphrase.value = ''
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="flex-1 flex items-center justify-center p-6">
    <div class="w-full max-w-md space-y-6">
      <div class="text-center space-y-2">
        <h1 class="text-2xl font-bold">{{ $t('unlock.title') }}</h1>
        <p v-if="vaultName" class="text-sm text-muted">
          {{ $t('unlock.subtitle', { name: vaultName }) }}
        </p>
      </div>

      <!-- Symmetric: unlock via keychain button -->
      <UCard v-if="cipherKind === 'symmetric'">
        <div class="flex flex-col items-center gap-4 py-4">
          <UIcon name="i-ph-fingerprint" class="size-10 text-primary" />
          <p class="text-sm text-muted">{{ $t('unlock.unlockingKeychain') }}</p>
          <UButton icon="i-ph-lock-open" :label="$t('unlock.unlock')" :loading="submitting" @click="unlockKeychain" />
        </div>
        <div class="text-center pt-2">
          <UButton to="/vaults" variant="link" color="neutral" :label="$t('unlock.switchVault')" size="sm" />
        </div>
      </UCard>

      <!-- Passphrase: manual entry -->
      <UCard v-else>
        <form class="space-y-4" @submit.prevent="onSubmit">
          <UFormField :label="$t('unlock.passphrase')">
            <UInput
              v-model="passphrase"
              type="password"
              :placeholder="$t('unlock.passphrasePlaceholder')"
              autofocus
              class="w-full"
            />
          </UFormField>

          <UButton type="submit" :label="$t('unlock.unlock')" :loading="submitting" block />

          <div class="text-center">
            <UButton to="/vaults" variant="link" color="neutral" :label="$t('unlock.switchVault')" size="sm" />
          </div>
        </form>
      </UCard>
    </div>
  </div>
</template>
