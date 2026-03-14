<script setup lang="ts">
import type { VaultEntrySummary } from '~/types/tauri'

const props = defineProps<{
  vault: VaultEntrySummary
}>()

const emit = defineEmits<{
  select: [vault: VaultEntrySummary]
  delete: [vault: VaultEntrySummary]
}>()

const { t } = useI18n()

const formattedDate = computed(() => {
  if (!props.vault.last_accessed) return t('vaultCard.neverAccessed')
  return t('vaultCard.lastUsed', { date: new Date(props.vault.last_accessed).toLocaleDateString() })
})
</script>

<template>
  <UCard class="cursor-pointer hover:ring-2 hover:ring-primary transition-shadow" @click="emit('select', vault)">
    <div class="flex items-center justify-between">
      <div class="space-y-1">
        <h3 class="font-semibold">{{ vault.name }}</h3>
        <p class="text-xs text-muted font-mono truncate max-w-xs">{{ vault.path }}</p>
        <p class="text-xs text-muted">{{ formattedDate }}</p>
      </div>
      <div class="flex items-center gap-2" @click.stop>
        <UBadge :color="vault.cipher_kind === 'passphrase' ? 'primary' : 'neutral'" variant="subtle" size="xs">
          {{ vault.cipher_kind }}
        </UBadge>
        <UButton icon="i-ph-trash" color="error" variant="ghost" size="xs" @click="emit('delete', vault)" />
      </div>
    </div>
  </UCard>
</template>
