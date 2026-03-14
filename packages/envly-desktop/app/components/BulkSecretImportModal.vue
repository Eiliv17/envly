<script setup lang="ts">
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import type { ParsedEnvEntry, SecretSummary } from '~/types/tauri'

const props = defineProps<{
  existingSecrets: SecretSummary[]
}>()

const open = defineModel<boolean>('open', { default: false })
const emit = defineEmits<{ imported: [] }>()

const secretsStore = useSecretsStore()
const toast = useToast()
const { t } = useI18n()

type Mode = 'file' | 'paste'
const mode = ref<Mode>('file')
const pasteText = ref('')
const entries = ref<(ParsedEnvEntry & { selected: boolean; duplicate: boolean })[]>([])
const revealedRows = ref<Set<number>>(new Set())
const importing = ref(false)

const existingKeys = computed(() => new Set(props.existingSecrets.map(s => s.key)))

const selectedCount = computed(() => entries.value.filter(e => e.selected).length)

watch(open, (val) => {
  if (val) {
    mode.value = 'file'
    pasteText.value = ''
    entries.value = []
    revealedRows.value = new Set()
  }
})

function processEntries(raw: ParsedEnvEntry[]) {
  entries.value = raw.map(e => ({
    ...e,
    selected: !existingKeys.value.has(e.key),
    duplicate: existingKeys.value.has(e.key),
  }))
}

async function browseFile() {
  const selected = await openDialog({
    title: t('bulkImport.browseTitle'),
    filters: [{ name: 'Env files', extensions: ['env', 'env.*', '*'] }],
  })
  if (!selected) return
  try {
    const parsed = await secretsStore.parseEnvFile(String(selected))
    processEntries(parsed)
  } catch (e) {
    toast.add({ title: t('bulkImport.parseError'), description: String(e), color: 'error' })
  }
}

function parsePastedText() {
  const lines = pasteText.value
  const parsed: ParsedEnvEntry[] = []
  for (const line of lines.split('\n')) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#')) continue
    const body = trimmed.startsWith('export ') ? trimmed.slice(7).trim() : trimmed
    const eq = body.indexOf('=')
    if (eq === -1) continue
    const key = body.substring(0, eq).trim()
    let value = body.substring(eq + 1).trim()
    if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1)
    }
    if (key) parsed.push({ key, value })
  }
  processEntries(parsed)
}

function toggleAll(checked: boolean) {
  entries.value.forEach(e => { e.selected = checked })
}

function toggleReveal(idx: number) {
  if (revealedRows.value.has(idx)) {
    revealedRows.value.delete(idx)
  } else {
    revealedRows.value.add(idx)
  }
  revealedRows.value = new Set(revealedRows.value)
}

async function doImport() {
  const toImport = entries.value.filter(e => e.selected).map(e => ({ key: e.key, value: e.value }))
  if (toImport.length === 0) return
  importing.value = true
  try {
    const result = await secretsStore.bulkCreateSecrets(toImport)
    toast.add({
      title: t('bulkImport.resultTitle'),
      description: t('bulkImport.resultDescription', { created: result.created, updated: result.updated }),
      color: 'success',
    })
    open.value = false
    emit('imported')
  } catch (e) {
    toast.add({ title: t('common.error'), description: String(e), color: 'error' })
  } finally {
    importing.value = false
  }
}
</script>

<template>
  <UModal v-model:open="open" :title="$t('bulkImport.title')" :ui="{ width: 'sm:max-w-2xl' }">
    <template #body>
      <div class="space-y-4">
        <div class="flex gap-2">
          <UButton
            :label="$t('bulkImport.fromFile')"
            :variant="mode === 'file' ? 'solid' : 'outline'"
            color="neutral"
            size="sm"
            @click="mode = 'file'"
          />
          <UButton
            :label="$t('bulkImport.paste')"
            :variant="mode === 'paste' ? 'solid' : 'outline'"
            color="neutral"
            size="sm"
            @click="mode = 'paste'"
          />
        </div>

        <div v-if="mode === 'file'" class="space-y-2">
          <UButton :label="$t('bulkImport.browse')" icon="i-ph-folder-open" variant="outline" @click="browseFile" />
        </div>

        <div v-else class="space-y-2">
          <UTextarea
            v-model="pasteText"
            :placeholder="$t('bulkImport.pastePlaceholder')"
            :rows="6"
            class="w-full font-mono text-xs"
          />
          <UButton :label="$t('bulkImport.parseBtn')" size="sm" @click="parsePastedText" />
        </div>

        <div v-if="entries.length > 0" class="space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-sm font-medium">{{ $t('bulkImport.preview') }} ({{ entries.length }})</span>
            <div class="flex gap-2">
              <UButton :label="$t('bulkImport.selectAll')" size="xs" variant="ghost" @click="toggleAll(true)" />
              <UButton :label="$t('bulkImport.deselectAll')" size="xs" variant="ghost" @click="toggleAll(false)" />
            </div>
          </div>

          <div class="border border-default rounded-lg overflow-hidden">
            <table class="w-full text-sm">
              <thead class="bg-elevated text-left">
                <tr>
                  <th class="px-3 py-2 w-8" />
                  <th class="px-3 py-2">{{ $t('bulkImport.keyCol') }}</th>
                  <th class="px-3 py-2">{{ $t('bulkImport.valueCol') }}</th>
                  <th class="px-3 py-2 w-24">{{ $t('bulkImport.statusCol') }}</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-default">
                <tr v-for="(entry, idx) in entries" :key="idx" class="hover:bg-elevated/50">
                  <td class="px-3 py-2">
                    <input v-model="entry.selected" type="checkbox" class="rounded" />
                  </td>
                  <td class="px-3 py-2 font-mono text-xs">{{ entry.key }}</td>
                  <td class="px-3 py-2 font-mono text-xs">
                    <span
                      class="cursor-pointer"
                      @click="toggleReveal(idx)"
                    >{{ revealedRows.has(idx) ? entry.value : '••••••••' }}</span>
                  </td>
                  <td class="px-3 py-2">
                    <UBadge v-if="entry.duplicate && entry.selected" color="info" variant="subtle" size="xs">
                      {{ $t('bulkImport.overwrite') }}
                    </UBadge>
                    <UBadge v-else-if="entry.duplicate" color="warning" variant="subtle" size="xs">
                      {{ $t('bulkImport.duplicate') }}
                    </UBadge>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="flex justify-end pt-2">
            <UButton
              :label="$t('bulkImport.importSelected', { n: selectedCount })"
              icon="i-ph-download-simple"
              :loading="importing"
              :disabled="selectedCount === 0"
              @click="doImport"
            />
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>
