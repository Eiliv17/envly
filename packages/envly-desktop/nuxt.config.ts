export default defineNuxtConfig({
  ssr: false,
  modules: ['@nuxt/ui', '@nuxtjs/i18n', '@pinia/nuxt'],

  devServer: {
    host: '0.0.0.0',
    port: 1420,
  },

  vite: {
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    server: {
      strictPort: true,
    },
  },

  ignore: ['**/src-tauri/**'],

  compatibilityDate: '2025-05-15',
})
