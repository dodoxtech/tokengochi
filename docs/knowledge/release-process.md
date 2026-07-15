---
type: knowledge
status: active
created: 2026-07-12
updated: 2026-07-12
tags:
  - knowledge
  - packaging
  - ai-context
---

# Release Process

How to cut a Tokengochi release. See [[../tasks/active/0008-packaging-ci-updater|task 0008]] and [[../decisions/0004-unsigned-mvp-release|ADR-0004]] for background.

## Versioning

Semantic Versioning (`MAJOR.MINOR.PATCH`). Pre-1.0, treat `MINOR` bumps as potentially breaking.

Three files must carry the same version number:

- `src-tauri/tauri.conf.json` (`version`)
- `src-tauri/Cargo.toml` (`package.version`)
- `ui/dashboard/package.json` (`version`) — cosmetic, keep in sync for consistency

## Cutting a release

1. Update `CHANGELOG.md`: move `[Unreleased]` items under a new `## [X.Y.Z] - YYYY-MM-DD` heading.
2. Bump the version in the three files listed above to `X.Y.Z`.
3. Commit: `git commit -m "release: vX.Y.Z"`.
4. Tag and push: `git tag vX.Y.Z && git push origin main --tags`.
5. [.github/workflows/release.yml](../../.github/workflows/release.yml) triggers on the tag push, builds installers for macOS (arm64 + x86_64), Windows, and Linux, signs the updater artifacts, signs/notarizes macOS DMGs with Developer ID, and publishes a **draft** GitHub Release with `latest.json` attached.
6. Review the draft release (binaries, changelog body), then publish it manually. `tauri-plugin-updater` polls the `latest` release's `latest.json`, so unpublished drafts are invisible to existing installs — this is the safety gate before a release goes live.
7. Verify auto-update: install the previous tagged build, launch it, click "Check for updates" in the dashboard settings panel, confirm it downloads and the "Restart now" button relaunches into the new version with pet state intact.

## Updater signing key

`tauri-plugin-updater` requires artifacts to be signed with a **minisign** keypair, separate from OS code signing (see ADR-0004). This key was generated once with:

```sh
cargo tauri signer generate -w /path/to/tokengochi-updater.key
```

- The **public key** is committed in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.
- The **private key** and its password are stored as GitHub Actions repository secrets (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) — never commit the private key file.
- If the private key is ever lost or rotated, existing installs cannot verify new updates until they're manually reinstalled with a build carrying the new pubkey.

## macOS Developer ID signing and notarization

Per [[../decisions/0007-macos-developer-id-distribution|ADR-0007]], macOS release builds are signed with a Developer ID Application certificate and notarized by Apple so users can install the `.dmg` outside the Mac App Store without the unsigned Gatekeeper workaround.

Required GitHub Actions repository secrets:

- `APPLE_CERTIFICATE` — base64-encoded `.p12` export of the Developer ID Application certificate and private key.
- `APPLE_CERTIFICATE_PASSWORD` — password used when exporting the `.p12`.
- `APPLE_SIGNING_IDENTITY` — optional explicit identity, for example `Developer ID Application: Example LLC (TEAMID)`. If omitted, CI picks the first imported `Developer ID Application` identity.
- `APPLE_API_KEY` — App Store Connect API Key ID.
- `APPLE_API_ISSUER` — App Store Connect API Issuer ID.
- `APPLE_API_PRIVATE_KEY` — base64-encoded contents of the downloaded `AuthKey_<KEYID>.p8` private key.

Prepare the secrets on a trusted Mac:

```sh
# Export this from Keychain Access as a password-protected .p12 first.
openssl base64 -A -in /path/to/developer-id-application.p12 -out developer-id-application.p12.base64

# Download AuthKey_<KEYID>.p8 from App Store Connect > Users and Access > Integrations.
openssl base64 -A -in /path/to/AuthKey_<KEYID>.p8 -out AuthKey_<KEYID>.p8.base64
```

The release workflow imports the `.p12` into a temporary macOS runner keychain, writes the App Store Connect API key into `$RUNNER_TEMP`, builds signed macOS artifacts, and then runs `xcrun notarytool` directly. The workflow submits the final `.dmg` without an indefinite wait, prints the Apple submission id, polls for up to 45 minutes, prints each status transition, fetches the full Apple notarization log automatically when the status is `Invalid`, and fails with the submission id when polling times out.

Expected GitHub Actions notarization log shape:

```text
Notarization submission id: 2f388aec-8a5e-4502-9c3a-0da4ec97e4cb
Status: In Progress
Status: Accepted
```

If Apple returns `Invalid`, the job prints `Status: Invalid` followed by the full `notarytool log` JSON. If Apple leaves the submission in progress for the whole polling window, the job fails after 45 minutes and prints the submission id so it can be inspected manually with `xcrun notarytool info` or `xcrun notarytool log`.

Mac release checks:

```sh
codesign --verify --deep --strict --verbose=2 /Applications/Tokengochi.app
spctl --assess --type open --context context:primary-signature --verbose /Applications/Tokengochi.app
spctl --assess --type install --verbose /path/to/Tokengochi_x.y.z_aarch64.dmg
```

For the final Gatekeeper check, download the `.dmg` from the published GitHub Release in Safari or Chrome, drag Tokengochi into `/Applications`, and launch it from Finder. This preserves quarantine metadata and matches the real user path.

## Unsigned Windows builds

Per [[../decisions/0004-unsigned-mvp-release|ADR-0004]], Windows builds are still unsigned. Users will see a SmartScreen warning on first install until a Windows code-signing certificate is added.
