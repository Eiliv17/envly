<script setup lang="ts">
useSeoMeta({
  title: 'Downloads',
  ogTitle: 'Download Envly',
  description: 'Download Envly for macOS, Windows, or Linux.',
  ogDescription: 'Download Envly for macOS, Windows, or Linux.'
})

const { data } = await useAsyncData('downloads', () => queryCollection('downloads').first())

const releasePageUrl = computed(() => data.value?.release_page_url ?? 'https://github.com/eiliv17/envly/releases')

const osConfig: Record<string, { label: string, icon: string }> = {
  macos: { label: 'macOS', icon: 'i-simple-icons-apple' },
  windows: { label: 'Windows', icon: 'i-simple-icons-windows' },
  linux: { label: 'Linux', icon: 'i-simple-icons-linux' }
}

function downloadUrl(filename: string): string {
  return `${releasePageUrl.value}/latest/download/${filename}`
}

const platforms = computed(() => {
  if (!data.value?.items) return []

  const grouped = new Map<string, typeof data.value.items>()
  for (const item of data.value.items) {
    const list = grouped.get(item.os) ?? []
    list.push(item)
    grouped.set(item.os, list)
  }

  return Array.from(grouped.entries()).map(([os, items]) => ({
    ...osConfig[os],
    os,
    items
  }))
})
</script>

<template>
  <UContainer class="py-16">
    <div class="max-w-3xl mx-auto">
      <div class="text-center mb-12">
        <h1 class="text-4xl font-bold tracking-tight sm:text-5xl">
          Downloads
        </h1>
        <p class="mt-4 text-lg text-muted">
          Download the latest release, or browse
          <ULink
            :to="releasePageUrl"
            target="_blank"
            class="text-primary hover:underline"
          >
            all releases on GitHub
          </ULink>
        </p>
      </div>

      <div class="space-y-10">
        <div
          v-for="platform in platforms"
          :key="platform.os"
        >
          <div class="flex items-center gap-2 mb-4">
            <UIcon
              :name="platform.icon"
              class="size-5"
            />
            <h2 class="text-xl font-semibold">
              {{ platform.label }}
            </h2>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <ULink
              v-for="item in platform.items"
              :key="item.name"
              :to="downloadUrl(item.filename)"
              target="_blank"
              class="block rounded-xl border border-default bg-elevated p-5 transition-colors hover:bg-elevated/80"
            >
              <div class="font-medium">
                {{ item.name }}
              </div>
              <div class="mt-1 text-sm text-muted">
                .{{ item.file_extension }}
              </div>
            </ULink>
          </div>
        </div>
      </div>

      <p class="mt-12 text-center text-sm text-muted">
        Looking for older versions? Check the
        <ULink
          :to="releasePageUrl"
          target="_blank"
          class="text-primary hover:underline"
        >
          GitHub releases page
        </ULink>
      </p>
    </div>
  </UContainer>
</template>
