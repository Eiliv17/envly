export default defineNuxtRouteMiddleware(async (to) => {
  const vaultStore = useVaultStore()

  if (to.path === '/vaults' || to.path === '/setup') {
    return
  }

  try {
    await vaultStore.fetchVaults()
    if (vaultStore.vaults.length === 0) {
      return navigateTo('/vaults')
    }

    await vaultStore.fetchStatus()
    if (!vaultStore.activeVaultId) {
      return navigateTo('/vaults')
    }

    if (vaultStore.status === 'uninitialized') {
      return navigateTo('/vaults')
    }

    if (vaultStore.status === 'locked') {
      if (to.path !== '/unlock') {
        return navigateTo('/unlock')
      }
      return
    }

    if (to.path === '/unlock') {
      return navigateTo('/dashboard')
    }
  } catch {
    return navigateTo('/vaults')
  }
})
