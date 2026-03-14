<script setup lang="ts">
definePageMeta({ layout: 'auth' })

const vaultStore = useVaultStore()

onMounted(async () => {
  try {
    await vaultStore.fetchVaults()
    if (vaultStore.vaults.length === 0) {
      return navigateTo('/vaults')
    }

    await vaultStore.fetchStatus()
    if (!vaultStore.activeVaultId) {
      return navigateTo('/vaults')
    }

    if (vaultStore.status === 'unlocked') {
      return navigateTo('/dashboard')
    }
    return navigateTo('/unlock')
  } catch {
    return navigateTo('/vaults')
  }
})
</script>

<template>
  <div class="flex items-center justify-center h-32">
    <UIcon name="i-ph-spinner" class="animate-spin size-6" />
  </div>
</template>
