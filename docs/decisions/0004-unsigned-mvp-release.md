---
type: decision
status: accepted
created: 2026-07-12
updated: 2026-07-12
owner: AI agent
tags:
  - decision
  - packaging
---

# Decision: Ship the MVP release unsigned on macOS and Windows

## Status

Accepted

## Date

2026-07-12

## Context

[[../tasks/active/0008-packaging-ci-updater|Task 0008]] needs `git tag vX.Y.Z` to produce installers for all three OS. macOS notarization requires a paid Apple Developer account ($99/yr) plus an App Store Connect API key; Windows code signing requires a paid EV/OV certificate ($200–500/yr, and EV needs hardware-token lead time). Neither is set up yet, and there is no budget/account decision from the project owner to block the first release on.

Per the task's own scope, an unsigned MVP is an explicitly allowed outcome ("macOS notarization and Windows code signing ... or documented unsigned-MVP decision").

## Decision

- Ship v0.x releases **unsigned** on macOS and Windows. Linux `.deb`/AppImage builds are unaffected (no OS-level signing gate).
- macOS builds use Tauri's default ad-hoc signature (not notarized). Gatekeeper will block first launch; README documents the `xattr -dr com.apple.quarantine` / right-click-Open workaround.
- Windows builds are unsigned. SmartScreen will show an "unknown publisher" warning; README documents "More info → Run anyway."
- The updater artifact (`latest.json` + `.sig`) is still cryptographically signed with a Tauri updater keypair (separate from OS code signing) so `tauri-plugin-updater` can verify update integrity even though the OS-level binary isn't notarized/signed.
- Revisit before a 1.0/public launch: budget for an Apple Developer account and a Windows code-signing certificate, then add notarization (`xcrun notarytool`) and `signtool` steps to the release workflow.

## Consequences

Positive:

- Unblocks the first tagged release immediately; no external account/cert lead time.
- Updater integrity is still guaranteed by the Tauri updater signature, independent of OS code signing.

Negative or tradeoffs:

- Gatekeeper/SmartScreen friction on first install — acceptable for an early-access MVP, documented in README.
- Users on managed/locked-down machines may not be able to bypass the warnings; this narrows the initial audience.

## Alternatives Considered

- **Self-signed certificates:** still trigger the same OS warnings as unsigned on first run (no trusted CA chain) while adding process overhead — not worth it before a real cert.
- **Delay release until certs are purchased:** blocks shipping the MVP on a budget/account decision outside engineering's control.

## References

- [[../tasks/active/0008-packaging-ci-updater|Task 0008]]
- [[../architecture|Architecture]] §Runtime and Deployment
