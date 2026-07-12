# Changelog

All notable changes to Tokengochi are documented here. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- GitHub Actions release workflow: pushing a `vX.Y.Z` tag builds signed-updater installers for macOS (arm64 + x86_64), Windows, and Linux, and publishes them as a draft GitHub Release.
- Auto-update via `tauri-plugin-updater` against GitHub Releases, with a "Check for updates" control in the dashboard settings panel.
- Unsigned-MVP packaging decision documented in [ADR-0004](docs/decisions/0004-unsigned-mvp-release.md).

## [0.1.0] - 2026-07-12

- MVP feature set: Claude Code token watcher, economy engine, pet overlay, tray, onboarding, settings, and stats dashboard (tasks 0001-0007).
