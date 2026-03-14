export default defineNuxtConfig({
  ssr: false,
  modules: ['@nuxt/ui', '@nuxtjs/i18n', '@pinia/nuxt'],

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
