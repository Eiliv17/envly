import { defineContentConfig, defineCollection, z } from '@nuxt/content'

export default defineContentConfig({
  collections: {
    landing: defineCollection({
      type: 'page',
      source: 'index.md'
    }),
    docs: defineCollection({
      type: 'page',
      source: {
        include: '**',
        exclude: ['index.md', 'downloads.yml']
      },
      schema: z.object({
        links: z.array(z.object({
          label: z.string(),
          icon: z.string(),
          to: z.string(),
          target: z.string().optional()
        })).optional()
      })
    }),
    downloads: defineCollection({
      type: 'data',
      source: 'downloads.yml',
      schema: z.object({
        release_page_url: z.string().url(),
        items: z.array(z.object({
          name: z.string().nonempty(),
          os: z.enum(['macos', 'windows', 'linux']),
          file_extension: z.enum(['dmg', 'exe', 'deb', 'rpm', 'tar.gz', 'zip', 'appimage']),
          filename: z.string().nonempty()
        }))
      })
    })
  }
})
