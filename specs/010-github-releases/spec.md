# Feature Specification: GitHub Releases Built in CI

**Feature Branch**: `010-github-releases`

**Created**: 2026-10-14

**Status**: Implemented (`v0.1.0` published)

**Input**: User description (translated from Portuguese): "Use GitHub Releases for the app: show
what changed from version to version and link the built app for download, built on GitHub instead
of keeping the app on my machine."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Download a version and see what changed (Priority: P1)

A visitor opens the repository's Releases page, sees each version with grouped notes (features,
fixes, documentation, maintenance) linking to the commits, and downloads a universal `.dmg` built
by GitHub Actions, with installation steps for an unsigned app.

### User Story 2 - Cut a release with one command (Priority: P1)

From an up-to-date `main`, the maintainer runs `npm run release -- x.y.z`: the version is updated
in every file, `CHANGELOG.md` is regenerated, and a `chore(release)` commit and a `vX.Y.Z` tag are
created. Pushing them starts the release workflow.

**Acceptance Scenarios**:

1. **Given** a version that is not newer, a dirty tree, a branch other than `main` or an existing
   tag, **When** releasing, **Then** the script stops without changing anything.
2. **Given** a pushed tag, **When** the workflow runs, **Then** every CI gate runs first, the tag
   must match the version in `package.json`, `tauri.conf.json` and `Cargo.toml`, and only then is
   the app built and the release published.
3. **Given** a tag with a pre-release suffix (`v0.2.0-beta.1`), **When** published, **Then** the
   release is marked as a pre-release.

## Requirements *(mandatory)*

- **FR-1001**: Release notes MUST be generated from Conventional Commits (`cliff.toml`); release
  commits are excluded.
- **FR-1002**: The build MUST happen on GitHub (`macos-latest`), universal (`aarch64` + `x86_64`).
- **FR-1003**: The release MUST NOT be published when any CI gate fails or versions disagree.
- **FR-1005**: Until the app is notarized, the whole bundle MUST carry a valid ad-hoc signature so
  macOS offers "Open Anyway" for downloaded copies.
- **FR-1004**: The release body MUST include installation steps for an unsigned app and how to
  uninstall it.
