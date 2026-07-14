# Changelog

All notable changes to Tokengochi are documented here. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.1] - 2026-07-14

### Added

- Dashboard now auto-checks for updates on open and shows a small "new version available" badge in the header, separate from the manual "Check for updates" flow in Settings.

### Changed

- Update check/download/install are now separate steps: an automatic startup check only detects availability (silently, with no error banner on failure), and downloading only starts when the user clicks the badge or the Settings button.

### Fixed

- Windows release builds were silently failing: the overlay's `beforeBuildCommand` used `mkdir -p`/`cp`/shell globs, none of which cmd.exe supports. Replaced with a cross-platform Node script, so Windows installers are now actually published (this had been broken since v0.1.0).

## [0.2.0] - 2026-07-14

### Added

- GitHub Actions release workflow: pushing a `vX.Y.Z` tag builds signed-updater installers for macOS (arm64 + x86_64), Windows, and Linux, and publishes them as a draft GitHub Release.
- Auto-update via `tauri-plugin-updater` against GitHub Releases, with a "Check for updates" control in the dashboard settings panel.
- Unsigned-MVP packaging decision documented in [ADR-0004](docs/decisions/0004-unsigned-mvp-release.md).
- Multi-LLM provider plugins: Codex CLI and OpenAI Usage API support behind the `TokenProvider` trait (task 0011).
- Cosmetics shop, food skins, collection album, and prestige loop for spending Sparks (task 0010).
- Pet notifies AI agent status (task complete / needs approval) with cute expressions, including a Claude Code hook (`resources/claude-hooks/tokengochi-notify.sh`) and a toggle to opt individual notifications off per Sparks sink (tasks 0017, 0018).
- Expanded gag/expression pack: sneeze pose, yawn, dance, drink-break (task 0014).
- New app icon and refreshed cosmetic/food sprites, including a generated Mushroom Cap cosmetic (task 0013).

### Changed

- Window geometry and pet shelf/window-climbing behavior overhauled for more natural wandering and settling (ADR-0006, hides the collection album UI by default).
- Dashboard window stays hidden on normal startup instead of flashing on launch (task 0015).
- README copy refreshed to emphasize the pet chasing down and eating food, and to document the Food conversion rate; Ko-fi badge removed.
- Above-head bubble/badge clearance in the overlay renderer now uses separate fixed offsets for bare head vs. worn hat, fixing clipping through hat brims.

## [0.1.0] - 2026-07-12

- MVP feature set: Claude Code token watcher, economy engine, pet overlay, tray, onboarding, settings, and stats dashboard (tasks 0001-0007).
