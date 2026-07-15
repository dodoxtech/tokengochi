---
type: task
status: active
priority: P1
delivery_order: 0019
estimate: 1d
created: 2026-07-15
updated: 2026-07-15
owner: AI agent
sprint: null
tags:
  - task
  - active
  - packaging
  - macos
---

# Task: Notarized macOS DMG distribution

## Status

- State: active
- Created: 2026-07-15
- Owner: AI agent
- Priority: P1
- Delivery order: 0019
- Estimate: 1d
- Sprint: TBD

## Context

Tokengochi already produces macOS `.dmg` release artifacts through [[../active/0008-packaging-ci-updater|0008 Packaging, signing, auto-update — MVP release]], but [[../../decisions/0004-unsigned-mvp-release|ADR-0004]] intentionally shipped the MVP unsigned. The project owner now has an Apple Developer account and wants users to install Tokengochi through a downloaded `.dmg` outside the Mac App Store.

For a public-feeling macOS download, the app should be signed with a Developer ID certificate, notarized by Apple, and stapled so Gatekeeper accepts it without the unsigned-app workaround currently documented in the root README.

Related:

- [[../README|Tasks]]
- [[../../architecture|Architecture]] §Runtime and Deployment
- [[../../knowledge/release-process|Release Process]]
- [[../../decisions/0004-unsigned-mvp-release|ADR-0004]]

## Goal

Users can download the macOS `.dmg`, drag Tokengochi into Applications, and launch it without the "developer cannot be verified" Gatekeeper block.

## Scope

In scope:

- Create or document the required Apple Developer assets:
  - Developer ID Application certificate.
  - App Store Connect API key or app-specific password usable by `xcrun notarytool`.
  - GitHub Actions secrets needed for macOS signing and notarization.
- Configure Tauri/macOS release builds to sign with Developer ID and hardened runtime.
- Add notarization and stapling to the macOS release path for both Apple Silicon and Intel `.dmg` artifacts.
- Update release docs and README install instructions once notarized builds are verified.
- Revisit [[../../decisions/0004-unsigned-mvp-release|ADR-0004]] with a superseding decision or status note for macOS.

Out of scope:

- Windows Authenticode signing or SmartScreen reputation.
- Mac App Store distribution.
- iOS/iPadOS distribution.
- Homebrew cask distribution.

## Acceptance Criteria

- [x] macOS release workflow imports the Developer ID signing certificate securely from GitHub Actions secrets.
- [x] macOS builds are signed with hardened runtime enabled and the correct Developer ID identity.
- [x] Both macOS `.dmg` artifacts are submitted to Apple notarization and the workflow fails loudly if notarization fails.
- [x] Notarization tickets are stapled to the distributable artifacts, or to the app before DMG creation if that is the Tauri-supported path.
- [ ] A freshly downloaded `.dmg` on a clean macOS machine can install and launch Tokengochi without the unsigned Developer ID Gatekeeper workaround.
- [x] README no longer tells macOS users that the current release is unsigned once the notarized release is live.
- [x] [[../../knowledge/release-process|Release Process]] documents the signing/notarization secrets and release checks.

## Dependencies

- [[../active/0008-packaging-ci-updater|0008]] should remain the base release pipeline.
- Apple Developer Program membership with Account Holder/Admin access to create Developer ID certificates.
- GitHub repository secrets access.

## Risks

- Apple certificate/private key handling is sensitive; never commit `.p12`, private keys, API keys, or passwords.
- Tauri's built-in macOS notarization support can hide the Apple submission id while waiting; keep notarization explicit in GitHub Actions so failures and timeouts are diagnosable.
- macOS notarization can fail on entitlements, hardened runtime, nested binaries, or unsigned helper files.
- Cross-arch builds may need separate validation because the release workflow builds both `aarch64-apple-darwin` and `x86_64-apple-darwin`.

## Implementation Notes

- Start by reading current Tauri v2 signing/bundling docs for macOS and checking `src-tauri/tauri.conf.json`.
- Keep macOS notarization as explicit `xcrun notarytool` and `xcrun stapler` steps so GitHub Actions prints submission ids, status polling, timeout details, and Apple error logs.
- Expected secret inputs may include a base64-encoded `.p12`, certificate password, Apple issuer/key IDs, API private key, team ID, and signing identity. Confirm names during implementation instead of guessing.
- Use `spctl --assess --type open --context context:primary-signature --verbose <artifact>` and `codesign --verify --deep --strict --verbose=2 Tokengochi.app` as local verification where practical.
- After the first notarized tag, test by downloading the GitHub Release asset rather than using the local build artifact, so quarantine/Gatekeeper behavior matches a real user install.

## References

- [[../../README|Documentation]]
- [[../../index|Project Map]]
- `README.md`
- `.github/workflows/release.yml`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- Apple: https://developer.apple.com/developer-id/
- Apple: https://developer.apple.com/help/account/certificates/create-developer-id-certificates/
- Apple: https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution

## Verification Plan

- [ ] Run `cargo tauri build --target aarch64-apple-darwin` on macOS, or verify through the macOS GitHub Actions job.
- [ ] Confirm `codesign --verify --deep --strict --verbose=2` passes for `Tokengochi.app`.
- [ ] Confirm `spctl` accepts the app or DMG on macOS.
- [ ] Download the release `.dmg` from GitHub Releases on a clean macOS account/machine and launch the app from `/Applications`.
- [ ] Record commands and results below.

## Verification Results

### 2026-07-15 — implementation

Implemented the macOS notarized DMG release path:

- Updated `.github/workflows/release.yml` so macOS release jobs import the Developer ID Application `.p12` from `APPLE_CERTIFICATE` / `APPLE_CERTIFICATE_PASSWORD`, create a temporary keychain, discover or use `APPLE_SIGNING_IDENTITY`, write the App Store Connect API key from `APPLE_API_PRIVATE_KEY`, and pass Tauri notarization env vars (`APPLE_API_KEY`, `APPLE_API_KEY_PATH`, `APPLE_API_ISSUER`) to `tauri-apps/tauri-action`.
- The workflow now fails loudly on macOS when the required signing/notarization secrets are missing.
- Added [[../../decisions/0007-macos-developer-id-distribution|ADR-0007]] to supersede the unsigned-MVP decision for macOS only.
- Updated [[../../knowledge/release-process|Release Process]] with the exact GitHub secrets, base64 commands, and macOS verification commands.
- Updated `README.md` so macOS is documented as the Developer ID notarized path once signing secrets are configured, while Windows remains unsigned.

Verification run locally:

- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release.yml"); puts "workflow yaml ok"'` passes.
- `python3 -m json.tool src-tauri/tauri.conf.json` passes.
- `rg -n "APPLE_|Developer ID|notar|ADR-0007" .github/workflows/release.yml docs README.md` confirms the docs/workflow references are present.
- `git diff --check` passes.

Not run:

- A real macOS release workflow, because the repository still needs the Apple signing/notarization secrets configured.
- A clean-machine Gatekeeper install test, because that requires a notarized GitHub Release asset produced by CI.

Required before moving to done:

- Add GitHub Actions secrets: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, optional `APPLE_SIGNING_IDENTITY`, `APPLE_API_KEY`, `APPLE_API_ISSUER`, and `APPLE_API_PRIVATE_KEY`.
- Push a test tag, confirm the two macOS jobs produce notarized DMGs, then download one `.dmg` from the GitHub Release and launch it from Finder on macOS.

### 2026-07-15 — v0.2.2 release prep

Owner confirmed the Apple signing/notarization env vars were added to the repository. Prepared release `0.2.2`:

- Bumped `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `ui/dashboard/package.json` to `0.2.2`.
- Synced the dashboard package lock root version to `0.2.2`.
- Added `CHANGELOG.md` entry for the notarized macOS DMG distribution path.

Verification:

- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release.yml"); puts "workflow yaml ok"'` passes.
- `python3 -m json.tool src-tauri/tauri.conf.json` passes.
- `npm --prefix ui/dashboard run check` passes.
- `npm --prefix ui/overlay run check` passes.
- `cargo test --manifest-path src-tauri/Cargo.toml` passes: 59 tests.
- `git diff --check` passes.

### 2026-07-15 — explicit notarization logging

The first notarized `v0.2.2` run showed that Tauri's built-in notarization wait can leave the macOS arm64 job stuck at `Notarizing ...tokengochi.app` without printing the Apple submission id. Apple history showed old submissions remaining `In Progress`, while one x64 submission was `Accepted`, so the workflow needed clearer diagnostics and a bounded wait.

Updated `.github/workflows/release.yml` again:

- Split macOS into a dedicated `release-macos` job and left Linux/Windows on `tauri-apps/tauri-action`.
- macOS now builds signed artifacts first, without passing Apple API notarization env vars into Tauri.
- The workflow submits the final `.dmg` with `xcrun notarytool submit --output-format json`, prints `Notarization submission id: ...`, and polls `xcrun notarytool info` for up to 45 minutes.
- If Apple returns `Invalid`, the workflow runs `xcrun notarytool log <submission-id>` and prints the full JSON log before failing.
- If Apple stays `In Progress` past the polling window, the workflow fails clearly with the submission id.
- If Apple returns `Accepted`, the workflow staples and validates the `.dmg`, then uploads macOS assets to the draft GitHub Release.
- Updated `ui/dashboard/package.json` so `npm --prefix ui/dashboard run build` runs `svelte-kit sync` before `vite build`, removing the `.svelte-kit/tsconfig.json` warning.

## Completion Notes

Fill this in before moving the task to `docs/tasks/done/`.

- Completed: YYYY-MM-DD
- Changed files:
- Follow-ups:
