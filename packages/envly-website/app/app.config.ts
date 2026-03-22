export default defineAppConfig({
  ui: {
    colors: {
      primary: 'purple',
      neutral: 'zinc'
    },
    footer: {
      slots: {
        root: 'border-t border-default',
        left: 'text-sm text-muted'
      }
    }
  },
  navigation: {
    links: [
      {
        labelTranslationKey: 'navigation.downloads',
        icon: 'ph:folder-open',
        to: '/downloads'
      },
      {
        labelTranslationKey: 'navigation.docs',
        icon: 'ph:file-text',
        to: '/docs'
      }
    ]
  },
  seo: {
    title: 'Envly',
    description: 'Secrets and environment variables manager'
  },
  link: [
    { rel: 'icon', href: '/favicon.ico' },
    { rel: 'icon', type: 'image/png', sizes: '32x32', href: '/favicon-32x32.png' },
    { rel: 'icon', type: 'image/png', sizes: '16x16', href: '/favicon-16x16.png' },
    { rel: 'apple-touch-icon', sizes: '180x180', href: '/apple-touch-icon.png' }
  ],
  header: {
    title: '',
    to: '/',
    logo: {
      alt: '',
      light: '',
      dark: ''
    },
    search: true,
    colorMode: true,
    links: [{
      'icon': 'i-simple-icons-github',
      'to': 'https://github.com/nuxt-ui-templates/docs',
      'target': '_blank',
      'aria-label': 'GitHub'
    }]
  },
  footer: {
    credits: `Copyright © ${new Date().getFullYear()} Envly`,
    colorMode: false,
    links: [
      {
        'icon': 'i-simple-icons-discord',
        'to': 'https://go.nuxt.com/discord',
        'target': '_blank',
        'aria-label': 'Nuxt on Discord'
      },
      {
        'icon': 'i-simple-icons-x',
        'to': 'https://go.nuxt.com/x',
        'target': '_blank',
        'aria-label': 'Nuxt on X'
      },
      {
        'icon': 'i-simple-icons-github',
        'to': 'https://github.com/nuxt/ui',
        'target': '_blank',
        'aria-label': 'Nuxt UI on GitHub'
      }
    ]
  },
  toc: {
    title: 'Table of Contents',
    bottom: {
      title: 'Community',
      edit: 'https://github.com/nuxt-ui-templates/docs/edit/main/content',
      links: [{
        icon: 'i-lucide-star',
        label: 'Star on GitHub',
        to: 'https://github.com/nuxt/ui',
        target: '_blank'
      }, {
        icon: 'i-lucide-book-open',
        label: 'Nuxt UI docs',
        to: 'https://ui.nuxt.com/docs/getting-started/installation/nuxt',
        target: '_blank'
      }]
    }
  }
})
