# Feature Specification: Releases for macOS, Windows and Debian

**Feature Branch**: `014-multiplatform-releases`

**Created**: 2026-10-15

**Status**: Implemented (first multiplatform release pending)

**Input**: User description (translated from Portuguese): "Generate a Debian package when a
release is made too. I will download the version and install it on my Windows machine."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - One release, three installers (Priority: P1)

When a version tag is pushed, the Releases page shows one release with the notes and a universal
`.dmg`, a Windows `-setup.exe` and an `amd64.deb`, with installation steps for each system.

**Acceptance Scenarios**:

1. **Given** a tag, **When** any CI gate or any build fails, **Then** the release stays a hidden
   draft (nothing half-finished is published).
2. **Given** every build succeeded, **When** the workflow ends, **Then** the release is published
   with all three installers.

## Requirements *(mandatory)*

- **FR-1401**: The workflow MUST draft the release once (notes from `cliff.toml` plus
  `.github/release-install.md`), build the three systems in parallel attaching their installers to
  that draft, and publish it only after all builds succeed.
- **FR-1402**: The `.deb` MUST be built on Ubuntu 22.04 so it runs on Debian 12 and newer.
