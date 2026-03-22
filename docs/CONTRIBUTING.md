# Contributing to Envly

Thanks for your interest in contributing to Envly! This guide covers everything you need to get started.

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) >= 18
- [pnpm](https://pnpm.io/) >= 10
- Platform-specific Tauri v2 dependencies ([see Tauri docs](https://v2.tauri.app/start/prerequisites/))

## Getting Started

1. **Fork** the repository on GitHub
2. **Clone** your fork:

   ```bash
   git clone https://github.com/<your-username>/envly.git
   cd envly
   pnpm install
   ```

3. **Create a branch** for your change:

   ```bash
   git checkout -b feature/my-change
   ```

4. **Run in development mode:**

   ```bash
   # Desktop app (Nuxt frontend + Tauri/Rust backend with hot-reload)
   pnpm dev:desktop

   # Website / documentation
   pnpm dev:website
   ```

## Monorepo Structure

```
envly/
├── packages/
│   ├── envly-desktop/       # Tauri + Nuxt 4 desktop app
│   │   ├── app/             # Nuxt frontend (pages, components, composables)
│   │   ├── i18n/            # Locale files (English, Spanish, Italian)
│   │   ├── src-tauri/       # Rust backend source
│   │   └── package.json
│   └── envly-website/       # Documentation site (Nuxt + Nuxt Content)
│       ├── content/         # Markdown docs and YAML data
│       └── package.json
├── docs/                    # Internal developer docs (this folder)
├── .github/workflows/       # CI/CD workflows
├── pnpm-workspace.yaml
└── package.json             # Root workspace scripts
```

## Useful Commands

| Command              | Description                            |
| -------------------- | -------------------------------------- |
| `pnpm dev:desktop`   | Run the desktop app in dev mode        |
| `pnpm dev:website`   | Run the docs site in dev mode          |
| `pnpm build:desktop` | Build the desktop app for production   |
| `pnpm build:website` | Build the docs site for production     |
| `pnpm lint`          | Run linters across all packages        |
| `pnpm typecheck`     | Run type checking across all packages  |

## Branch Naming

Use descriptive branch names with a prefix:

| Prefix       | Use For               |
| ------------ | --------------------- |
| `feature/`   | New features          |
| `fix/`       | Bug fixes             |
| `docs/`      | Documentation changes |
| `refactor/`  | Code refactoring      |
| `chore/`     | Build, CI, or tooling |

## Commit Messages

Write clear commit messages that explain what changed and why:

```
feat: add secret expiration warnings

Show a warning badge on secrets that expire within 7 days
and an alert for expired secrets.
```

Common prefixes: `feat`, `fix`, `docs`, `refactor`, `chore`, `test`, `style`.

## Code Style

### Frontend (TypeScript / Vue)

- Use the Composition API with `<script setup>`
- Follow the existing ESLint configuration (`pnpm lint`)
- Use `@nuxt/ui` components where possible
- Keep components focused — one responsibility per component
- All Tauri IPC calls go through the `useTauri()` composable

### Backend (Rust)

- Follow standard Rust formatting (`cargo fmt`)
- Use `thiserror` for error types
- Keep the `core` library free of Tauri dependencies so it remains independently testable
- Use `Zeroizing<String>` for any data that holds secrets
- Don't implement `Debug` or `Display` on types that contain sensitive values

## Running Tests

```bash
cd packages/envly-desktop/src-tauri
cargo test
```

## Pull Request Guidelines

- Keep PRs focused on a single change
- Include a clear description of what the PR does
- If it's a visual change, include a screenshot
- Make sure existing tests pass
- Update documentation if your change affects user-facing behavior

## What to Contribute

Some areas where contributions are especially welcome:

- **Bug reports** — open an issue with steps to reproduce
- **Feature requests** — describe what you'd like and why
- **Documentation** — improve existing docs or add missing guides
- **Translations** — add or improve locale files in `packages/envly-desktop/i18n/`
- **Tests** — add unit tests for the Rust core library
- **UI polish** — improve responsiveness, accessibility, or visual consistency

## Reporting Issues

When opening an issue, include:

- Envly version and your operating system
- Steps to reproduce the problem
- Expected vs actual behavior
- Any error messages or logs

## License

By contributing to Envly, you agree that your contributions will be licensed under the [Apache 2.0 License](https://github.com/eiliv17/envly/blob/main/LICENSE).
