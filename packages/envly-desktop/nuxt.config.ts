export default defineNuxtConfig({
  ssr: false,
  modules: ['@nuxt/ui', '@nuxtjs/i18n', '@pinia/nuxt'],
  css: ['~/assets/css/main.css'],

  i18n: {
    locales: [
      { code: 'en', name: 'English', file: 'en.json' },
      { code: 'it', name: 'Italiano', file: 'it.json' },
      { code: 'es', name: 'Español', file: 'es.json' },
    ],
    defaultLocale: 'en',
    langDir: 'locales/',
    strategy: 'no_prefix',
  },

  devServer: {
    host: process.env.TAURI_DEV_HOST || 'localhost',
    port: 1420,
  },

  vite: {
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    server: {
      strictPort: true,
      hmr: {
        protocol: 'ws',
        host: process.env.TAURI_DEV_HOST || 'localhost',
        port: 1421,
      },
    },
  },

  ignore: ['**/src-tauri/**'],

  compatibilityDate: '2025-05-15',
})
