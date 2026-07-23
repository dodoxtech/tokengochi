# Changelog

All notable changes to Tokengochi are documented here. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed

- The release workflow's `finalize-updater-manifest` job now uploads the merged manifest as a file literally named `latest.json` instead of using `gh release upload merged-latest.json#latest.json`. The `file#name` syntax only sets a cosmetic display label, not the asset filename, so it created a stray `merged-latest.json` asset and left the real `latest.json` as the windows/linux-only base — the actual reason macOS "Check for updates" kept failing. The v0.2.10 release asset was corrected in place. The finalize job now also re-downloads the published `latest.json` to confirm the macOS keys are present.

## [0.2.10] - 2026-07-23

### Fixed

- The macOS updater manifest is now assembled by a single dedicated `finalize-updater-manifest` job that runs after all platform builds, instead of each build job racing to read-modify-write the shared `latest.json`. That approach never converged: `gh release upload --clobber` recreates the asset and an immediate `gh release download` is served the stale pre-clobber copy, so every job's verify-and-retry loop kept failing to see its own `darwin-*` entry (v0.2.8 and v0.2.9 both shipped as drafts with a missing macOS updater manifest). The finalize job is the only writer and uploads `latest.json` exactly once.

## [0.2.9] - 2026-07-23

### Fixed

- The two macOS release jobs (`apple-silicon`, `intel`) no longer run in parallel: they raced to read-modify-write the same published `latest.json` with identical retry backoff timing, clobbering each other in lockstep every attempt until retries were exhausted. They now run one at a time (`max-parallel: 1`), and the retry backoff has jitter as defense-in-depth. v0.2.8 shipped as a draft with a missing macOS updater manifest as a result.

## [0.2.8] - 2026-07-23

### Fixed

- macOS release jobs now hand-build a `latest.json` from their signed updater artifacts before the merge step, fixing the merge always failing with "No macOS latest.json was found" (`cargo tauri build` never wrote one, unlike `tauri-action` on Windows/Linux) — v0.2.7 shipped without macOS entries in the published updater manifest as a result.

## [0.2.7] - 2026-07-23

### Added

- Dashboard now shows today/week token totals broken out per provider (Claude Code, Codex) alongside the combined total.
- Tray menu gained a "Delete all data & quit…" action that wipes the game database and watcher bookkeeping after confirmation.

### Changed

- Only tokens spent while the app is running are converted into food/XP; usage logged while Tokengochi was closed (including the very first run) is credited to the ledger silently, without spawning falling food or backfill animation.
- Codex token events now read the real model from the session's `turn_context` record instead of always assuming `gpt-5`.
- Claude Code and Codex watchers now honor `CLAUDE_CONFIG_DIR` / `CODEX_HOME` overrides instead of hardcoding `~/.claude` and `~/.codex`.
- Uninstalling (macOS drag-to-Trash flow documented; Windows NSIS uninstaller; Linux `.deb` purge) now removes the app's SQLite database and watcher state, while in-place version updates preserve existing data.
- The Claude Code notification hook script is now bundled as a Tauri resource and resolved at runtime, fixing "Install hook" failing with a path-not-found error on any downloaded build.
- macOS release jobs now merge their `darwin-*` platform entries into the published `latest.json` instead of clobbering it, fixing "Check for updates" always failing on macOS.

## [0.2.6] - 2026-07-16

### Changed

- macOS release notarization now retries with a fresh submission (up to 3 attempts of 15 minutes each) instead of a single 45-minute wait, working around Apple notary submissions that occasionally get stuck in "In Progress" indefinitely.

## [0.2.5] - 2026-07-16

### Added

- Dashboard settings panel now shows the currently installed app version next to the update status, so update checks can be verified against a visible baseline.

### Fixed

- Overlay pet no longer keeps walking past the screen edge when heading to bed: the bed-seek path was missing the same bounds clamp every other movement path already had, and the underlying furniture-position math used a hard-coded pet size instead of the configured one.
- Overlay pet no longer visually drifts off the floor line after a monitor/display change: `pet.y` is now resynced to the ground line every tick while grounded, matching the same per-tick resync the window-ledge "sit" state already had.

## [0.2.4] - 2026-07-16

### Fixed

- macOS release jobs now run `cargo tauri build` from `src-tauri`, preserving the expected project-root-relative frontend build commands while keeping explicit notarization logging.

## [0.2.3] - 2026-07-16

### Changed

- macOS release notarization now runs through explicit `notarytool` submit/poll/log steps so GitHub Actions prints the Apple submission id, bounded status polling, timeout errors, and full Apple failure logs.
- Development/debug builds now use separate data namespaces from production builds, keeping local development data out of downloaded release apps.
- Onboarding now skips the starter egg picker while Tokengochi has only one pet, keeping the picker code behind a future multi-pet flag.

### Fixed

- CI smoke builds no longer require the updater private key by disabling updater artifact creation through a CI-only Tauri config overlay.
- Rust formatting and clippy warnings are fixed across the macOS signing/data-path changes.

## [0.2.2] - 2026-07-15

### Added

- macOS Developer ID signing and Apple notarization path for direct `.dmg` distribution outside the Mac App Store.
- Release documentation for Apple signing/notarization GitHub Actions secrets and clean-machine Gatekeeper verification.
- ADR-0007 documenting macOS Developer ID distribution while leaving Windows code signing as a separate follow-up.

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
