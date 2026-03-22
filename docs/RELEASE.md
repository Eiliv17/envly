# Release Guide

This document describes how to publish a new version of the Envly desktop app.

## Overview

Releases are fully driven by **git tags**. There are no version numbers to update manually. The flow is:

1. Create and push a `v*` tag
2. CI automatically creates a GitHub release with a changelog
3. CI builds the desktop app for all platforms and uploads artifacts to the release

## How It Works

A single GitHub Actions workflow (`.github/workflows/release.yml`) handles the entire pipeline:

1. **`create-release` job** — uses [git-cliff](https://git-cliff.org/) to generate a changelog from conventional commits between the previous tag and the current one, then creates a GitHub release.
2. **`build` job** (runs after `create-release`) — builds the Tauri app in parallel on macOS (ARM64 + Intel), Windows, and Linux, then uploads artifacts with fixed filenames to the release.

The website download page uses GitHub's `/releases/latest/download/FILENAME` URL pattern, so download links always point to the latest release automatically — no website redeployment needed.

## Creating a Release

```bash
git tag v1.0.0
git push origin v1.0.0
```

That's it. The rest is automatic.

### What happens next

1. The `create-release` job runs: generates changelog from conventional commits, creates a GitHub release titled `v1.0.0`
2. The `build` job starts (4 parallel runners):

| Platform       | Artifact                       |
| -------------- | ------------------------------ |
| macOS ARM64    | `envly-macos-arm64.dmg`        |
| macOS Intel    | `envly-macos-x86_64.dmg`       |
| Windows x86_64 | `envly-windows-x86_64.exe`     |
| Linux x86_64   | `envly-linux-x86_64.AppImage`  |

3. Each artifact is uploaded to the release with `gh release upload --clobber`
4. The website download links (`/releases/latest/download/...`) automatically resolve to the new release

## Version Numbers in Config Files

The version fields in `tauri.conf.json`, `Cargo.toml`, and `package.json` are set to `0.0.0`. They are not used for release versioning — the git tag is the single source of truth.

## Conventional Commits

The changelog is generated from commit messages using [conventional commits](https://www.conventionalcommits.org/). Use these prefixes:

| Prefix      | Changelog Section |
| ----------- | ----------------- |
| `feat:`     | Features          |
| `fix:`      | Bug Fixes         |
| `docs:`     | Documentation     |
| `perf:`     | Performance       |
| `refactor:` | Refactoring       |
| `chore:`    | Miscellaneous     |
| `ci:`       | CI/CD             |
| `build:`    | Build             |

The configuration lives in `cliff.toml` at the repo root.

## Checklist

```
- [ ] Ensure all changes are committed and pushed to main
- [ ] Create tag: git tag vX.Y.Z
- [ ] Push tag: git push origin vX.Y.Z
- [ ] Wait for the release workflow to create the release and upload all artifacts
- [ ] Verify downloads work on the website
```

## Troubleshooting

### CI fails with "Resource not accessible by integration"

The workflow needs write permissions. Go to **Settings > Actions > General > Workflow permissions** and select **Read and write permissions**.

### A platform build fails but others succeed

The workflow uses `fail-fast: false`, so other platforms still complete. Check the failing job's logs in the Actions tab. Re-run just the failed job after fixing the issue.

### Artifacts are missing from the release

If a matrix job failed, its artifact won't be uploaded. Fix the build and re-run the job. The `--clobber` flag means re-running a job safely overwrites any existing artifact with the same name.

### git-cliff produces an empty changelog

Make sure you're using conventional commit prefixes (`feat:`, `fix:`, etc.). Non-conventional commits are filtered out by default. Check `cliff.toml` for the configured parsers.
