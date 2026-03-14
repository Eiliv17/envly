import { listen } from '@tauri-apps/api/event'

export function useVaultEvents() {
  const secretsStore = useSecretsStore()
  const projectsStore = useProjectsStore()
  const environmentsStore = useEnvironmentsStore()
  const vaultStore = useVaultStore()

  let unlisteners: (() => void)[] = []

  onMounted(async () => {
    unlisteners = await Promise.all([
      listen('vault:secrets-changed', () => {
        if (vaultStore.status === 'unlocked') {
          secretsStore.fetchSecrets()
        }
      }),
      listen('vault:projects-changed', () => {
        if (vaultStore.status === 'unlocked') {
          projectsStore.fetchProjects()
        }
      }),
      listen('vault:environments-changed', () => {
        if (vaultStore.status === 'unlocked') {
          environmentsStore.refetchLoaded()
          projectsStore.fetchProjects()
        }
      }),
      listen('vault:status-changed', () => {
        vaultStore.fetchStatus()
        vaultStore.fetchVaults()
      }),
    ])
  })

  onUnmounted(() => {
    unlisteners.forEach(fn => fn())
    unlisteners = []
  })
}
