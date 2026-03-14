<script setup lang="ts">
import type { NavigationMenuItem, DropdownMenuItem } from '@nuxt/ui'

const route = useRoute()
const colorMode = useColorMode()
const vaultStore = useVaultStore()
const toast = useToast()
const { t } = useI18n()

const navItems = computed<NavigationMenuItem[]>(() => [
  {
    label: t('nav.dashboard'),
    icon: 'i-ph-squares-four',
    to: '/dashboard',
    active: route.path === '/dashboard',
  },
  {
    label: t('nav.secrets'),
    icon: 'i-ph-key',
    to: '/secrets',
    active: route.path === '/secrets',
  },
  {
    label: t('nav.projects'),
    icon: 'i-ph-kanban',
    to: '/projects',
    active: route.path.startsWith('/projects'),
  },
  {
    label: t('nav.settings'),
    icon: 'i-ph-gear',
    to: '/settings',
    active: route.path === '/settings',
  },
])

const themeItems = computed<DropdownMenuItem[][]>(() => [
  [
    {
      label: t('theme.light'),
      icon: 'i-ph-sun',
      type: 'checkbox' as const,
      checked: colorMode.preference === 'light',
      onSelect() { colorMode.preference = 'light' },
    },
    {
      label: t('theme.dark'),
      icon: 'i-ph-moon',
      type: 'checkbox' as const,
      checked: colorMode.preference === 'dark',
      onSelect() { colorMode.preference = 'dark' },
    },
    {
      label: t('theme.system'),
      icon: 'i-ph-monitor',
      type: 'checkbox' as const,
      checked: colorMode.preference === 'system',
      onSelect() { colorMode.preference = 'system' },
    },
  ],
])

const vaultItems = computed<DropdownMenuItem[][]>(() => [
  [
    {
      label: t('vault.lock'),
      icon: 'i-ph-lock',
      async onSelect() {
        try {
          await vaultStore.lockVault()
          await navigateTo('/unlock')
        } catch (e) {
          toast.add({ title: t('common.error'), description: String(e), color: 'error' })
        }
      },
    },
    {
      label: t('vault.switch'),
      icon: 'i-ph-arrows-clockwise',
      async onSelect() {
        try {
          await vaultStore.lockVault()
          await navigateTo('/vaults')
        } catch (e) {
          toast.add({ title: t('common.error'), description: String(e), color: 'error' })
        }
      },
    },
  ],
])

const themeIcon = computed(() => {
  if (colorMode.preference === 'dark') return 'i-ph-moon'
  if (colorMode.preference === 'light') return 'i-ph-sun'
  return 'i-ph-monitor'
})
</script>

<template>
  <div class="h-screen flex flex-col overflow-hidden">
    <header class="shrink-0 z-50 border-b border-default bg-default/75 backdrop-blur">
      <div class="max-w-6xl mx-auto flex items-center h-14 px-4 gap-6">
        <NuxtLink to="/dashboard" class="flex items-center gap-2 shrink-0">
          <UIcon name="i-ph-shield-check" class="size-5 text-primary" />
          <span class="font-bold">Envly</span>
        </NuxtLink>

        <UNavigationMenu :items="navItems" class="flex-1" />

        <div class="flex items-center gap-1 shrink-0">
          <UDropdownMenu :items="themeItems">
            <UButton :icon="themeIcon" color="neutral" variant="ghost" size="sm" />
          </UDropdownMenu>

          <UDropdownMenu :items="vaultItems">
            <UButton icon="i-ph-vault" color="neutral" variant="ghost" size="sm" />
          </UDropdownMenu>
        </div>
      </div>
    </header>

    <main class="flex-1 overflow-y-auto p-6">
      <div class="max-w-6xl mx-auto">
        <slot />
      </div>
    </main>
  </div>
</template>
