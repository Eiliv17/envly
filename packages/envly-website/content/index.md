---
seo:
  title: Envly - Secrets and Environment Variables Manager
  description: A desktop app to centrally manage secrets, organize them across projects and environments, and safely inject them into your local development workflow.
---

::u-page-hero{class="dark:bg-gradient-to-b from-neutral-900 to-neutral-950"}
---
orientation: horizontal
---
#top
:hero-background

#title
Your Secrets, [Encrypted]{.text-primary} and Organized.

#description
Envly is a desktop secrets and environment variable manager. Centrally manage secrets in encrypted vaults, organize them across projects and environments, and safely inject them into your local development workflow using symlinks.

#links
  :::u-button
  ---
  to: /downloads
  size: xl
  trailing-icon: i-lucide-download
  ---
  Download
  :::

  :::u-button
  ---
  icon: i-lucide-book-open
  color: neutral
  variant: outline
  size: xl
  to: /getting-started
  ---
  Go to Docs
  :::

#default
  :::div{class="flex items-center justify-center w-full h-full min-h-[280px] rounded-xl border border-default bg-elevated/50 backdrop-blur p-6"}
  <img src="/product-placeholder.svg" alt="Envly Application Screenshot" class="rounded-lg max-w-full max-h-[320px] object-contain" />
  :::
::

::u-page-section{class="dark:bg-neutral-950"}
#title
Everything you need to manage secrets

#features
  :::u-page-feature
  ---
  icon: i-lucide-shield-check
  ---
  #title
  Encrypted Vaults

  #description
  All secrets are stored in encrypted vault files using XChaCha20-Poly1305 AEAD encryption. Choose between a master passphrase or OS keychain for authentication.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-folder-git-2
  ---
  #title
  Multi-Project Management

  #description
  Organize secrets across multiple projects with per-environment mappings. Each project points to a local repository and supports multiple environments like dev, staging, and production.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-link
  ---
  #title
  Symlink-Based Injection

  #description
  Secrets are injected via symlinks to secure temp files. If accidentally committed to git, they're just broken links — no secret data is ever leaked.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-clock
  ---
  #title
  Expiration Tracking

  #description
  Set expiry dates on secrets with automatic UI warnings as deadlines approach. Encourage regular secret rotation without the hassle.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-monitor-smartphone
  ---
  #title
  Cross-Platform

  #description
  Built with Tauri 2, Envly runs natively on macOS (Apple Silicon and Intel), Windows, and Linux with a small footprint and fast performance.
  :::

  :::u-page-feature
  ---
  icon: i-lucide-scale
  ---
  #title
  Open Source

  #description
  Envly is fully open source under the Apache 2.0 license. Inspect the code, contribute, or self-host with confidence.
  :::
::
